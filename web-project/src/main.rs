use actix_files as fs;
use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::{cookie::Key, middleware, web, App, HttpResponse, HttpServer, Responder};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Local};
use rand::Rng;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tera::{Context, Tera};


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
    pub role: String, // stuff like admin or customer
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BankAccount {
    pub id: i64,
    pub account_number: String,
    pub user_id: i64,
    pub account_type: String, // savings / fixed deposit
    pub balance: f64,
    pub created_at: String,
}


// ----- DB INITIALIZATION IS HERE -----
// enter stuff into DB (keep adding on here to make it neat)
fn init_db(conn: &Connection) {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            phone TEXT NOT NULL DEFAULT '',
            role TEXT NOT NULL DEFAULT 'customer',
            created_at TEXT NOT NULL
        );
            CREATE TABLE IF NOT EXISTS bank_accounts (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            account_number TEXT NOT NULL UNIQUE,
            user_id        INTEGER NOT NULL,
            account_type   TEXT NOT NULL DEFAULT 'savings',
            balance        REAL NOT NULL DEFAULT 0,
            created_at     TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)

        );
        "
    ).expect("Failed to create tables");


    // Check if table is empty and create default admin user if it is (if yall wanna add extra stuff need do it outside the IF or delete DB)
    // TO DO. Should only admin be able to open a new user or can users create themselves? For now no sign up procedure
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap();

    if count == 0 {
        let now = now_str();
        let admin_pw = hash("admin123", DEFAULT_COST).unwrap();
        let cus_pw  = hash("customer123", DEFAULT_COST).unwrap();
        let acc_num = gen_acc_num(conn);

        conn.execute(
            "INSERT INTO users (username,password_hash,email,name,phone,role,created_at)
             VALUES (?1,?2,?3,?4,?5,'admin',?6)",
            params!["admin", admin_pw, "admin@example.com", "System Administrator", "+65 6000 0000", now],
        ).ok();

        conn.execute(
            "INSERT INTO users (username,password_hash,email,name,phone,role,created_at)
             VALUES (?1,?2,?3,?4,?5,'customer',?6)",
            params!["john", cus_pw, "johndoe@example.com", "John Doe", "+65 6000 0001", now],
        ).ok();

        conn.execute(
            "INSERT INTO bank_accounts (account_number,user_id,account_type,balance,created_at)
                VALUES (?1,?2,'savings',5000.00,?3)",
            params![acc_num, 2, now],
        ).ok();        
    }
}

// Helper functions
// get the current time (for created_at field in DB)
fn now_str() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

//fn to bring u to another page
fn redirect(path: &str) -> HttpResponse {
    HttpResponse::Found()
        .append_header(("Location", path))
        .finish()
}

// gen 10 digit acc num
fn gen_acc_num(conn: &Connection) -> String {
    loop {
        let n: u64 = rand::thread_rng().gen_range(1_000_000_000, 10_000_000_000);
        let num = n.to_string();
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM bank_accounts WHERE account_number=?1)",
            params![num],
            |r| r.get(0),
        ).unwrap_or(false);
        if !exists {
            return num;
        }
    }
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
    let conn = data.db.lock().unwrap();
    let result = conn.query_row(
        "SELECT id,username,password_hash,role,name FROM users WHERE username=?1",
        params![form.username],
        |r| Ok((r.get::<_,i64>(0)?, r.get::<_,String>(1)?, r.get::<_,String>(2)?,
                 r.get::<_,String>(3)?, r.get::<_,String>(4)?)),
    );
    drop(conn);
    match result {
        Ok((id, username, hash_val, role, name)) => {
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
        Err(_) => render_login_error(&data, "Invalid username or password."),
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
    let conn    = data.db.lock().unwrap();

    if role == "admin" {
         let total_accounts = conn.query_row("SELECT COUNT(*) FROM bank_accounts", [], |r| r.get(0)).unwrap_or(0);
         ctx.insert("total_accounts", &total_accounts);
         ctx.insert("is_admin", &true);
    } else {
        // customer dash
        let accounts = get_user_accounts(&conn, session.get::<i64>("user_id").unwrap().unwrap());
        ctx.insert("accounts", &accounts);
        ctx.insert("is_admin", &false);
    }

    let rendered = data.tera.render("dashboard.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

// get the accounts for a user
fn get_user_accounts(conn: &Connection, user_id: i64) -> Vec<BankAccount> {
    let mut stmt = conn.prepare("SELECT account_number, account_type, balance FROM bank_accounts WHERE user_id=?1").unwrap();
    let accounts_iter = stmt.query_map(params![user_id], |r| {
        Ok(BankAccount {
            id: 0, // not needed for display
            account_number: r.get(0)?,
            user_id,
            account_type: r.get(1)?,
            balance: r.get(2)?,
            created_at: String::new(), // not needed for display
        })
    }).unwrap();
    accounts_iter.filter_map(Result::ok).collect()
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
    let conn = Connection::open("bank.db").expect("Failed to open database");
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