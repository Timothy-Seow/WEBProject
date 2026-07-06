use serde::Serialize;
use sqlx::FromRow;

// One Revenue or Expense account with totals read from the journal.
#[derive(Debug, Serialize, FromRow)]
pub struct ProfitLossAccount {
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub debit_total_cents: i64,
    pub credit_total_cents: i64,
}

// One Revenue or Expense account prepared for the report page.
#[derive(Debug, Serialize)]
pub struct ProfitLossAccountView {
    pub code: String,
    pub name: String,
    pub amount: String,
}
