use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, web};
use tera::Context;

use crate::{
    AppState,
    audit_log::AuditLog,
    helpers::{add_user_to_ctx, has_role},
};

// Shows recent important actions to administrators.
#[get("/audit")]
pub async fn list_audit_logs(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !has_role(&session, "admin") {
        return HttpResponse::Found()
            .append_header(("Location", "/dashboard"))
            .finish();
    }

    let logs: Vec<AuditLog> = sqlx::query_as(
        "SELECT id, user_id, username, action, entity_type, entity_id, details, created_at
         FROM audit_logs
         ORDER BY id DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    ctx.insert("logs", &logs);

    let rendered = state.tera.render("audit.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}
