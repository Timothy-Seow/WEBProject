use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// An invoice with an unpaid balance available for customer payment.
#[derive(Debug, Serialize, FromRow)]
pub struct UnpaidInvoice {
    pub id: i64,
    pub invoice_no: String,
    pub customer_name: String,
    pub amount_cents: i64,
    pub paid_cents: i64,
    pub remaining_cents: i64,
}

// The same invoice prepared with readable money values for the form.
#[derive(Debug, Serialize)]
pub struct UnpaidInvoiceView {
    pub id: i64,
    pub invoice_no: String,
    pub customer_name: String,
    pub amount_display: String,
    pub paid_display: String,
    pub remaining_display: String,
}

// The payment details submitted from the payment form.
#[derive(Debug, Deserialize)]
pub struct CustomerPaymentInput {
    pub invoice_id: i64,
    pub payment_date: String,
    pub amount: String,
    pub cash_account_id: i64,
}
