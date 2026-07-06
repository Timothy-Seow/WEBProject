use serde::Serialize;
use sqlx::FromRow;

// Invoice and customer details prepared for PDF generation.
#[derive(Debug, Serialize, FromRow)]
pub struct InvoicePdfData {
    pub invoice_no: String,
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    pub invoice_date: String,
    pub due_date: String,
    pub description: String,
    pub amount_cents: i64,
    pub status: String,
}
