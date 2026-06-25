use serde::Serialize;
use sqlx::FromRow;

// One chart account with totals read directly from the database query.
#[derive(Debug, Serialize, FromRow)]
pub struct LedgerAccount {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub normal_balance: String,
    pub is_active: bool,
    pub debit_total_cents: i64,
    pub credit_total_cents: i64,
}

// One ledger row prepared for display on the HTML page.
#[derive(Debug, Serialize)]
pub struct LedgerAccountView {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub debit_total: String,
    pub credit_total: String,
    pub balance_side: String,
    pub balance: String,
}

// One journal line shown in the history of a selected ledger account.
#[derive(Debug, Serialize, FromRow)]
pub struct LedgerLine {
    pub entry_no: String,
    pub entry_date: String,
    pub memo: String,
    pub debit_cents: i64,
    pub credit_cents: i64,
}

// One ledger history line with debit and credit amounts ready for HTML.
#[derive(Debug, Serialize)]
pub struct LedgerLineView {
    pub entry_no: String,
    pub entry_date: String,
    pub memo: String,
    pub debit_display: String,
    pub credit_display: String,
    pub running_balance_side: String,
    pub running_balance: String,
}
