use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, post, web};
use tera::Context;

use crate::{
    AppState,
    audit::record_audit_event,
    chart_account::ChartAccount,
    customer_payment::{CustomerPaymentInput, UnpaidInvoice, UnpaidInvoiceView},
    helpers::{
        add_user_to_ctx, can_create_accounting_records, format_cents, parse_amount_to_cents,
    },
};

// Loads invoices with remaining balances and active Asset accounts for the payment form.
async fn add_payment_form_data(state: &web::Data<AppState>, ctx: &mut Context) {
    let invoices: Vec<UnpaidInvoice> = sqlx::query_as(
        "SELECT
            i.id,
            i.invoice_no,
            c.name AS customer_name,
            i.amount_cents,
            COALESCE(SUM(cp.amount_cents), 0) AS paid_cents,
            i.amount_cents - COALESCE(SUM(cp.amount_cents), 0) AS remaining_cents
         FROM invoices i
         JOIN customers c ON c.id = i.customer_id
         LEFT JOIN customer_payments cp ON cp.invoice_id = i.id
         WHERE i.status IN ('unpaid', 'partial')
         GROUP BY i.id, i.invoice_no, c.name, i.amount_cents, i.invoice_date
         HAVING remaining_cents > 0
         ORDER BY i.invoice_date ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let invoice_views: Vec<UnpaidInvoiceView> = invoices
        .into_iter()
        .map(|invoice| UnpaidInvoiceView {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            customer_name: invoice.customer_name,
            amount_display: format_cents(invoice.amount_cents),
            paid_display: format_cents(invoice.paid_cents),
            remaining_display: format_cents(invoice.remaining_cents),
        })
        .collect();

    let cash_accounts: Vec<ChartAccount> = sqlx::query_as(
        "SELECT id, code, name, account_type, normal_balance, is_active
         FROM chart_accounts
         WHERE account_type = 'Asset' AND is_active = 1
         ORDER BY code ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    ctx.insert("invoices", &invoice_views);
    ctx.insert("cash_accounts", &cash_accounts);
}

async fn render_payment_error(
    state: &web::Data<AppState>,
    ctx: &mut Context,
    error: &str,
) -> HttpResponse {
    add_payment_form_data(state, ctx).await;
    ctx.insert("error", error);
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.tera.render("payments_new.html", ctx).unwrap())
}

#[get("/payments/new")]
// Displays unpaid invoices that can be settled.
pub async fn new_payment_form(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !can_create_accounting_records(&session) {
        return HttpResponse::Found()
            .append_header(("Location", "/invoices"))
            .finish();
    }

    add_payment_form_data(&state, &mut ctx).await;
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.tera.render("payments_new.html", &ctx).unwrap())
}

#[post("/payments")]
// Records a full or partial payment with Debit Cash and Credit Accounts Receivable.
pub async fn create_payment(
    state: web::Data<AppState>,
    session: Session,
    form: web::Form<CustomerPaymentInput>,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !can_create_accounting_records(&session) {
        return HttpResponse::Found()
            .append_header(("Location", "/invoices"))
            .finish();
    }

    let invoice: Option<(String, i64, i64)> = sqlx::query_as(
        "SELECT
            i.invoice_no,
            i.amount_cents,
            COALESCE(SUM(cp.amount_cents), 0) AS paid_cents
         FROM invoices i
         LEFT JOIN customer_payments cp ON cp.invoice_id = i.id
         WHERE i.id = ?1
           AND i.status IN ('unpaid', 'partial')
         GROUP BY i.id, i.invoice_no, i.amount_cents",
    )
    .bind(form.invoice_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let Some((invoice_no, amount_cents, paid_cents)) = invoice else {
        return render_payment_error(
            &state,
            &mut ctx,
            "Choose an invoice with an unpaid balance.",
        )
        .await;
    };

    let remaining_cents = amount_cents - paid_cents;
    if remaining_cents <= 0 {
        return render_payment_error(&state, &mut ctx, "This invoice is already fully paid.").await;
    }

    let payment_cents = match parse_amount_to_cents(&form.amount) {
        Ok(cents) if cents > 0 => cents,
        Ok(_) => {
            return render_payment_error(&state, &mut ctx, "Amount must be greater than zero.")
                .await;
        }
        Err(error) => return render_payment_error(&state, &mut ctx, &error).await,
    };

    if payment_cents > remaining_cents {
        return render_payment_error(
            &state,
            &mut ctx,
            "Payment cannot be greater than the remaining invoice balance.",
        )
        .await;
    }

    let cash_account_exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM chart_accounts
         WHERE id = ?1 AND account_type = 'Asset' AND is_active = 1",
    )
    .bind(form.cash_account_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    if cash_account_exists.is_none() {
        return render_payment_error(&state, &mut ctx, "Choose an active Cash account.").await;
    }

    let receivable_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM chart_accounts
         WHERE code = '1100' AND account_type = 'Asset' AND is_active = 1",
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let Some(receivable_id) = receivable_id else {
        return render_payment_error(
            &state,
            &mut ctx,
            "Active Accounts Receivable account 1100 is required.",
        )
        .await;
    };

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let entry_no = format!("JE-{:04}", count + 1);
    let memo = format!("Payment received for {}", invoice_no);
    let new_paid_cents = paid_cents + payment_cents;
    let new_status = if new_paid_cents >= amount_cents {
        "paid"
    } else {
        "partial"
    };

    // Saves the journal entry, payment record, and invoice status together.
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return render_payment_error(&state, &mut ctx, "Could not start payment transaction.")
                .await;
        }
    };

    let journal_id = match sqlx::query(
        "INSERT INTO journal_entries (entry_no, entry_date, memo, created_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
    )
    .bind(&entry_no)
    .bind(&form.payment_date)
    .bind(&memo)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(_) => {
            return render_payment_error(
                &state,
                &mut ctx,
                "Could not create payment journal entry.",
            )
            .await;
        }
    };

    for (account_id, debit, credit) in [
        (form.cash_account_id, payment_cents, 0_i64),
        (receivable_id, 0_i64, payment_cents),
    ] {
        if sqlx::query(
            "INSERT INTO journal_lines (journal_entry_id, chart_account_id, debit_cents, credit_cents)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(journal_id).bind(account_id).bind(debit).bind(credit)
        .execute(&mut *tx).await.is_err() {
            return render_payment_error(&state, &mut ctx, "Could not create payment journal lines.").await;
        }
    }

    let payment_id = match sqlx::query(
        "INSERT INTO customer_payments
        (invoice_id, payment_date, amount_cents, cash_account_id, journal_entry_id, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
    )
    .bind(form.invoice_id)
    .bind(&form.payment_date)
    .bind(payment_cents)
    .bind(form.cash_account_id)
    .bind(journal_id)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(_) => {
            return render_payment_error(&state, &mut ctx, "Could not save payment.").await;
        }
    };

    let updated = sqlx::query("UPDATE invoices SET status = ?1 WHERE id = ?2")
        .bind(new_status)
        .bind(form.invoice_id)
        .execute(&mut *tx)
        .await;

    if !matches!(updated, Ok(result) if result.rows_affected() == 1) {
        return render_payment_error(&state, &mut ctx, "Could not update invoice status.").await;
    }

    let details = format!(
        "Received {} for invoice {}; new status is {}.",
        format_cents(payment_cents),
        invoice_no,
        new_status
    );

    if record_audit_event(
        &mut tx,
        &session,
        "created",
        "customer_payment",
        payment_id,
        &details,
    )
    .await
    .is_err()
    {
        return render_payment_error(&state, &mut ctx, "Could not save audit trail record.").await;
    }

    if tx.commit().await.is_err() {
        return render_payment_error(&state, &mut ctx, "Could not complete payment.").await;
    }

    HttpResponse::Found()
        .append_header(("Location", "/invoices"))
        .finish()
}
