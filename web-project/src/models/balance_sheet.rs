use serde::Serialize;
use sqlx::FromRow;

// An account with journal totals used to build the Balance Sheet.
#[derive(Debug, Serialize, FromRow)]
pub struct BalanceSheetAccount {
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub debit_total_cents: i64,
    pub credit_total_cents: i64,
}

// One balance-sheet account prepared for display on the report page.
#[derive(Debug, Serialize)]
pub struct BalanceSheetAccountView {
    pub code: String,
    pub name: String,
    pub amount: String,
}
