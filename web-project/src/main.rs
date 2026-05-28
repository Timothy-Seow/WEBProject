use actix_files as fs;
use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::{
    cookie::Key,
    middleware,
    web, App, HttpResponse, HttpServer, Responder,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tera::{Context, Tera};
use chrono::Local;
use bcrypt::{hash, verify, DEFAULT_COST};

// app state for db and html
pub struct AppState {
    pub db: Mutex<Connection>,
    pub tera: Tera,
}

// ----- ALL STRUCTS GO HERE -----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String, // stuff like admin or patient
    pub full_name: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

// ----- DB INITIALIZATION IS HERE -----
// enter stuff into DB (keep adding on here to make it neat)
fn init_db(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL,
            full_name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "
        
        // add more tables here as needed, like appointments, medical records, etc.

    ).expect("Failed to create tables");


    // Get all users from the database
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0)).unwrap_or(0);
    if count == 0 {
        // Insert a default admin user if the users table is empty
        let password_hash = hash("admin123", DEFAULT_COST).unwrap();
        conn.execute(
            "INSERT INTO users (username, email, password_hash, role, full_name, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["admin", "admin@example.com", password_hash, "admin", "Admin User", Local::now().to_rfc3339()],
        ).expect("Failed to insert default admin user");
    }
}

// Get session user (uses session library to save user info in cookies, so can check who is logged without calling DB every time)
// not taught so idk if can use lol
fn get_session_user(session: &Session) -> Option<(i64, String, String, String)> {
    let id = session.get::<i64>("user_id").ok()??;
    let username = session.get::<String>("username").ok()??;
    let role = session.get::<String>("role").ok()??;
    let full_name = session.get::<String>("full_name").ok()??;
    Some((id, username, role, full_name)) // either returns user info or None
}

// add user to context for tera templates, so can show user info on pages without calling DB every time (like showing name in header)
fn add_user_to_ctx(session: &Session, ctx: &mut Context) -> bool {
    if let Some((id, username, role, full_name)) = get_session_user(session) {
        ctx.insert("logged_in", &true);
        ctx.insert("session_user_id", &id);
        ctx.insert("session_username", &username);
        ctx.insert("session_role", &role);
        ctx.insert("session_full_name", &full_name);
        true
    } else {
        ctx.insert("logged_in", &false);
        false
    }
}

// Authenticate user when logging in
async fn login_page(
    data: web::Data<AppState>,
    session: Session,
) -> impl Responder {
    if get_session_user(&session).is_some() {
        return HttpResponse::Found()
            .append_header(("Location", "/dashboard"))
            .finish();
    }
    let mut ctx = Context::new();
    ctx.insert("page_title", "Login");
    let rendered = data.tera.render("login.html", &ctx).unwrap_or_default();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

// call to DB and check whether username and password returns anything
async fn login_post(
    data: web::Data<AppState>,
    session: Session,
    form: web::Form<LoginInput>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let result = conn.query_row(
        "SELECT id, username, password_hash, role, full_name FROM users WHERE username = ?1",
        params![form.username],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        },
    );

    // compares entered password with hash in DB, if matches then save user info in session and go to dashboard
    match result {
        Ok((id, username, hash_val, role, full_name)) => {
            if verify(&form.password, &hash_val).unwrap_or(false) {
                session.insert("user_id", id).ok();
                session.insert("username", &username).ok();
                session.insert("role", &role).ok();
                session.insert("full_name", &full_name).ok();
                HttpResponse::Found()
                    .append_header(("Location", "/dashboard"))
                    .finish()
            } else {
                drop(conn);
                let mut ctx = Context::new();
                ctx.insert("page_title", "Login");
                ctx.insert("error", "Invalid username or password.");
                let rendered = data.tera.render("login.html", &ctx).unwrap_or_default();
                HttpResponse::Ok().content_type("text/html").body(rendered)
            }
        }
        Err(_) => {
            drop(conn);
            let mut ctx = Context::new();
            ctx.insert("page_title", "Login");
            ctx.insert("error", "Invalid username or password.");
            let rendered = data.tera.render("login.html", &ctx).unwrap_or_default();
            HttpResponse::Ok().content_type("text/html").body(rendered)
        }
    }
}

async fn logout(session: Session) -> impl Responder {
    session.purge();
    HttpResponse::Found()
        .append_header(("Location", "/login"))
        .finish()
}

// DASHBOARD (not finished, got a lot of stuff to add)
async fn dashboard(
    data: web::Data<AppState>,
    session: Session,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    ctx.insert("page_title", "Dashboard");

    let rendered = data.tera.render("dashboard.html", &ctx).unwrap_or_default();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

// use root to redirect to login page or dahsboard if already logged in
async fn root(session: Session) -> impl Responder {
    if get_session_user(&session).is_some() {
        HttpResponse::Found().append_header(("Location", "/dashboard")).finish()
    } else {
        HttpResponse::Found().append_header(("Location", "/login")).finish()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let conn = Connection::open("hospital.db").expect("Failed to open database");
    init_db(&conn);

    let tera = Tera::new("templates/**/*").expect("Failed to load templates");
    let secret_key = Key::generate(); // for encrypting session cookies
    let app_state = web::Data::new(AppState {
        db: Mutex::new(conn),
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

    })
    .bind(("127.0.0.1", 9876))?
    .run()
    .await
}