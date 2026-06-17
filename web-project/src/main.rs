use actix_files as fs;
use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::{cookie::Key, middleware, web, App, HttpResponse, HttpServer, Responder};
use bcrypt::verify;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use tera::{Context, Tera};

mod db;

#[path = "models/user.rs"]
mod user;

#[path = "models/transactions.rs"]
mod models_transactions;

#[path = "handlers/transactions.rs"]
mod transaction_handler;

use db::{get_user_accounts, get_user_by_username, init_db};
use user::LoginInput;


// app state for db and html
pub struct AppState {
    pub db: SqlitePool,
    pub tera: Tera,
}

//fn to bring u to another page
fn redirect(path: &str) -> HttpResponse {
    HttpResponse::Found()
        .append_header(("Location", path))
        .finish()
}


// Get session user (uses session library to save user info in cookies, so can check who is logged without calling DB every time)
// not taught so idk if can use lol
fn get_session_user(session: &Session) -> Option<(i64, String, String, String)> {
    let id        = session.get::<i64>("user_id").ok()??;
    let username  = session.get::<String>("username").ok()??;
    let role      = session.get::<String>("role").ok()??;
    let name = session.get::<String>("name").ok()??;
    Some((id, username, role, name))
}

fn add_user_to_ctx(session: &Session, ctx: &mut Context) -> bool {
    if let Some((id, username, role, name)) = get_session_user(session) {
        ctx.insert("logged_in",       &true);
        ctx.insert("session_user_id", &id);
        ctx.insert("session_username",&username);
        ctx.insert("session_role",    &role);
        ctx.insert("session_name",&name);
        true
    } else {
        ctx.insert("logged_in", &false);
        false
    }
}

// Authenticate user when logging in
async fn login_page(data: web::Data<AppState>, session: Session) -> impl Responder {
    if get_session_user(&session).is_some() {
        return redirect("/dashboard");
    }
    let mut ctx = Context::new();
    ctx.insert("page_title", "Login");
    HttpResponse::Ok()
        .content_type("text/html")
        .body(data.tera.render("login.html", &ctx).unwrap())
}

// call to DB and check whether username and password returns anything
async fn login_post(
    data: web::Data<AppState>,
    session: Session,
    form: web::Form<LoginInput>,
) -> impl Responder {
    let result = get_user_by_username(&data.db, &form.username).await;
    match result {
        Ok(Some((id, username, hash_val, role, name))) => {
            if verify(&form.password, &hash_val).unwrap() {
                session.insert("user_id", id).ok();
                session.insert("username", &username).ok();
                session.insert("role", &role).ok();
                session.insert("name", &name).ok();
                redirect("/dashboard")
            } else {
                render_login_error(&data, "Invalid username or password.")
            }
        }
        _ => render_login_error(&data, "Invalid username or password."),
    }
}

fn render_login_error(data: &AppState, msg: &str) -> HttpResponse {
    let mut ctx = Context::new();
    ctx.insert("page_title", "Login");
    ctx.insert("error", msg);
    HttpResponse::Ok()
        .content_type("text/html")
        .body(data.tera.render("login.html", &ctx).unwrap())
}

async fn logout(session: Session) -> impl Responder {
    session.purge();
    redirect("/login")
}

// DASHBOARD (not finished, got a lot of stuff to add)
async fn dashboard(
    data: web::Data<AppState>,
    session: Session,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return redirect("/login");
    }

    let role    = session.get::<String>("role").ok().flatten().unwrap_or_default();

    if role == "admin" {
         let total_accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bank_accounts")
             .fetch_one(&data.db)
             .await
             .unwrap_or(0);
         ctx.insert("total_accounts", &total_accounts);
         ctx.insert("is_admin", &true);
    } else {
        // customer dash
        let user_id = session.get::<i64>("user_id").ok().flatten().unwrap_or_default();
        let accounts = get_user_accounts(&data.db, user_id).await.unwrap_or_default();
        ctx.insert("accounts", &accounts);
        ctx.insert("is_admin", &false);
    }

    let rendered = data.tera.render("dashboard.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

// use root to redirect to login page or dahsboard if already logged in
async fn root(session: Session) -> impl Responder {
    if get_session_user(&session).is_some() {
        redirect("/dashboard")
    } else {
        redirect("/login")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = SqlitePoolOptions::new()
        .connect("sqlite://bank.db")
        .await
        .expect("Failed to open database");
    init_db(&db).await.expect("Failed to initialize database");

    let tera = Tera::new("templates/**/*").expect("Failed to load templates");
    let secret_key = Key::generate(); // for encrypting session cookies
    let app_state = web::Data::new(AppState {
        db,
        tera,
    });

    HttpServer::new(move || {
    App::new()
        .app_data(app_state.clone())
        .wrap(
            SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                .cookie_secure(false)
                .build(),
        )
        .wrap(middleware::Logger::default())
        // Static files
        .service(fs::Files::new("/static", "./static").show_files_listing())
        // Root
        .route("/", web::get().to(root))
        // Login/logout
        .route("/login", web::get().to(login_page))
        .route("/login", web::post().to(login_post))
        .route("/logout", web::get().to(logout))
        // Dashboard
        .route("/dashboard", web::get().to(dashboard))
        // Transactions
        .service(transaction_handler::list_transactions)

    })
    .bind(("127.0.0.1", 9876))?
    .run()
    .await
}