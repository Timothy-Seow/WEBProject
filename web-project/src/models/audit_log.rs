use serde::Serialize;
use sqlx::FromRow;

// Represents one permanent record of an important user action.
#[derive(Debug, FromRow, Serialize)]
pub struct AuditLog {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: i64,
    pub details: String,
    pub created_at: String,
}
