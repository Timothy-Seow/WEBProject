use crate::{
    AppState,
    audit::record_audit_event,
    chart_account::{ChartAccount, ChartAccountInput},
    helpers::{add_user_to_ctx, has_role},
};
use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, post, web};
use tera::Context;

// Maps the first account-code digit to its required accounting category.
fn expected_account_type_for_code(code: &str) -> Option<&'static str> {
    match code.chars().next()? {
        '1' => Some("Asset"),
        '2' => Some("Liability"),
        '3' => Some("Equity"),
        '4' => Some("Revenue"),
        '5' => Some("Expense"),
        _ => None,
    }
}

// Returns the normal debit or credit side for an account category.
fn expected_normal_balance_for_type(account_type: &str) -> Option<&'static str> {
    match account_type {
        "Asset" | "Expense" => Some("Debit"),
        "Liability" | "Equity" | "Revenue" => Some("Credit"),
        _ => None,
    }
}

// Checks that a submitted account follows the project's code and balance rules.
fn validate_chart_account_input(form: &ChartAccountInput) -> Result<(), String> {
    let code = form.code.trim();
    let account_type = form.account_type.trim();
    let normal_balance = form.normal_balance.trim();

    if code.len() != 4 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("Account code must be exactly 4 digits.".to_string());
    }

    let expected_type = expected_account_type_for_code(code)
        .ok_or_else(|| "Account code must start with 1, 2, 3, 4, or 5.".to_string())?;

    if account_type != expected_type {
        return Err(format!(
            "Code {} belongs to {} accounts, but account type is {}.",
            code, expected_type, account_type
        ));
    }

    let expected_balance = expected_normal_balance_for_type(account_type).ok_or_else(|| {
        "Account type must be Asset, Liability, Equity, Revenue, or Expense.".to_string()
    })?;

    if normal_balance != expected_balance {
        return Err(format!(
            "{} accounts must use {} as the normal balance.",
            account_type, expected_balance
        ));
    }

    Ok(())
}

// Rebuilds an account from form input so an invalid edit can be shown again.
fn chart_account_from_input(id: i64, form: &ChartAccountInput) -> ChartAccount {
    ChartAccount {
        id,
        code: form.code.trim().to_string(),
        name: form.name.trim().to_string(),
        account_type: form.account_type.trim().to_string(),
        normal_balance: form.normal_balance.trim().to_string(),
        is_active: true,
    }
}

#[get("/accounts")]
// Shows every chart-of-accounts record, including deactivated accounts.
pub async fn list_chart_accounts(state: web::Data<AppState>, session: Session) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    let accounts: Vec<ChartAccount> = sqlx::query_as::<_, ChartAccount>(
        "SELECT id, code, name, account_type, normal_balance, is_active
         FROM chart_accounts
         ORDER BY code ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    ctx.insert("accounts", &accounts);

    let rendered = state.tera.render("accounts.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[get("/accounts/new")]
// Displays the form for creating a new chart account.
pub async fn new_chart_account_form(
    state: web::Data<AppState>,
    session: Session,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !has_role(&session, "admin") {
        return HttpResponse::Found()
            .append_header(("Location", "/accounts"))
            .finish();
    }

    let rendered = state.tera.render("accounts_new.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

#[post("/accounts")]
// Validates and saves a newly submitted chart account.
pub async fn create_chart_account(
    state: web::Data<AppState>,
    session: Session,
    form: web::Form<ChartAccountInput>,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    if !has_role(&session, "admin") {
        return HttpResponse::Found()
            .append_header(("Location", "/accounts"))
            .finish();
    }

    if let Err(error) = validate_chart_account_input(&form) {
        ctx.insert("error", &error);
        let rendered = state.tera.render("accounts_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            ctx.insert("error", "Could not start account transaction.");
            let rendered = state.tera.render("accounts_new.html", &ctx).unwrap();
            return HttpResponse::Ok().content_type("text/html").body(rendered);
        }
    };

    let account_id = match sqlx::query(
        "INSERT INTO chart_accounts (code, name, account_type, normal_balance, is_active)
     VALUES (?1, ?2, ?3, ?4, 1)",
    )
    .bind(form.code.trim())
    .bind(form.name.trim())
    .bind(form.account_type.trim())
    .bind(form.normal_balance.trim())
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(_) => {
            ctx.insert(
                "error",
                "Could not create account. Check that the account code is unique.",
            );
            let rendered = state.tera.render("accounts_new.html", &ctx).unwrap();
            return HttpResponse::Ok().content_type("text/html").body(rendered);
        }
    };

    let details = format!(
        "Created chart account {} {}.",
        form.code.trim(),
        form.name.trim()
    );

    if record_audit_event(
        &mut tx,
        &session,
        "created",
        "chart_account",
        account_id,
        &details,
    )
    .await
    .is_err()
    {
        ctx.insert("error", "Could not save audit trail record.");
        let rendered = state.tera.render("accounts_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    if tx.commit().await.is_err() {
        ctx.insert("error", "Could not save account transaction.");
        let rendered = state.tera.render("accounts_new.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    HttpResponse::Found()
        .append_header(("Location", "/accounts"))
        .finish()
}

#[get("/accounts/{id}/edit")]
// Loads one account into the edit form using its database ID.
pub async fn edit_chart_account_form(
    state: web::Data<AppState>,
    session: Session,
    path: web::Path<i64>,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }
    if !has_role(&session, "admin") {
        return HttpResponse::Found()
            .append_header(("Location", "/accounts"))
            .finish();
    }

    let account_id = path.into_inner();
    let account = sqlx::query_as::<_, ChartAccount>(
        "SELECT id, code, name, account_type, normal_balance, is_active
         FROM chart_accounts
         WHERE id = ?1",
    )
    .bind(account_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    match account {
        Some(account) => {
            ctx.insert("account", &account);
            let rendered = state.tera.render("accounts_edit.html", &ctx).unwrap();
            HttpResponse::Ok().content_type("text/html").body(rendered)
        }
        None => HttpResponse::Found()
            .append_header(("Location", "/accounts"))
            .finish(),
    }
}

#[post("/accounts/{id}/edit")]
// Validates and saves edits to an existing chart account.
pub async fn update_chart_account(
    state: web::Data<AppState>,
    session: Session,
    path: web::Path<i64>,
    form: web::Form<ChartAccountInput>,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }
    if !has_role(&session, "admin") {
        return HttpResponse::Found()
            .append_header(("Location", "/accounts"))
            .finish();
    }

    let account_id = path.into_inner();

    if let Err(error) = validate_chart_account_input(&form) {
        let account = chart_account_from_input(account_id, &form);
        ctx.insert("account", &account);
        ctx.insert("error", &error);
        let rendered = state.tera.render("accounts_edit.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            let account = chart_account_from_input(account_id, &form);
            ctx.insert("account", &account);
            ctx.insert("error", "Could not start account transaction.");
            let rendered = state.tera.render("accounts_edit.html", &ctx).unwrap();
            return HttpResponse::Ok().content_type("text/html").body(rendered);
        }
    };

    let updated = match sqlx::query(
        "UPDATE chart_accounts
     SET code = ?1, name = ?2, account_type = ?3, normal_balance = ?4
     WHERE id = ?5",
    )
    .bind(form.code.trim())
    .bind(form.name.trim())
    .bind(form.account_type.trim())
    .bind(form.normal_balance.trim())
    .bind(account_id)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let account = chart_account_from_input(account_id, &form);
            ctx.insert("account", &account);
            ctx.insert(
                "error",
                "Could not update account. Check that the account code is unique.",
            );
            let rendered = state.tera.render("accounts_edit.html", &ctx).unwrap();
            return HttpResponse::Ok().content_type("text/html").body(rendered);
        }
    };

    if updated.rows_affected() == 0 {
        return HttpResponse::Found()
            .append_header(("Location", "/accounts"))
            .finish();
    }

    let details = format!(
        "Updated chart account {} {}.",
        form.code.trim(),
        form.name.trim()
    );

    if record_audit_event(
        &mut tx,
        &session,
        "updated",
        "chart_account",
        account_id,
        &details,
    )
    .await
    .is_err()
    {
        let account = chart_account_from_input(account_id, &form);
        ctx.insert("account", &account);
        ctx.insert("error", "Could not save audit trail record.");
        let rendered = state.tera.render("accounts_edit.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    if tx.commit().await.is_err() {
        let account = chart_account_from_input(account_id, &form);
        ctx.insert("account", &account);
        ctx.insert("error", "Could not save account transaction.");
        let rendered = state.tera.render("accounts_edit.html", &ctx).unwrap();
        return HttpResponse::Ok().content_type("text/html").body(rendered);
    }

    HttpResponse::Found()
        .append_header(("Location", "/accounts"))
        .finish()
}

#[post("/accounts/{id}/deactivate")]
// Hides an account from future journal entries without deleting its history.
pub async fn deactivate_chart_account(
    state: web::Data<AppState>,
    session: Session,
    path: web::Path<i64>,
) -> impl Responder {
    let mut ctx = Context::new();
    if !add_user_to_ctx(&session, &mut ctx) {
        return HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }
    if !has_role(&session, "admin") {
        return HttpResponse::Found()
            .append_header(("Location", "/accounts"))
            .finish();
    }

    let account_id = path.into_inner();

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let account = match sqlx::query_as::<_, ChartAccount>(
        "SELECT id, code, name, account_type, normal_balance, is_active
     FROM chart_accounts
     WHERE id = ?1 AND is_active = 1",
    )
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(account)) => account,
        Ok(None) => {
            return HttpResponse::Found()
                .append_header(("Location", "/accounts"))
                .finish();
        }
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let deactivated = match sqlx::query(
        "UPDATE chart_accounts
     SET is_active = 0
     WHERE id = ?1 AND is_active = 1",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if deactivated.rows_affected() == 0 {
        return HttpResponse::Found()
            .append_header(("Location", "/accounts"))
            .finish();
    }

    let details = format!(
        "Deactivated chart account {} {}.",
        account.code, account.name
    );

    if record_audit_event(
        &mut tx,
        &session,
        "deactivated",
        "chart_account",
        account_id,
        &details,
    )
    .await
    .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if tx.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Found()
        .append_header(("Location", "/accounts"))
        .finish()
}
