use bcrypt::{hash, DEFAULT_COST};
use chrono::Local;
use crate::models::BankAccount;
use rand::Rng;
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
            role TEXT NOT NULL DEFAULT 'customer',
            created_at TEXT NOT NULL
        );",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS bank_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account_number TEXT NOT NULL UNIQUE,
            user_id INTEGER NOT NULL,
            account_type TEXT NOT NULL DEFAULT 'savings',
            balance REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            description TEXT NOT NULL,
            amount REAL NOT NULL
        );",
    )
    .await?;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    if count == 0 {
        let now = now_str();
        let admin_pw = hash("admin123", DEFAULT_COST).unwrap();
        let cus_pw = hash("customer123", DEFAULT_COST).unwrap();
        let acc_num = gen_acc_num(pool).await;

        sqlx::query(
            "INSERT INTO users (username,password_hash,email,name,phone,role,created_at)
             VALUES (?1,?2,?3,?4,?5,'admin',?6)",
        )
        .bind("admin")
        .bind(admin_pw)
        .bind("admin@example.com")
        .bind("System Administrator")
        .bind("+65 6000 0000")
        .bind(&now)
        .execute(pool)
        .await?;

        sqlx::query(
            "INSERT INTO users (username,password_hash,email,name,phone,role,created_at)
             VALUES (?1,?2,?3,?4,?5,'customer',?6)",
        )
        .bind("john")
        .bind(cus_pw)
        .bind("johndoe@example.com")
        .bind("John Doe")
        .bind("+65 6000 0001")
        .bind(&now)
        .execute(pool)
        .await?;

        sqlx::query(
            "INSERT INTO bank_accounts (account_number,user_id,account_type,balance,created_at)
                VALUES (?1,?2,'savings',5000.00,?3)",
        )
        .bind(&acc_num)
        .bind(2_i64)
        .bind(&now)
        .execute(pool)
        .await?;

        sqlx::query(
            "INSERT INTO transactions (date,description,amount)
             VALUES (?1,?2,?3)",
        )
        .bind(&now)
        .bind("food")
        .bind(7.00_f64)
        .execute(pool)
        .await?;

        sqlx::query(
            "INSERT INTO transactions (date,description,amount)
             VALUES (?1,?2,?3)",
        )
        .bind(&now)
        .bind("transport")
        .bind(12.00_f64)
        .execute(pool)
        .await?;
    }

    let tx_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    if tx_count == 0 {
        let now = now_str();

        sqlx::query(
            "INSERT INTO transactions (date,description,amount)
             VALUES (?1,?2,?3)",
        )
        .bind(&now)
        .bind("food")
        .bind(7.00_f64)
        .execute(pool)
        .await?;

        sqlx::query(
            "INSERT INTO transactions (date,description,amount)
             VALUES (?1,?2,?3)",
        )
        .bind(&now)
        .bind("transport")
        .bind(12.00_f64)
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn now_str() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

async fn gen_acc_num(pool: &SqlitePool) -> String {
    loop {
        let n: u64 = rand::thread_rng().gen_range(1_000_000_000, 10_000_000_000);
        let num = n.to_string();
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bank_accounts WHERE account_number=?1)",
        )
        .bind(&num)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if !exists {
            return num;
        }
    }
}

pub async fn get_user_accounts(pool: &SqlitePool, user_id: i64) -> Result<Vec<BankAccount>, sqlx::Error> {
    let accounts = sqlx::query_as::<_, BankAccount>(
        "SELECT 0 AS id, account_number, user_id, account_type, balance, created_at FROM bank_accounts WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(accounts)
}

pub async fn get_user_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<(i64, String, String, String, String)>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, username, password_hash, role, name FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| (
        r.get::<i64, _>("id"),
        r.get::<String, _>("username"),
        r.get::<String, _>("password_hash"),
        r.get::<String, _>("role"),
        r.get::<String, _>("name"),
    )))
}
