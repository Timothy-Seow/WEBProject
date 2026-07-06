use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// One expense record read from the database with its account names.
#[derive(Debug, Serialize, FromRow)]
pub struct Expense {
    pub id: i64,
    pub expense_date: String,
    pub description: String,
    pub expense_account_code: String,
    pub expense_account_name: String,
    pub payment_account_code: String,
    pub payment_account_name: String,
    pub amount_cents: i64,
    pub journal_entry_id: i64,
    pub created_at: String,
}

// One expense record prepared for display on the HTML page.
#[derive(Debug, Serialize)]
pub struct ExpenseView {
    pub id: i64,
    pub expense_date: String,
    pub description: String,
    pub expense_account: String,
    pub payment_account: String,
    pub amount_display: String,
    pub journal_entry_id: i64,
}

// The expense details submitted from the new-expense form.
#[derive(Debug, Deserialize)]
pub struct ExpenseInput {
    pub expense_date: String,
    pub description: String,
    pub expense_account_id: i64,
    pub payment_account_id: i64,
    pub amount: String,
}
