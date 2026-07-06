use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, post, web};
use tera::Context;

use crate::{
    AppState,
    audit::record_audit_event,
    customer::Customer,
    filters::DateRangeFilter,
    helpers::{
        add_user_to_ctx, can_create_accounting_records, format_cents, parse_amount_to_cents,
    },
    invoice::{Invoice, InvoiceInput, InvoiceView},
};

// Gets customers for the invoice form dropdown.
async fn get_customers(state: &web::Data<AppState>) -> Vec<Customer> {
    sqlx::query_as(
        "SELECT id, name, email, phone, created_at
         FROM customers
         ORDER BY name ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
}

// Gets one active standard account by its code and accounting type.
async fn get_active_account_id(
    state: &web::Data<AppState>,
    code: &str,
    account_type: &str,
) -> Option<i64> {
    sqlx::query_scalar(
        "SELECT id
         FROM chart_accounts
         WHERE code = ?1
           AND account_type = ?2
           AND is_active = 1",
    )
    .bind(code)
    .bind(account_type)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None)
}

// Reloads the invoice form with its customers and an error message.
async fn render_invoice_form_error(
    state: &web::Data<AppState>,
    ctx: &mut Context,
    error: &str,
) -> HttpResponse {
    let customers = get_customers(state).await;
    ctx.insert("customers", &customers);
    ctx.insert("error", error);

    let rendered = state.tera.render("invoices_new.html", ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/invoices")]
// Shows saved invoices and their payment status.
pub async fn list_invoices(
    state: web::Data<AppState>,
    session: Session,
    query: web::Query<DateRangeFilter>,
) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    let start_date = query.start_date.as_deref().unwrap_or("");
    let end_date = query.end_date.as_deref().unwrap_or("");
    let invoices: Vec<Invoice> = sqlx::query_as(
        "SELECT
            i.id,
            i.invoice_no,
            c.name AS customer_name,
            i.invoice_date,
            i.due_date,
            i.description,
            i.amount_cents,
            COALESCE((
                SELECT SUM(cp.amount_cents)
                FROM customer_payments cp
                WHERE cp.invoice_id = i.id
            ), 0) AS paid_cents,
            i.status,
            i.journal_entry_id,
            i.created_at
         FROM invoices i
         JOIN customers c ON c.id = i.customer_id
         WHERE (?1 = '' OR i.invoice_date >= ?1)
           AND (?2 = '' OR i.invoice_date <= ?2)
         ORDER BY i.invoice_date DESC, i.id DESC",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let invoice_views: Vec<InvoiceView> = invoices
        .into_iter()
        .map(|invoice| {
            let balance_cents = invoice.amount_cents - invoice.paid_cents;

            InvoiceView {
                id: invoice.id,
                invoice_no: invoice.invoice_no,
                customer_name: invoice.customer_name,
                invoice_date: invoice.invoice_date,
                due_date: invoice.due_date,
                description: invoice.description,
                amount_display: format_cents(invoice.amount_cents),
                paid_display: format_cents(invoice.paid_cents),
                balance_display: format_cents(balance_cents),
                status: invoice.status,
                journal_entry_id: invoice.journal_entry_id,
            }
        })
        .collect();

    ctx.insert("invoices", &invoice_views);
    ctx.insert("start_date", start_date);
    ctx.insert("end_date", end_date);

    let rendered = state.tera.render("invoices.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/invoices/new")]
// Displays the form for creating one customer invoice.
pub async fn new_invoice_form(state: web::Data<AppState>, session: Session) -> impl Responder {
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

    let customers = get_customers(&state).await;
    ctx.insert("customers", &customers);

    let rendered = state.tera.render("invoices_new.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[post("/invoices")]
// Saves an invoice and its Debit Receivable / Credit Revenue journal entry.
pub async fn create_invoice(
    state: web::Data<AppState>,
    session: Session,
    form: web::Form<InvoiceInput>,
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

    if form.description.trim().is_empty() {
        return render_invoice_form_error(&state, &mut ctx, "Description is required.").await;
    }

    if form.due_date < form.invoice_date {
        return render_invoice_form_error(
            &state,
            &mut ctx,
            "Due date cannot be before invoice date.",
        )
        .await;
    }

    let amount_cents = match parse_amount_to_cents(&form.amount) {
        Ok(cents) if cents > 0 => cents,
        Ok(_) => {
            return render_invoice_form_error(
                &state,
                &mut ctx,
                "Amount must be greater than zero.",
            )
            .await;
        }
        Err(error) => {
            return render_invoice_form_error(&state, &mut ctx, &error).await;
        }
    };

    let customer_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM customers WHERE id = ?1")
        .bind(form.customer_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    if customer_exists.is_none() {
        return render_invoice_form_error(&state, &mut ctx, "Choose a valid customer.").await;
    }

    let receivable_account_id = get_active_account_id(&state, "1100", "Asset").await;
    let revenue_account_id = get_active_account_id(&state, "4000", "Revenue").await;

    let (Some(receivable_account_id), Some(revenue_account_id)) =
        (receivable_account_id, revenue_account_id)
    else {
        return render_invoice_form_error(
            &state,
            &mut ctx,
            "Active Accounts Receivable and Sales Revenue accounts are required.",
        )
        .await;
    };

    let invoice_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invoices")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let journal_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let invoice_no = format!("INV-{:04}", invoice_count + 1);
    let entry_no = format!("JE-{:04}", journal_count + 1);
    let memo = format!("Invoice {}: {}", invoice_no, form.description.trim());

    // Keep the invoice record and all journal records together, or save none on error.
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return render_invoice_form_error(
                &state,
                &mut ctx,
                "Could not start invoice transaction.",
            )
            .await;
        }
    };

    let journal_entry_id = match sqlx::query(
        "INSERT INTO journal_entries (entry_no, entry_date, memo, created_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
    )
    .bind(&entry_no)
    .bind(&form.invoice_date)
    .bind(&memo)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(_) => {
            return render_invoice_form_error(
                &state,
                &mut ctx,
                "Could not create invoice journal entry.",
            )
            .await;
        }
    };

    if sqlx::query(
        "INSERT INTO journal_lines (
            journal_entry_id,
            chart_account_id,
            debit_cents,
            credit_cents
         )
         VALUES (?1, ?2, ?3, 0)",
    )
    .bind(journal_entry_id)
    .bind(receivable_account_id)
    .bind(amount_cents)
    .execute(&mut *tx)
    .await
    .is_err()
    {
        return render_invoice_form_error(
            &state,
            &mut ctx,
            "Could not create Accounts Receivable debit line.",
        )
        .await;
    }

    if sqlx::query(
        "INSERT INTO journal_lines (
            journal_entry_id,
            chart_account_id,
            debit_cents,
            credit_cents
         )
         VALUES (?1, ?2, 0, ?3)",
    )
    .bind(journal_entry_id)
    .bind(revenue_account_id)
    .bind(amount_cents)
    .execute(&mut *tx)
    .await
    .is_err()
    {
        return render_invoice_form_error(
            &state,
            &mut ctx,
            "Could not create Sales Revenue credit line.",
        )
        .await;
    }

    let invoice_id = match sqlx::query(
        "INSERT INTO invoices (
            invoice_no,
            customer_id,
            invoice_date,
            due_date,
            description,
            amount_cents,
            status,
            journal_entry_id,
            created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'unpaid', ?7, datetime('now'))",
    )
    .bind(&invoice_no)
    .bind(form.customer_id)
    .bind(&form.invoice_date)
    .bind(&form.due_date)
    .bind(form.description.trim())
    .bind(amount_cents)
    .bind(journal_entry_id)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(_) => {
            return render_invoice_form_error(&state, &mut ctx, "Could not save invoice record.")
                .await;
        }
    };

    let details = format!(
        "Created invoice {}: {}.",
        invoice_no,
        form.description.trim()
    );

    if record_audit_event(
        &mut tx, &session, "created", "invoice", invoice_id, &details,
    )
    .await
    .is_err()
    {
        return render_invoice_form_error(&state, &mut ctx, "Could not save audit trail record.")
            .await;
    }

    if tx.commit().await.is_err() {
        return render_invoice_form_error(&state, &mut ctx, "Could not save invoice transaction.")
            .await;
    }

    HttpResponse::Found()
        .append_header(("Location", "/invoices"))
        .finish()
}
