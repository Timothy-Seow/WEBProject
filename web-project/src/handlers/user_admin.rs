use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, post, web};
use bcrypt::{DEFAULT_COST, hash};
use tera::Context;

use crate::{
    AppState,
    audit::record_audit_event,
    helpers::{add_user_to_ctx, is_admin},
    user_admin::{NewUserInput, RoleInput, UserAccount, is_valid_role},
};

// Redirects anyone who isn't logged in, or isn't the System Administrator,
// away from every route in this file. Even normal "admin" accounts are
// blocked here on purpose: only "admin" may view, create, delete, or
// reassign roles for user accounts.
fn require_admin(session: &Session, ctx: &mut Context) -> Option<HttpResponse> {
    if !add_user_to_ctx(session, ctx) {
        return Some(
            HttpResponse::Found()
                .append_header(("Location", "/login"))
                .finish(),
        );
    }

    if !is_admin(session) {
        return Some(
            HttpResponse::Found()
                .append_header(("Location", "/dashboard"))
                .finish(),
        );
    }

    None
}

#[get("/admin/users")]
// Lists every user account and its assigned role.
pub async fn list_users(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();
    if let Some(response) = require_admin(&session, &mut ctx) {
        return response;
    }

    let users: Vec<UserAccount> = sqlx::query_as(
        "SELECT id, username, email, name, phone, role, created_at
         FROM users
         ORDER BY username ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let current_user_id = session.get::<i64>("user_id").ok().flatten();

    ctx.insert("users", &users);
    ctx.insert("current_user_id", &current_user_id);
    ctx.insert("roles", &["admin", "accountant", "viewer"]);

    let rendered = state.tera.render("users.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/admin/users/new")]
// Displays the form for creating a new user account.
pub async fn new_user_form(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();
    if let Some(response) = require_admin(&session, &mut ctx) {
        return response;
    }

    ctx.insert("roles", &["admin", "accountant", "viewer"]);

    let rendered = state.tera.render("users_new.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[post("/admin/users")]
// Validates and saves a newly submitted user account.
pub async fn create_user(
    state: web::Data<AppState>,
    session: Session,
    form: web::Form<NewUserInput>,
) -> impl Responder {
    let mut ctx = Context::new();
    if let Some(response) = require_admin(&session, &mut ctx) {
        return response;
    }
    ctx.insert("roles", &["admin", "accountant", "viewer"]);

    let username = form.username.trim();
    let email = form.email.trim();
    let name = form.name.trim();
    let phone = form.phone.trim();
    let role = form.role.trim();

    if username.is_empty() || email.is_empty() || name.is_empty() || form.password.is_empty() {
        ctx.insert("error", "Username, email, name, and password are required.");
        let rendered = state.tera.render("users_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    if !is_valid_role(role) {
        ctx.insert("error", "Choose a valid role.");
        let rendered = state.tera.render("users_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    let password_hash = match hash(&form.password, DEFAULT_COST) {
        Ok(hashed) => hashed,
        Err(_) => {
            ctx.insert("error", "Could not secure the password.");
            let rendered = state.tera.render("users_new.html", &ctx).unwrap();
            return HttpResponse::Ok().content_type("text/html").body(rendered);
        }
    };

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            ctx.insert("error", "Could not start user transaction.");
            let rendered = state.tera.render("users_new.html", &ctx).unwrap();
            return HttpResponse::Ok().content_type("text/html").body(rendered);
        }
    };

    let user_id = match sqlx::query(
        "INSERT INTO users (username, password_hash, email, name, phone, role, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
    )
    .bind(username)
    .bind(&password_hash)
    .bind(email)
    .bind(name)
    .bind(phone)
    .bind(role)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(_) => {
            ctx.insert(
                "error",
                "Could not create user. Check that the username and email are unique.",
            );
            let rendered = state.tera.render("users_new.html", &ctx).unwrap();
            return HttpResponse::Ok().content_type("text/html").body(rendered);
        }
    };

    let details = format!("Created user {} with role {}.", username, role);

    if record_audit_event(&mut tx, &session, "created", "user", user_id, &details)
        .await
        .is_err()
    {
        ctx.insert("error", "Could not save audit trail record.");
        let rendered = state.tera.render("users_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    if tx.commit().await.is_err() {
        ctx.insert("error", "Could not save user transaction.");
        let rendered = state.tera.render("users_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    HttpResponse::Found()
        .append_header(("Location", "/admin/users"))
        .finish()
}

#[post("/admin/users/{id}/role")]
// Reassigns the role of an existing user account.
pub async fn update_user_role(
    state: web::Data<AppState>,
    session: Session,
    path: web::Path<i64>,
    form: web::Form<RoleInput>,
) -> impl Responder {
    let mut ctx = Context::new();
    if let Some(response) = require_admin(&session, &mut ctx) {
        return response;
    }

    let user_id = path.into_inner();
    let role = form.role.trim();

    if !is_valid_role(role) {
        return HttpResponse::Found()
            .append_header(("Location", "/admin/users"))
            .finish();
    }

    // An Administrator can't demote themselves this way, which would
    // otherwise be able to lock every admin out of this page.
    let current_user_id = session.get::<i64>("user_id").ok().flatten();
    if current_user_id == Some(user_id) {
        return HttpResponse::Found()
            .append_header(("Location", "/admin/users"))
            .finish();
    }

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let target: Option<String> =
        match sqlx::query_scalar("SELECT username FROM users WHERE id = ?1")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
        {
            Ok(row) => row,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    let username = match target {
        Some(username) => username,
        None => {
            return HttpResponse::Found()
                .append_header(("Location", "/admin/users"))
                .finish();
        }
    };

    let updated = match sqlx::query("UPDATE users SET role = ?1 WHERE id = ?2")
        .bind(role)
        .bind(user_id)
        .execute(&mut *tx)
        .await
    {
        Ok(result) => result,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if updated.rows_affected() == 0 {
        return HttpResponse::Found()
            .append_header(("Location", "/admin/users"))
            .finish();
    }

    let details = format!("Reassigned user {} to role {}.", username, role);

    if record_audit_event(&mut tx, &session, "updated", "user", user_id, &details)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if tx.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Found()
        .append_header(("Location", "/admin/users"))
        .finish()
}

#[post("/admin/users/{id}/delete")]
// Removes a user account. Only reachable by a System Administrator.
pub async fn delete_user(
    state: web::Data<AppState>,
    session: Session,
    path: web::Path<i64>,
) -> impl Responder {
    let mut ctx = Context::new();
    if let Some(response) = require_admin(&session, &mut ctx) {
        return response;
    }

    let user_id = path.into_inner();

    // Prevent a System Administrator from deleting their own account.
    let current_user_id = session.get::<i64>("user_id").ok().flatten();
    if current_user_id == Some(user_id) {
        return HttpResponse::Found()
            .append_header(("Location", "/admin/users"))
            .finish();
    }

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let target: Option<String> =
        match sqlx::query_scalar("SELECT username FROM users WHERE id = ?1")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
        {
            Ok(row) => row,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    let username = match target {
        Some(username) => username,
        None => {
            return HttpResponse::Found()
                .append_header(("Location", "/admin/users"))
                .finish();
        }
    };

    let deleted = match sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
    {
        Ok(result) => result,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if deleted.rows_affected() == 0 {
        return HttpResponse::Found()
            .append_header(("Location", "/admin/users"))
            .finish();
    }

    let details = format!("Deleted user {}.", username);

    if record_audit_event(&mut tx, &session, "deleted", "user", user_id, &details)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if tx.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Found()
        .append_header(("Location", "/admin/users"))
        .finish()
}
