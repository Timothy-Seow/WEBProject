use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, post, web};
use std::collections::HashMap;
use tera::Context;

use crate::{
    AppState,
    audit::record_audit_event,
    chart_account::ChartAccount,
    filters::DateRangeFilter,
    helpers::{
        add_user_to_ctx, can_create_accounting_records, format_cents, format_cents_or_blank,
        parse_amount_to_cents,
    },
    journal::{JournalEntry, JournalEntryInput, JournalEntryView, JournalLine, JournalLineView},
};

// Gets accounts that are still available for new journal entries.
async fn get_active_chart_accounts(state: &web::Data<AppState>) -> Vec<ChartAccount> {
    sqlx::query_as::<_, ChartAccount>(
        "SELECT id, code, name, account_type, normal_balance, is_active
         FROM chart_accounts
         WHERE is_active = 1
         ORDER BY code ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
}

// Reloads the journal form with its account lists and an error message.
async fn render_journal_form_error(
    state: &web::Data<AppState>,
    ctx: &mut Context,
    error: &str,
) -> HttpResponse {
    let accounts = get_active_chart_accounts(state).await;
    ctx.insert("accounts", &accounts);
    ctx.insert("error", error);
    let rendered = state.tera.render("journal_new.html", ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/journal")]
// Shows journal entries, their debit/credit lines, and the overall balance check.
pub async fn list_journal_entries(
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

    let entries: Vec<JournalEntry> = sqlx::query_as::<_, JournalEntry>(
        "SELECT id, entry_no, entry_date, memo, created_at
         FROM journal_entries
         WHERE (?1 = '' OR entry_date >= ?1)
           AND (?2 = '' OR entry_date <= ?2)
         ORDER BY entry_date DESC, id DESC",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let lines: Vec<JournalLine> = sqlx::query_as::<_, JournalLine>(
        "SELECT jl.id,
                jl.journal_entry_id,
                ca.code AS account_code,
                ca.name AS account_name,
                jl.debit_cents,
                jl.credit_cents
         FROM journal_lines jl
         JOIN chart_accounts ca ON ca.id = jl.chart_account_id
         JOIN journal_entries je ON je.id = jl.journal_entry_id
         WHERE (?1 = '' OR je.entry_date >= ?1)
           AND (?2 = '' OR je.entry_date <= ?2)
         ORDER BY jl.journal_entry_id DESC, jl.id ASC",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let total_debit_cents: i64 = lines.iter().map(|line| line.debit_cents).sum();
    let total_credit_cents: i64 = lines.iter().map(|line| line.credit_cents).sum();
    let is_balanced = total_debit_cents == total_credit_cents;

    let mut entry_totals: HashMap<i64, (i64, i64)> = HashMap::new();

    for line in &lines {
        let totals = entry_totals.entry(line.journal_entry_id).or_insert((0, 0));

        totals.0 += line.debit_cents;
        totals.1 += line.credit_cents;
    }

    let entry_views: Vec<JournalEntryView> = entries
        .into_iter()
        .map(|entry| {
            let (debit_cents, credit_cents) =
                entry_totals.get(&entry.id).copied().unwrap_or((0, 0));

            JournalEntryView {
                id: entry.id,
                entry_no: entry.entry_no,
                entry_date: entry.entry_date,
                memo: entry.memo,
                debit_total: format_cents(debit_cents),
                credit_total: format_cents(credit_cents),
                is_balanced: debit_cents == credit_cents,
            }
        })
        .collect();

    let line_views: Vec<JournalLineView> = lines
        .into_iter()
        .map(|line| JournalLineView {
            id: line.id,
            journal_entry_id: line.journal_entry_id,
            account_code: line.account_code,
            account_name: line.account_name,
            debit_display: format_cents_or_blank(line.debit_cents),
            credit_display: format_cents_or_blank(line.credit_cents),
        })
        .collect();

    ctx.insert("entries", &entry_views);
    ctx.insert("lines", &line_views);
    ctx.insert("total_debits", &format_cents(total_debit_cents));
    ctx.insert("total_credits", &format_cents(total_credit_cents));
    ctx.insert("is_balanced", &is_balanced);
    ctx.insert("start_date", start_date);
    ctx.insert("end_date", end_date);

    let rendered = state.tera.render("journal.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/journal/new")]
// Displays the form for creating one balanced journal entry.
pub async fn new_journal_entry_form(
    state: web::Data<AppState>,
    session: Session,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !can_create_accounting_records(&session) {
        return HttpResponse::Found()
            .append_header(("Location", "/journal"))
            .finish();
    }

    let accounts = get_active_chart_accounts(&state).await;
    ctx.insert("accounts", &accounts);

    let rendered = state.tera.render("journal_new.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[post("/journal")]
// Saves a balanced debit and credit pair as one journal entry.
pub async fn create_journal_entry(
    state: web::Data<AppState>,
    session: Session,
    form: web::Form<JournalEntryInput>,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !can_create_accounting_records(&session) {
        return HttpResponse::Found()
            .append_header(("Location", "/journal"))
            .finish();
    }

    let amount_cents = match parse_amount_to_cents(&form.amount) {
        Ok(cents) => cents,
        Err(error) => return render_journal_form_error(&state, &mut ctx, &error).await,
    };
    if amount_cents <= 0 {
        return render_journal_form_error(&state, &mut ctx, "Amount must be greater than zero.")
            .await;
    }

    if form.debit_account_id == form.credit_account_id {
        return render_journal_form_error(
            &state,
            &mut ctx,
            "Debit and credit accounts must be different.",
        )
        .await;
    }

    let entry_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let entry_no = format!("JE-{:04}", entry_count + 1);

    // Keep the entry header and both lines together, or save none of them on error.
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return render_journal_form_error(
                &state,
                &mut ctx,
                "Could not start journal transaction.",
            )
            .await;
        }
    };

    let result = sqlx::query(
        "INSERT INTO journal_entries (entry_no, entry_date, memo, created_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
    )
    .bind(&entry_no)
    .bind(&form.entry_date)
    .bind(&form.memo)
    .execute(&mut *tx)
    .await;

    match result {
        Ok(result) => {
            let journal_entry_id = result.last_insert_rowid();

            if sqlx::query(
                "INSERT INTO journal_lines (journal_entry_id, chart_account_id, debit_cents, credit_cents)
                 VALUES (?1, ?2, ?3, 0)",
            )
            .bind(journal_entry_id)
            .bind(form.debit_account_id)
            .bind(amount_cents)
            .execute(&mut *tx)
            .await
            .is_err()
            {
                return render_journal_form_error(
                    &state,
                    &mut ctx,
                    "Could not create debit journal line.",
                )
                .await;
            }

            if sqlx::query(
                "INSERT INTO journal_lines (journal_entry_id, chart_account_id, debit_cents, credit_cents)
                 VALUES (?1, ?2, 0, ?3)",
            )
            .bind(journal_entry_id)
            .bind(form.credit_account_id)
            .bind(amount_cents)
            .execute(&mut *tx)
            .await
            .is_err()
            {
                return render_journal_form_error(
                    &state,
                    &mut ctx,
                    "Could not create credit journal line.",
                )
                .await;
            }

            let details = format!("Created journal entry {}.", entry_no);

            if record_audit_event(
                &mut tx,
                &session,
                "created",
                "journal_entry",
                journal_entry_id,
                &details,
            )
            .await
            .is_err()
            {
                return render_journal_form_error(
                    &state,
                    &mut ctx,
                    "Could not save audit trail record.",
                )
                .await;
            }

            if tx.commit().await.is_err() {
                return render_journal_form_error(
                    &state,
                    &mut ctx,
                    "Could not save journal transaction.",
                )
                .await;
            }

            HttpResponse::Found()
                .append_header(("Location", "/journal"))
                .finish()
        }
        Err(_) => {
            render_journal_form_error(&state, &mut ctx, "Could not create journal entry.").await
        }
    }
}
