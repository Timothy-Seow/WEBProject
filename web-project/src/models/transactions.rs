use serde::Serialize;
use sqlx::FromRow;

#[derive(Serialize, FromRow)]
pub struct Transaction {
    pub id: i32,
    pub date: String,
    pub description: String,
    pub amount: f64,
}