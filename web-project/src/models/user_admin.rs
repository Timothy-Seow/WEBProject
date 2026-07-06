use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// One user account record, shown on the System Administrator's user page.
#[derive(Debug, Serialize, FromRow)]
pub struct UserAccount {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub name: String,
    pub phone: String,
    pub role: String,
    pub created_at: String,
}

// The details submitted from the new-user form.
#[derive(Debug, Deserialize)]
pub struct NewUserInput {
    pub username: String,
    pub password: String,
    pub email: String,
    pub name: String,
    pub phone: String,
    pub role: String,
}

// The role submitted from the inline role-reassignment form.
#[derive(Debug, Deserialize)]
pub struct RoleInput {
    pub role: String,
}

// Every role the System Administrator is allowed to assign.
pub const ASSIGNABLE_ROLES: [&str; 4] = ["sysadmin", "admin", "accountant", "viewer"];

pub fn is_valid_role(role: &str) -> bool {
    ASSIGNABLE_ROLES.contains(&role)
}
