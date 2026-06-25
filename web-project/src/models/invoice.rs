use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// One invoice read from the database with its customer's name.
#[derive(Debug, Serialize, FromRow)]
pub struct Invoice {
    pub id: i64,
    pub invoice_no: String,
    pub customer_name: String,
    pub invoice_date: String,
    pub due_date: String,
    pub description: String,
    pub amount_cents: i64,
    pub paid_cents: i64,
    pub status: String,
    pub journal_entry_id: i64,
    pub created_at: String,
}

// One invoice prepared for display on the HTML page.
#[derive(Debug, Serialize)]
pub struct InvoiceView {
    pub id: i64,
    pub invoice_no: String,
    pub customer_name: String,
    pub invoice_date: String,
    pub due_date: String,
    pub description: String,
    pub amount_display: String,
    pub paid_display: String,
    pub balance_display: String,
    pub status: String,
    pub journal_entry_id: i64,
}

// The invoice details submitted from the new-invoice form.
#[derive(Debug, Deserialize)]
pub struct InvoiceInput {
    pub customer_id: i64,
    pub invoice_date: String,
    pub due_date: String,
    pub description: String,
    pub amount: String,
}
