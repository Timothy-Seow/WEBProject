use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
// A chart-of-accounts record read from the database and shown in templates.
pub struct ChartAccount {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub normal_balance: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
// The account details submitted from the create and edit forms.
pub struct ChartAccountInput {
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub normal_balance: String,
}
