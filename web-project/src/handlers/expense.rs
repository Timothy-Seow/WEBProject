use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, post, web};
use tera::Context;

use crate::{
    AppState,
    audit::record_audit_event,
    chart_account::ChartAccount,
    expense::{Expense, ExpenseInput, ExpenseView},
    filters::DateRangeFilter,
    helpers::{
        add_user_to_ctx, can_create_accounting_records, format_cents, parse_amount_to_cents,
    },
};

// Gets active accounts for the expense form dropdowns.
async fn get_active_chart_accounts(state: &web::Data<AppState>) -> Vec<ChartAccount> {
    sqlx::query_as(
        "SELECT id, code, name, account_type, normal_balance, is_active
         FROM chart_accounts
         WHERE is_active = 1
         ORDER BY code ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
}

// Gets the type of one active account so submitted IDs can be validated.
async fn get_active_account_type(state: &web::Data<AppState>, account_id: i64) -> Option<String> {
    sqlx::query_scalar(
        "SELECT account_type
         FROM chart_accounts
         WHERE id = ?1 AND is_active = 1",
    )
    .bind(account_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None)
}

// Reloads the expense form with its account dropdowns and an error message.
async fn render_expense_form_error(
    state: &web::Data<AppState>,
    ctx: &mut Context,
    error: &str,
) -> HttpResponse {
    let accounts = get_active_chart_accounts(state).await;
    ctx.insert("accounts", &accounts);
    ctx.insert("error", error);

    let rendered = state.tera.render("expenses_new.html", ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/expenses")]
// Shows saved expense records and the journal entry linked to each one.
pub async fn list_expenses(
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
    let expenses: Vec<Expense> = sqlx::query_as(
        "SELECT
            e.id,
            e.expense_date,
            e.description,
            expense_account.code AS expense_account_code,
            expense_account.name AS expense_account_name,
            payment_account.code AS payment_account_code,
            payment_account.name AS payment_account_name,
            e.amount_cents,
            e.journal_entry_id,
            e.created_at
         FROM expenses e
         JOIN chart_accounts expense_account
            ON expense_account.id = e.expense_account_id
         JOIN chart_accounts payment_account
            ON payment_account.id = e.payment_account_id
         WHERE (?1 = '' OR e.expense_date >= ?1)
           AND (?2 = '' OR e.expense_date <= ?2)
         ORDER BY e.expense_date DESC, e.id DESC",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let expense_views: Vec<ExpenseView> = expenses
        .into_iter()
        .map(|expense| ExpenseView {
            id: expense.id,
            expense_date: expense.expense_date,
            description: expense.description,
            expense_account: format!(
                "{} {}",
                expense.expense_account_code, expense.expense_account_name
            ),
            payment_account: format!(
                "{} {}",
                expense.payment_account_code, expense.payment_account_name
            ),
            amount_display: format_cents(expense.amount_cents),
            journal_entry_id: expense.journal_entry_id,
        })
        .collect();

    ctx.insert("expenses", &expense_views);
    ctx.insert("start_date", start_date);
    ctx.insert("end_date", end_date);

    let rendered = state.tera.render("expenses.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/expenses/new")]
// Displays the form for recording one business expense.
pub async fn new_expense_form(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !can_create_accounting_records(&session) {
        return HttpResponse::Found()
            .append_header(("Location", "/expenses"))
            .finish();
    }

    let accounts = get_active_chart_accounts(&state).await;
    ctx.insert("accounts", &accounts);

    let rendered = state.tera.render("expenses_new.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[post("/expenses")]
// Saves an expense and its balanced debit and credit journal entry together.
pub async fn create_expense(
    state: web::Data<AppState>,
    session: Session,
    form: web::Form<ExpenseInput>,
) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !can_create_accounting_records(&session) {
        return HttpResponse::Found()
            .append_header(("Location", "/expenses"))
            .finish();
    }

    if form.description.trim().is_empty() {
        return render_expense_form_error(&state, &mut ctx, "Description is required.").await;
    }

    let amount_cents = match parse_amount_to_cents(&form.amount) {
        Ok(cents) => cents,
        Err(error) => {
            return render_expense_form_error(&state, &mut ctx, &error).await;
        }
    };

    if amount_cents <= 0 {
        return render_expense_form_error(&state, &mut ctx, "Amount must be greater than zero.")
            .await;
    }

    if form.expense_account_id == form.payment_account_id {
        return render_expense_form_error(
            &state,
            &mut ctx,
            "Expense and payment accounts must be different.",
        )
        .await;
    }

    let expense_account_type = get_active_account_type(&state, form.expense_account_id).await;

    if expense_account_type.as_deref() != Some("Expense") {
        return render_expense_form_error(&state, &mut ctx, "Choose an active Expense account.")
            .await;
    }

    let payment_account_type = get_active_account_type(&state, form.payment_account_id).await;

    if !matches!(
        payment_account_type.as_deref(),
        Some("Asset") | Some("Liability")
    ) {
        return render_expense_form_error(
            &state,
            &mut ctx,
            "Choose an active Asset or Liability payment account.",
        )
        .await;
    }

    let entry_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let entry_no = format!("JE-{:04}", entry_count + 1);
    let memo = format!("Expense: {}", form.description.trim());

    // Keep the expense record and all journal records together, or save none on error.
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return render_expense_form_error(
                &state,
                &mut ctx,
                "Could not start expense transaction.",
            )
            .await;
        }
    };

    let journal_entry_id = match sqlx::query(
        "INSERT INTO journal_entries (entry_no, entry_date, memo, created_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
    )
    .bind(&entry_no)
    .bind(&form.expense_date)
    .bind(&memo)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(_) => {
            return render_expense_form_error(
                &state,
                &mut ctx,
                "Could not create expense journal entry.",
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
    .bind(form.expense_account_id)
    .bind(amount_cents)
    .execute(&mut *tx)
    .await
    .is_err()
    {
        return render_expense_form_error(&state, &mut ctx, "Could not create expense debit line.")
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
    .bind(form.payment_account_id)
    .bind(amount_cents)
    .execute(&mut *tx)
    .await
    .is_err()
    {
        return render_expense_form_error(
            &state,
            &mut ctx,
            "Could not create expense credit line.",
        )
        .await;
    }

    let expense_id = match sqlx::query(
        "INSERT INTO expenses (
            expense_date,
            description,
            expense_account_id,
            payment_account_id,
            amount_cents,
            journal_entry_id,
            created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
    )
    .bind(&form.expense_date)
    .bind(form.description.trim())
    .bind(form.expense_account_id)
    .bind(form.payment_account_id)
    .bind(amount_cents)
    .bind(journal_entry_id)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(_) => {
            return render_expense_form_error(&state, &mut ctx, "Could not save expense record.")
                .await;
        }
    };

    let details = format!("Created expense: {}.", form.description.trim());

    if record_audit_event(
        &mut tx, &session, "created", "expense", expense_id, &details,
    )
    .await
    .is_err()
    {
        return render_expense_form_error(&state, &mut ctx, "Could not save audit trail record.")
            .await;
    }

    if tx.commit().await.is_err() {
        return render_expense_form_error(&state, &mut ctx, "Could not save expense transaction.")
            .await;
    }

    HttpResponse::Found()
        .append_header(("Location", "/expenses"))
        .finish()
}
