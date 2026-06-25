use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, post, web};
use tera::Context;

use crate::{
    AppState,
    audit::record_audit_event,
    customer::{Customer, CustomerInput},
    helpers::{add_user_to_ctx, can_create_accounting_records},
};

#[get("/customers")]
// Shows all saved customers.
pub async fn list_customers(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    let customers: Vec<Customer> = sqlx::query_as(
        "SELECT id, name, email, phone, created_at
         FROM customers
         ORDER BY name ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    ctx.insert("customers", &customers);

    let rendered = state.tera.render("customers.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/customers/new")]
// Displays the form for creating one customer.
pub async fn new_customer_form(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !can_create_accounting_records(&session) {
        return HttpResponse::Found()
            .append_header(("Location", "/customers"))
            .finish();
    }

    let rendered = state.tera.render("customers_new.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[post("/customers")]
// Validates and saves a new customer record.
pub async fn create_customer(
    state: web::Data<AppState>,
    session: Session,
    form: web::Form<CustomerInput>,
) -> impl Responder {
    let mut ctx = Context::new();

    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !can_create_accounting_records(&session) {
        return HttpResponse::Found()
            .append_header(("Location", "/customers"))
            .finish();
    }

    if form.name.trim().is_empty() {
        ctx.insert("error", "Customer name is required.");
        let rendered = state.tera.render("customers_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            ctx.insert("error", "Could not start customer transaction.");
            let rendered = state.tera.render("customers_new.html", &ctx).unwrap();
            return HttpResponse::Ok().content_type("text/html").body(rendered);
        }
    };

    let customer_id = match sqlx::query(
        "INSERT INTO customers (name, email, phone, created_at)
     VALUES (?1, ?2, ?3, datetime('now'))",
    )
    .bind(form.name.trim())
    .bind(form.email.trim())
    .bind(form.phone.trim())
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(_) => {
            ctx.insert("error", "Could not create customer.");
            let rendered = state.tera.render("customers_new.html", &ctx).unwrap();
            return HttpResponse::Ok().content_type("text/html").body(rendered);
        }
    };

    let details = format!("Created customer: {}.", form.name.trim());

    if record_audit_event(
        &mut tx,
        &session,
        "created",
        "customer",
        customer_id,
        &details,
    )
    .await
    .is_err()
    {
        ctx.insert("error", "Could not save audit trail record.");
        let rendered = state.tera.render("customers_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    if tx.commit().await.is_err() {
        ctx.insert("error", "Could not save customer transaction.");
        let rendered = state.tera.render("customers_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    HttpResponse::Found()
        .append_header(("Location", "/customers"))
        .finish()
}
