use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// One customer record read from the database.
#[derive(Debug, Serialize, FromRow)]
pub struct Customer {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub created_at: String,
}

// The customer details submitted from the new-customer form.
#[derive(Debug, Deserialize)]
pub struct CustomerInput {
    pub name: String,
    pub email: String,
    pub phone: String,
}
