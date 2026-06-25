use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, web};
use chrono::Local;
use tera::Context;

use crate::{
    AppState,
    balance_sheet::{BalanceSheetAccount, BalanceSheetAccountView},
    filters::MonthFilter,
    helpers::{add_user_to_ctx, format_cents},
};

#[get("/reports/balance-sheet")]
// Shows Assets, Liabilities, Equity, and the current business result at month end.
pub async fn balance_sheet_report(
    state: web::Data<AppState>,
    session: Session,
    query: web::Query<MonthFilter>,
) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    let default_month = Local::now().format("%Y-%m").to_string();
    let requested_month = query
        .month
        .as_deref()
        .filter(|month| {
            chrono::NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").is_ok()
        })
        .unwrap_or(&default_month);
    let selected_month = if requested_month > default_month.as_str() {
        default_month.clone()
    } else {
        requested_month.to_string()
    };
    let first_day_of_month = format!("{selected_month}-01");

    let accounts: Vec<BalanceSheetAccount> = sqlx::query_as(
        "SELECT
            ca.code,
            ca.name,
            ca.account_type,
            COALESCE(SUM(CASE
                WHEN je.entry_date < date(?1, '+1 month') THEN jl.debit_cents
                ELSE 0
            END), 0) AS debit_total_cents,
            COALESCE(SUM(CASE
                WHEN je.entry_date < date(?1, '+1 month') THEN jl.credit_cents
                ELSE 0
            END), 0) AS credit_total_cents
         FROM chart_accounts ca
         LEFT JOIN journal_lines jl ON jl.chart_account_id = ca.id
         LEFT JOIN journal_entries je ON je.id = jl.journal_entry_id
         WHERE ca.account_type IN (
            'Asset',
            'Liability',
            'Equity',
            'Revenue',
            'Expense'
         )
         GROUP BY ca.code, ca.name, ca.account_type
         ORDER BY ca.account_type ASC, ca.code ASC",
    )
    .bind(&first_day_of_month)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut assets: Vec<BalanceSheetAccountView> = Vec::new();
    let mut liabilities: Vec<BalanceSheetAccountView> = Vec::new();
    let mut equity: Vec<BalanceSheetAccountView> = Vec::new();

    let mut total_assets_cents = 0;
    let mut total_liabilities_cents = 0;
    let mut total_equity_before_result_cents = 0;
    let mut current_result_cents = 0;

    // Sort balance-sheet accounts and fold Revenue and Expenses into current Equity.
    for account in accounts {
        let account_type = account.account_type.clone();

        let amount_cents = match account_type.as_str() {
            "Asset" | "Expense" => account.debit_total_cents - account.credit_total_cents,
            "Liability" | "Equity" | "Revenue" => {
                account.credit_total_cents - account.debit_total_cents
            }
            _ => 0,
        };

        match account_type.as_str() {
            "Asset" => {
                total_assets_cents += amount_cents;
                assets.push(BalanceSheetAccountView {
                    code: account.code,
                    name: account.name,
                    amount: format_cents(amount_cents),
                });
            }
            "Liability" => {
                total_liabilities_cents += amount_cents;
                liabilities.push(BalanceSheetAccountView {
                    code: account.code,
                    name: account.name,
                    amount: format_cents(amount_cents),
                });
            }
            "Equity" => {
                total_equity_before_result_cents += amount_cents;
                equity.push(BalanceSheetAccountView {
                    code: account.code,
                    name: account.name,
                    amount: format_cents(amount_cents),
                });
            }
            "Revenue" => {
                current_result_cents += amount_cents;
            }
            "Expense" => {
                current_result_cents -= amount_cents;
            }
            _ => {}
        }
    }

    let total_equity_cents = total_equity_before_result_cents + current_result_cents;
    let total_liabilities_and_equity_cents = total_liabilities_cents + total_equity_cents;

    let (result_label, result_amount_cents) = if current_result_cents >= 0 {
        ("Current Profit", current_result_cents)
    } else {
        ("Current Loss", -current_result_cents)
    };

    let is_balanced = total_assets_cents == total_liabilities_and_equity_cents;

    ctx.insert("assets", &assets);
    ctx.insert("liabilities", &liabilities);
    ctx.insert("equity", &equity);
    ctx.insert("total_assets", &format_cents(total_assets_cents));
    ctx.insert("total_liabilities", &format_cents(total_liabilities_cents));
    ctx.insert(
        "total_equity_before_result",
        &format_cents(total_equity_before_result_cents),
    );
    ctx.insert("result_label", result_label);
    ctx.insert("result_amount", &format_cents(result_amount_cents));
    ctx.insert("total_equity", &format_cents(total_equity_cents));
    ctx.insert(
        "total_liabilities_and_equity",
        &format_cents(total_liabilities_and_equity_cents),
    );
    ctx.insert("is_balanced", &is_balanced);
    ctx.insert("selected_month", &selected_month);
    ctx.insert("current_month", &default_month);

    let rendered = state.tera.render("balance_sheet.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}
