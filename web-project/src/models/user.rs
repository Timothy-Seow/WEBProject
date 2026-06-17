use serde::{Deserialize, Serialize};
use sqlx::FromRow;
// ----- ALL STRUCTS GO HERE -----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String, // stuff like admin or customer
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct BankAccount {
    pub id: i64,
    pub account_number: String,
    pub user_id: i64,
    pub account_type: String, // savings / fixed deposit
    pub balance: f64,
    pub created_at: String,
}

