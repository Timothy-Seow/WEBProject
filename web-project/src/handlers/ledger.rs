use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, web};
use tera::Context;

use crate::{
    AppState,
    chart_account::ChartAccount,
    filters::DateRangeFilter,
    helpers::{add_user_to_ctx, format_cents},
    ledger::{LedgerAccount, LedgerAccountView, LedgerLine, LedgerLineView},
};

// Calculates the correct balance side for one account.
fn ledger_view_from_account(account: LedgerAccount) -> LedgerAccountView {
    let net_cents = if account.normal_balance == "Debit" {
        account.debit_total_cents - account.credit_total_cents
    } else {
        account.credit_total_cents - account.debit_total_cents
    };

    let (balance_side, balance_cents) = if net_cents >= 0 {
        (account.normal_balance.clone(), net_cents)
    } else if account.normal_balance == "Debit" {
        ("Credit".to_string(), -net_cents)
    } else {
        ("Debit".to_string(), -net_cents)
    };

    LedgerAccountView {
        id: account.id,
        code: account.code,
        name: account.name,
        account_type: account.account_type,
        debit_total: format_cents(account.debit_total_cents),
        credit_total: format_cents(account.credit_total_cents),
        balance_side,
        balance: format_cents(balance_cents),
    }
}

#[get("/ledger")]
// Shows debit totals, credit totals, and the current balance for every account.
pub async fn list_ledger_accounts(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    let accounts: Vec<LedgerAccount> = sqlx::query_as(
        "SELECT
            ca.id,
            ca.code,
            ca.name,
            ca.account_type,
            ca.normal_balance,
            ca.is_active,
            COALESCE(SUM(jl.debit_cents), 0) AS debit_total_cents,
            COALESCE(SUM(jl.credit_cents), 0) AS credit_total_cents
         FROM chart_accounts ca
         LEFT JOIN journal_lines jl ON jl.chart_account_id = ca.id
         GROUP BY
            ca.id,
            ca.code,
            ca.name,
            ca.account_type,
            ca.normal_balance,
            ca.is_active
         ORDER BY ca.code ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let ledger_accounts: Vec<LedgerAccountView> =
        accounts.into_iter().map(ledger_view_from_account).collect();

    ctx.insert("accounts", &ledger_accounts);

    let rendered = state.tera.render("ledger.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/ledger/{id}")]
// Shows the journal history for one selected chart account.
pub async fn view_ledger_account(
    state: web::Data<AppState>,
    session: Session,
    path: web::Path<i64>,
    query: web::Query<DateRangeFilter>,
) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    let account_id = path.into_inner();
    let start_date = query.start_date.as_deref().unwrap_or("");
    let end_date = query.end_date.as_deref().unwrap_or("");

    let account: Option<ChartAccount> = sqlx::query_as(
        "SELECT id, code, name, account_type, normal_balance, is_active
         FROM chart_accounts
         WHERE id = ?1",
    )
    .bind(account_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let Some(account) = account else {
        return HttpResponse::Found()
            .append_header(("Location", "/ledger"))
            .finish();
    };

    // Include activity before the filter so the first displayed balance is accurate.
    let opening_debit_cents: i64 = if start_date.is_empty() {
        0
    } else {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(jl.debit_cents), 0)
             FROM journal_lines jl
             JOIN journal_entries je ON je.id = jl.journal_entry_id
             WHERE jl.chart_account_id = ?1 AND je.entry_date < ?2",
        )
        .bind(account_id)
        .bind(start_date)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0)
    };
    let opening_credit_cents: i64 = if start_date.is_empty() {
        0
    } else {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(jl.credit_cents), 0)
             FROM journal_lines jl
             JOIN journal_entries je ON je.id = jl.journal_entry_id
             WHERE jl.chart_account_id = ?1 AND je.entry_date < ?2",
        )
        .bind(account_id)
        .bind(start_date)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0)
    };
    let opening_balance_cents = if account.normal_balance == "Debit" {
        opening_debit_cents - opening_credit_cents
    } else {
        opening_credit_cents - opening_debit_cents
    };

    let lines: Vec<LedgerLine> = sqlx::query_as(
        "SELECT
            je.entry_no,
            je.entry_date,
            je.memo,
            jl.debit_cents,
            jl.credit_cents
         FROM journal_lines jl
         JOIN journal_entries je ON je.id = jl.journal_entry_id
         WHERE jl.chart_account_id = ?1
           AND (?2 = '' OR je.entry_date >= ?2)
           AND (?3 = '' OR je.entry_date <= ?3)
         ORDER BY je.entry_date ASC, jl.id ASC",
    )
    .bind(account_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Build each history row while carrying its balance forward from earlier entries.
    let mut running_balance_cents = opening_balance_cents;
    let mut line_views: Vec<LedgerLineView> = Vec::new();

    for line in lines {
        let change_cents = if account.normal_balance == "Debit" {
            line.debit_cents - line.credit_cents
        } else {
            line.credit_cents - line.debit_cents
        };

        running_balance_cents += change_cents;

        let (running_balance_side, running_balance_amount) = if running_balance_cents >= 0 {
            (account.normal_balance.clone(), running_balance_cents)
        } else if account.normal_balance == "Debit" {
            ("Credit".to_string(), -running_balance_cents)
        } else {
            ("Debit".to_string(), -running_balance_cents)
        };

        line_views.push(LedgerLineView {
            entry_no: line.entry_no,
            entry_date: line.entry_date,
            memo: line.memo,
            debit_display: format_cents(line.debit_cents),
            credit_display: format_cents(line.credit_cents),
            running_balance_side,
            running_balance: format_cents(running_balance_amount),
        });
    }
    ctx.insert("account", &account);
    ctx.insert("lines", &line_views);
    ctx.insert("start_date", start_date);
    ctx.insert("end_date", end_date);
    ctx.insert("opening_balance_side", &account.normal_balance);
    ctx.insert(
        "opening_balance",
        &format_cents(opening_balance_cents.abs()),
    );

    let rendered = state.tera.render("ledger_detail.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}
