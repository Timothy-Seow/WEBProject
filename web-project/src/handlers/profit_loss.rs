use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, web};
use tera::Context;

use crate::{
    AppState,
    filters::DateRangeFilter,
    helpers::{add_user_to_ctx, format_cents},
    profit_loss::{ProfitLossAccount, ProfitLossAccountView},
};

#[get("/reports/profit-loss")]
// Shows all-time Revenue, Expenses, and the resulting profit or loss.
pub async fn profit_loss_report(
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

    let accounts: Vec<ProfitLossAccount> = sqlx::query_as(
        "SELECT
            ca.code,
            ca.name,
            ca.account_type,
            COALESCE(SUM(jl.debit_cents), 0) AS debit_total_cents,
            COALESCE(SUM(jl.credit_cents), 0) AS credit_total_cents
        FROM chart_accounts ca
        LEFT JOIN journal_lines jl ON jl.chart_account_id = ca.id
        LEFT JOIN journal_entries je ON je.id = jl.journal_entry_id
        WHERE ca.account_type IN ('Revenue', 'Expense')
        AND (?1 = '' OR je.entry_date >= ?1)
        AND (?2 = '' OR je.entry_date <= ?2)
        GROUP BY ca.code, ca.name, ca.account_type
        ORDER BY ca.account_type ASC, ca.code ASC",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut revenues: Vec<ProfitLossAccountView> = Vec::new();
    let mut expenses: Vec<ProfitLossAccountView> = Vec::new();
    let mut total_revenue_cents = 0;
    let mut total_expense_cents = 0;

    for account in accounts {
        let is_revenue = account.account_type == "Revenue";

        let amount_cents = if is_revenue {
            account.credit_total_cents - account.debit_total_cents
        } else {
            account.debit_total_cents - account.credit_total_cents
        };

        let view = ProfitLossAccountView {
            code: account.code,
            name: account.name,
            amount: format_cents(amount_cents),
        };

        if is_revenue {
            total_revenue_cents += amount_cents;
            revenues.push(view);
        } else {
            total_expense_cents += amount_cents;
            expenses.push(view);
        }
    }

    let net_result_cents = total_revenue_cents - total_expense_cents;

    let (result_label, result_amount_cents) = if net_result_cents >= 0 {
        ("Net Profit", net_result_cents)
    } else {
        ("Net Loss", -net_result_cents)
    };

    ctx.insert("revenues", &revenues);
    ctx.insert("expenses", &expenses);
    ctx.insert("total_revenue", &format_cents(total_revenue_cents));
    ctx.insert("total_expenses", &format_cents(total_expense_cents));
    ctx.insert("result_label", result_label);
    ctx.insert("result_amount", &format_cents(result_amount_cents));
    ctx.insert("start_date", start_date);
    ctx.insert("end_date", end_date);

    let rendered = state.tera.render("profit_loss.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}
