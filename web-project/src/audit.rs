use actix_session::Session;
use sqlx::{Sqlite, Transaction};

// Saves one audit event inside the same transaction as the accounting action.
pub(crate) async fn record_audit_event(
    transaction: &mut Transaction<'_, Sqlite>,
    session: &Session,
    action: &str,
    entity_type: &str,
    entity_id: i64,
    details: &str,
) -> Result<(), String> {
    let user_id = session
        .get::<i64>("user_id")
        .map_err(|_| "Could not read the logged-in user.".to_string())?
        .ok_or_else(|| "Missing logged-in user ID.".to_string())?;

    let username = session
        .get::<String>("username")
        .map_err(|_| "Could not read the logged-in username.".to_string())?
        .ok_or_else(|| "Missing logged-in username.".to_string())?;

    sqlx::query(
        "INSERT INTO audit_logs
         (user_id, username, action, entity_type, entity_id, details, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
    )
    .bind(user_id)
    .bind(username)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(details)
    .execute(&mut **transaction)
    .await
    .map_err(|_| "Could not save audit event.".to_string())?;

    Ok(())
}
