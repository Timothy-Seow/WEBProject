use actix_session::Session;
use tera::Context;

// Formats integer cents as readable money, such as 2550 into 25.50.
pub(crate) fn format_cents(cents: i64) -> String {
    let absolute_cents = cents.abs();

    if cents < 0 {
        format!("-{}.{:02}", absolute_cents / 100, absolute_cents % 100)
    } else {
        format!("{}.{:02}", absolute_cents / 100, absolute_cents % 100)
    }
}

// Formats cents for a debit or credit cell, leaving zero values blank.
pub(crate) fn format_cents_or_blank(cents: i64) -> String {
    if cents == 0 {
        String::new()
    } else {
        format_cents(cents)
    }
}

// Converts a dollar amount from a form into exact integer cents.
pub(crate) fn parse_amount_to_cents(amount_str: &str) -> Result<i64, String> {
    let amount = amount_str.trim();

    if amount.is_empty() {
        return Err("Amount is required.".to_string());
    }

    if amount.starts_with('-') {
        return Err("Amount cannot be negative.".to_string());
    }

    let mut parts = amount.split('.');
    let dollars = parts.next().unwrap_or_default();
    let cents = parts.next().unwrap_or_default();

    if parts.next().is_some()
        || dollars.is_empty()
        || !dollars.chars().all(|character| character.is_ascii_digit())
        || !cents.chars().all(|character| character.is_ascii_digit())
        || cents.len() > 2
    {
        return Err("Enter an amount with up to two decimal places.".to_string());
    }

    let dollar_cents = dollars
        .parse::<i64>()
        .map_err(|_| "Amount is too large.".to_string())?
        .checked_mul(100)
        .ok_or_else(|| "Amount is too large.".to_string())?;

    let fractional_cents = match cents.len() {
        0 => 0,
        1 => cents.parse::<i64>().unwrap_or(0) * 10,
        2 => cents.parse::<i64>().unwrap_or(0),
        _ => unreachable!(),
    };

    dollar_cents
        .checked_add(fractional_cents)
        .ok_or_else(|| "Amount is too large.".to_string())
}

// Adds logged-in user details to a Tera context for shared page navigation.
pub(crate) fn add_user_to_ctx(session: &Session, ctx: &mut Context) -> bool {
    let id = session.get::<i64>("user_id").ok().flatten();
    let username = session.get::<String>("username").ok().flatten();
    let role = session.get::<String>("role").ok().flatten();
    let name = session.get::<String>("name").ok().flatten();

    if let (Some(_), Some(username), Some(role), Some(name)) = (id, username, role, name) {
        let is_admin = role == "admin";
        let can_create_records = is_admin || role == "accountant";

        ctx.insert("logged_in", &true);
        ctx.insert("session_username", &username);
        ctx.insert("session_role", &role);
        ctx.insert("session_name", &name);
        ctx.insert("is_admin", &is_admin);
        ctx.insert("can_create_records", &can_create_records);
        true
    } else {
        ctx.insert("logged_in", &false);
        false
    }
}

// Returns true when the logged-in user has the requested role.
pub(crate) fn has_role(session: &Session, required_role: &str) -> bool {
    session
        .get::<String>("role")
        .ok()
        .flatten()
        .is_some_and(|role| role == required_role)
}

// Returns true when a user may create accounting records.
pub(crate) fn can_create_accounting_records(session: &Session) -> bool {
    session
        .get::<String>("role")
        .ok()
        .flatten()
        .is_some_and(|role| role == "admin" || role == "accountant")
}
