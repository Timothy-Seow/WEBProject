use actix_files as fs;
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::{App, HttpResponse, HttpServer, Responder, cookie::Key, middleware, web};
use bcrypt::verify;
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use tera::{Context, Tera};

mod audit;
mod db;
mod filters;
mod helpers;
mod pdf_generator;

#[path = "models/user.rs"]
mod user;

#[path = "models/chart_account.rs"]
mod chart_account;

#[path = "handlers/chart_accounts.rs"]
mod chart_accounts_handler;

#[path = "models/journal.rs"]
mod journal;

#[path = "handlers/journal.rs"]
mod journal_handler;

#[path = "models/ledger.rs"]
mod ledger;

#[path = "handlers/ledger.rs"]
mod ledger_handler;

#[path = "models/profit_loss.rs"]
mod profit_loss;

#[path = "handlers/profit_loss.rs"]
mod profit_loss_handler;

#[path = "models/balance_sheet.rs"]
mod balance_sheet;

#[path = "handlers/balance_sheet.rs"]
mod balance_sheet_handler;

#[path = "models/expense.rs"]
mod expense;

#[path = "handlers/expense.rs"]
mod expense_handler;

#[path = "models/customer.rs"]
mod customer;

#[path = "handlers/customer.rs"]
mod customer_handler;

#[path = "models/invoice.rs"]
mod invoice;

#[path = "handlers/invoice.rs"]
mod invoice_handler;

#[path = "models/invoice_pdf.rs"]
mod invoice_pdf;

#[path = "handlers/invoice_pdf.rs"]
mod invoice_pdf_handler;

#[path = "models/customer_payment.rs"]
mod customer_payment;

#[path = "handlers/customer_payment.rs"]
mod customer_payment_handler;

#[path = "models/audit_log.rs"]
mod audit_log;

#[path = "handlers/audit_trail.rs"]
mod audit_trail_handler;

use db::{get_user_by_username, init_db};
use helpers::{add_user_to_ctx, format_cents};
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
    let id = session.get::<i64>("user_id").ok()??;
    let username = session.get::<String>("username").ok()??;
    let role = session.get::<String>("role").ok()??;
    let name = session.get::<String>("name").ok()??;
    Some((id, username, role, name))
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

// DASHBOARD 
async fn dashboard(data: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return redirect("/login");
    }

    let role = session
        .get::<String>("role")
        .ok()
        .flatten()
        .unwrap_or_default();

    if role == "admin" {
        let cash_debits: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(jl.debit_cents), 0)
             FROM journal_lines jl
             JOIN chart_accounts ca ON ca.id = jl.chart_account_id
             WHERE ca.code = '1000'",
        )
        .fetch_one(&data.db)
        .await
        .unwrap_or(0);

        let cash_credits: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(jl.credit_cents), 0)
             FROM journal_lines jl
             JOIN chart_accounts ca ON ca.id = jl.chart_account_id
             WHERE ca.code = '1000'",
        )
        .fetch_one(&data.db)
        .await
        .unwrap_or(0);

        let receivable_debits: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(jl.debit_cents), 0)
             FROM journal_lines jl
             JOIN chart_accounts ca ON ca.id = jl.chart_account_id
             WHERE ca.code = '1100'",
        )
        .fetch_one(&data.db)
        .await
        .unwrap_or(0);

        let receivable_credits: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(jl.credit_cents), 0)
             FROM journal_lines jl
             JOIN chart_accounts ca ON ca.id = jl.chart_account_id
             WHERE ca.code = '1100'",
        )
        .fetch_one(&data.db)
        .await
        .unwrap_or(0);

        let unpaid_invoices: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoices WHERE status IN ('unpaid', 'partial')",
        )
        .fetch_one(&data.db)
        .await
        .unwrap_or(0);

        let monthly_revenue: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(jl.credit_cents - jl.debit_cents), 0)
             FROM journal_lines jl
             JOIN chart_accounts ca ON ca.id = jl.chart_account_id
             JOIN journal_entries je ON je.id = jl.journal_entry_id
             WHERE ca.account_type = 'Revenue'
               AND strftime('%Y-%m', je.entry_date) = strftime('%Y-%m', 'now')",
        )
        .fetch_one(&data.db)
        .await
        .unwrap_or(0);

        let monthly_expenses: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(jl.debit_cents - jl.credit_cents), 0)
             FROM journal_lines jl
             JOIN chart_accounts ca ON ca.id = jl.chart_account_id
             JOIN journal_entries je ON je.id = jl.journal_entry_id
             WHERE ca.account_type = 'Expense'
               AND strftime('%Y-%m', je.entry_date) = strftime('%Y-%m', 'now')",
        )
        .fetch_one(&data.db)
        .await
        .unwrap_or(0);

        ctx.insert("cash_balance", &format_cents(cash_debits - cash_credits));
        ctx.insert(
            "outstanding_receivables",
            &format_cents(receivable_debits - receivable_credits),
        );
        ctx.insert("unpaid_invoices", &unpaid_invoices);
        ctx.insert("monthly_revenue", &format_cents(monthly_revenue));
        ctx.insert("monthly_expenses", &format_cents(monthly_expenses));
        ctx.insert("is_admin", &true);
    } else {
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
        .connect("sqlite://accounting.db")
        .await
        .expect("Failed to open database");
    init_db(&db).await.expect("Failed to initialize database");

    // Anchored to the crate's own folder (not the process's current working
    // directory) so the app finds templates/static the same way no matter
    // where `cargo run` / the .exe is launched from (fixes "works on my
    // machine but not my teammate's" path issues across OSes).
    let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
        .expect("Failed to load templates");
    let secret_key = Key::generate(); // for encrypting session cookies
    let app_state = web::Data::new(AppState { db, tera });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false)
                    .build(),
            )
            .wrap(middleware::Logger::default())
            // Static files (absolute path, see comment on Tera::new above)
            .service(
                fs::Files::new("/static", concat!(env!("CARGO_MANIFEST_DIR"), "/static"))
                    .show_files_listing(),
            )
            // Root
            .route("/", web::get().to(root))
            // Login/logout
            .route("/login", web::get().to(login_page))
            .route("/login", web::post().to(login_post))
            .route("/logout", web::get().to(logout))
            // Dashboard
            .route("/dashboard", web::get().to(dashboard))
            // Chart of Accounts
            .service(chart_accounts_handler::list_chart_accounts)
            .service(chart_accounts_handler::new_chart_account_form)
            .service(chart_accounts_handler::create_chart_account)
            .service(chart_accounts_handler::edit_chart_account_form)
            .service(chart_accounts_handler::update_chart_account)
            .service(chart_accounts_handler::deactivate_chart_account)
            // Journal entry pages
            .service(journal_handler::list_journal_entries)
            .service(journal_handler::new_journal_entry_form)
            .service(journal_handler::create_journal_entry)
            // General ledger page
            .service(ledger_handler::list_ledger_accounts)
            // Individual ledger account history page
            .service(ledger_handler::view_ledger_account)
            // Profit and Loss report
            .service(profit_loss_handler::profit_loss_report)
            // Balance Sheet report
            .service(balance_sheet_handler::balance_sheet_report)
            // Expense pages and automatic journal posting
            .service(expense_handler::list_expenses)
            .service(expense_handler::new_expense_form)
            .service(expense_handler::create_expense)
            // Customer pages
            .service(customer_handler::list_customers)
            .service(customer_handler::new_customer_form)
            .service(customer_handler::create_customer)
            // Invoice pages
            .service(invoice_handler::list_invoices)
            .service(invoice_handler::new_invoice_form)
            .service(invoice_handler::create_invoice)
            .service(invoice_pdf_handler::download_invoice_pdf)
            // Customer Payment pages
            .service(customer_payment_handler::new_payment_form)
            .service(customer_payment_handler::create_payment)
            // Audit trail page
            .service(audit_trail_handler::list_audit_logs)
    })
    .bind(("127.0.0.1", 9876))?
    .run()
    .await
}
