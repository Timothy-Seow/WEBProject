use bcrypt::{DEFAULT_COST, hash};
use chrono::Local;
use sqlx::sqlite::SqlitePool;
use sqlx::{Executor, Row};

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    pool.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            phone TEXT NOT NULL DEFAULT '',
            role TEXT NOT NULL DEFAULT 'viewer',
            created_at TEXT NOT NULL
        );",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS audit_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            username TEXT NOT NULL,
            action TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id INTEGER NOT NULL,
            details TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );",
    )
    .await?;

    // Development-only System Administrator account. Only this role can
    // manage user accounts (create, delete, reassign roles).
    seed_demo_user(
        pool,
        "sysadmin",
        "sysadmin123",
        "sysadmin@example.com",
        "Demo System Administrator",
        "sysadmin",
    )
    .await?;

    // Development-only users for checking accountant and viewer permissions.
    seed_demo_user(
        pool,
        "accountant",
        "accountant123",
        "accountant@example.com",
        "Demo Accountant",
        "accountant",
    )
    .await?;

    seed_demo_user(
        pool,
        "viewer",
        "viewer123",
        "viewer@example.com",
        "Demo Viewer",
        "viewer",
    )
    .await?;

    // Stores the project's account categories and their accounting rules.
    pool.execute(
        "CREATE TABLE IF NOT EXISTS chart_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            code TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            account_type TEXT NOT NULL,
            normal_balance TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        );",
    )
    .await?;

    // Stores one journal entry header, shared by its debit and credit lines.
    pool.execute(
        "CREATE TABLE IF NOT EXISTS journal_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entry_no TEXT NOT NULL UNIQUE,
            entry_date TEXT NOT NULL,
            memo TEXT NOT NULL,
            created_at TEXT NOT NULL
        );",
    )
    .await?;

    // Stores the individual debit and credit sides of each journal entry.
    pool.execute(
        "CREATE TABLE IF NOT EXISTS journal_lines (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            journal_entry_id INTEGER NOT NULL,
            chart_account_id INTEGER NOT NULL,
            debit_cents INTEGER NOT NULL DEFAULT 0,
            credit_cents INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (journal_entry_id) REFERENCES journal_entries(id),
            FOREIGN KEY (chart_account_id) REFERENCES chart_accounts(id)
        );",
    )
    .await?;

    // Stores expense records linked to their automatic journal entries.
    pool.execute(
        "CREATE TABLE IF NOT EXISTS expenses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            expense_date TEXT NOT NULL,
            description TEXT NOT NULL,
            expense_account_id INTEGER NOT NULL,
            payment_account_id INTEGER NOT NULL,
            amount_cents INTEGER NOT NULL,
            journal_entry_id INTEGER NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            FOREIGN KEY (expense_account_id) REFERENCES chart_accounts(id),
            FOREIGN KEY (payment_account_id) REFERENCES chart_accounts(id),
            FOREIGN KEY (journal_entry_id) REFERENCES journal_entries(id)
        );",
    )
    .await?;

    // Stores customer contact details for invoices and future payments.
    pool.execute(
        "CREATE TABLE IF NOT EXISTS customers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL DEFAULT '',
            phone TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        );",
    )
    .await?;

    // Stores customer invoices linked to their automatic revenue journal entries.
    pool.execute(
        "CREATE TABLE IF NOT EXISTS invoices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            invoice_no TEXT NOT NULL UNIQUE,
            customer_id INTEGER NOT NULL,
            invoice_date TEXT NOT NULL,
            due_date TEXT NOT NULL,
            description TEXT NOT NULL,
            amount_cents INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'unpaid',
            journal_entry_id INTEGER NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            FOREIGN KEY (customer_id) REFERENCES customers(id),
            FOREIGN KEY (journal_entry_id) REFERENCES journal_entries(id)
        );",
    )
    .await?;

    // Stores customer payments linked to invoices and their automatic journal entries.
    pool.execute(
        "CREATE TABLE IF NOT EXISTS customer_payments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            invoice_id INTEGER NOT NULL,
            payment_date TEXT NOT NULL,
            amount_cents INTEGER NOT NULL,
            cash_account_id INTEGER NOT NULL,
            journal_entry_id INTEGER NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            FOREIGN KEY (invoice_id) REFERENCES invoices(id),
            FOREIGN KEY (cash_account_id) REFERENCES chart_accounts(id),
            FOREIGN KEY (journal_entry_id) REFERENCES journal_entries(id)
        );",
    )
    .await?;

    migrate_customer_payments_for_partial_payments(pool).await?;

    // Adds the starter account list only when the chart is empty.
    let account_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chart_accounts")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    if account_count == 0 {
        let accounts = [
            ("1000", "Cash", "Asset", "Debit"),
            ("1100", "Accounts Receivable", "Asset", "Debit"),
            ("2000", "Accounts Payable", "Liability", "Credit"),
            ("2100", "Tax Payable", "Liability", "Credit"),
            ("3000", "Owner Equity", "Equity", "Credit"),
            ("4000", "Sales Revenue", "Revenue", "Credit"),
            ("5000", "Expenses", "Expense", "Debit"),
        ];

        for (code, name, account_type, normal_balance) in accounts {
            sqlx::query(
                "INSERT INTO chart_accounts (code, name, account_type, normal_balance, is_active)
                 VALUES (?1, ?2, ?3, ?4, 1)",
            )
            .bind(code)
            .bind(name)
            .bind(account_type)
            .bind(normal_balance)
            .execute(pool)
            .await?;
        }
    }

    // Adds one balanced opening entry only when no journal history exists yet.
    let journal_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    if journal_count == 0 {
        let now = now_str();
        let cash_account_id: i64 =
            sqlx::query_scalar("SELECT id FROM chart_accounts WHERE code = ?1")
                .bind("1000")
                .fetch_one(pool)
                .await?;

        let equity_account_id: i64 =
            sqlx::query_scalar("SELECT id FROM chart_accounts WHERE code = ?1")
                .bind("3000")
                .fetch_one(pool)
                .await?;

        let result = sqlx::query(
            "INSERT INTO journal_entries (entry_no, entry_date, memo, created_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind("JE-0001")
        .bind(Local::now().format("%Y-%m-%d").to_string())
        .bind("Opening balance")
        .bind(&now)
        .execute(pool)
        .await?;

        let journal_entry_id = result.last_insert_rowid();

        sqlx::query(
            "INSERT INTO journal_lines (journal_entry_id, chart_account_id, debit_cents, credit_cents)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(journal_entry_id)
        .bind(cash_account_id)
        .bind(500_000_i64)
        .bind(0_i64)
        .execute(pool)
        .await?;

        sqlx::query(
            "INSERT INTO journal_lines (journal_entry_id, chart_account_id, debit_cents, credit_cents)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(journal_entry_id)
        .bind(equity_account_id)
        .bind(0_i64)
        .bind(500_000_i64)
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn now_str() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

// Removes the old one-payment-per-invoice rule when an older database is opened.
async fn migrate_customer_payments_for_partial_payments(
    pool: &SqlitePool,
) -> Result<(), sqlx::Error> {
    let table_sql: Option<String> = sqlx::query_scalar(
        "SELECT sql FROM sqlite_master
         WHERE type = 'table' AND name = 'customer_payments'",
    )
    .fetch_optional(pool)
    .await?;

    if !table_sql
        .as_deref()
        .is_some_and(|sql| sql.contains("invoice_id INTEGER NOT NULL UNIQUE"))
    {
        return Ok(());
    }

    pool.execute("ALTER TABLE customer_payments RENAME TO customer_payments_old;")
        .await?;

    pool.execute(
        "CREATE TABLE customer_payments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            invoice_id INTEGER NOT NULL,
            payment_date TEXT NOT NULL,
            amount_cents INTEGER NOT NULL,
            cash_account_id INTEGER NOT NULL,
            journal_entry_id INTEGER NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            FOREIGN KEY (invoice_id) REFERENCES invoices(id),
            FOREIGN KEY (cash_account_id) REFERENCES chart_accounts(id),
            FOREIGN KEY (journal_entry_id) REFERENCES journal_entries(id)
        );",
    )
    .await?;

    pool.execute(
        "INSERT INTO customer_payments (
            id,
            invoice_id,
            payment_date,
            amount_cents,
            cash_account_id,
            journal_entry_id,
            created_at
        )
        SELECT
            id,
            invoice_id,
            payment_date,
            amount_cents,
            cash_account_id,
            journal_entry_id,
            created_at
        FROM customer_payments_old;",
    )
    .await?;

    pool.execute("DROP TABLE customer_payments_old;").await?;

    pool.execute(
        "UPDATE invoices
         SET status = CASE
            WHEN COALESCE((
                SELECT SUM(cp.amount_cents)
                FROM customer_payments cp
                WHERE cp.invoice_id = invoices.id
            ), 0) >= amount_cents THEN 'paid'
            WHEN COALESCE((
                SELECT SUM(cp.amount_cents)
                FROM customer_payments cp
                WHERE cp.invoice_id = invoices.id
            ), 0) > 0 THEN 'partial'
            ELSE 'unpaid'
         END;",
    )
    .await?;

    Ok(())
}

// Creates a demo user once without changing an existing account on later restarts.
async fn seed_demo_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    email: &str,
    name: &str,
    role: &str,
) -> Result<(), sqlx::Error> {
    let password_hash = hash(password, DEFAULT_COST).expect("Demo password should hash");

    sqlx::query(
        "INSERT OR IGNORE INTO users
         (username, password_hash, email, name, phone, role, created_at)
         VALUES (?1, ?2, ?3, ?4, '', ?5, ?6)",
    )
    .bind(username)
    .bind(password_hash)
    .bind(email)
    .bind(name)
    .bind(role)
    .bind(now_str())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_user_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<(i64, String, String, String, String)>, sqlx::Error> {
    let row =
        sqlx::query("SELECT id, username, password_hash, role, name FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await?;

    Ok(row.map(|r| {
        (
            r.get::<i64, _>("id"),
            r.get::<String, _>("username"),
            r.get::<String, _>("password_hash"),
            r.get::<String, _>("role"),
            r.get::<String, _>("name"),
        )
    }))
}
