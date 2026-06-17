use actix_session::Session;
use actix_web::{get, web, Responder, HttpResponse};
use tera::Context;
use crate::{AppState, models_transactions::Transaction};

fn add_user_to_ctx(session: &Session, ctx: &mut Context) -> bool {
    let id = session.get::<i64>("user_id").ok().flatten();
    let username = session.get::<String>("username").ok().flatten();
    let role = session.get::<String>("role").ok().flatten();
    let name = session.get::<String>("name").ok().flatten();

    if let (Some(_), Some(username), Some(role), Some(name)) = (id, username, role, name) {
        ctx.insert("logged_in", &true);
        ctx.insert("session_username", &username);
        ctx.insert("session_role", &role);
        ctx.insert("session_name", &name);
        true
    } else {
        ctx.insert("logged_in", &false);
        false
    }
}

#[get("/transactions")]
pub async fn list_transactions(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    let rows: Vec<Transaction> = sqlx::query_as::<_, Transaction>(
        "SELECT id, date, description, amount FROM transactions ORDER BY date DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap();

    ctx.insert("transactions", &rows);

    let rendered = state.tera.render("transactions.html", &ctx).unwrap();
    HttpResponse::Ok().body(rendered)
}
