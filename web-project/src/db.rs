use bcrypt::{hash, DEFAULT_COST};
use chrono::Local;
use crate::models::BankAccount;
use rand::Rng;
use rusqlite::{params, Connection};

pub fn init_db(conn: &Connection) {
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
    )
    .expect("Failed to create tables");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
        .unwrap();

    if count == 0 {
        let now = now_str();
        let admin_pw = hash("admin123", DEFAULT_COST).unwrap();
        let cus_pw = hash("customer123", DEFAULT_COST).unwrap();
        let acc_num = gen_acc_num(conn);

        conn.execute(
            "INSERT INTO users (username,password_hash,email,name,phone,role,created_at)
             VALUES (?1,?2,?3,?4,?5,'admin',?6)",
            params!["admin", admin_pw, "admin@example.com", "System Administrator", "+65 6000 0000", now],
        )
        .ok();

        conn.execute(
            "INSERT INTO users (username,password_hash,email,name,phone,role,created_at)
             VALUES (?1,?2,?3,?4,?5,'customer',?6)",
            params!["john", cus_pw, "johndoe@example.com", "John Doe", "+65 6000 0001", now],
        )
        .ok();

        conn.execute(
            "INSERT INTO bank_accounts (account_number,user_id,account_type,balance,created_at)
                VALUES (?1,?2,'savings',5000.00,?3)",
            params![acc_num, 2, now],
        )
        .ok();
    }
}

fn now_str() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn gen_acc_num(conn: &Connection) -> String {
    loop {
        let n: u64 = rand::thread_rng().gen_range(1_000_000_000, 10_000_000_000);
        let num = n.to_string();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM bank_accounts WHERE account_number=?1)",
                params![num],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if !exists {
            return num;
        }
    }
}

pub fn get_user_accounts(conn: &Connection, user_id: i64) -> Vec<BankAccount> {
    let mut stmt = conn
        .prepare("SELECT account_number, account_type, balance FROM bank_accounts WHERE user_id=?1")
        .unwrap();
    let accounts_iter = stmt
        .query_map(params![user_id], |r| {
            Ok(BankAccount {
                id: 0,
                account_number: r.get(0)?,
                user_id,
                account_type: r.get(1)?,
                balance: r.get(2)?,
                created_at: String::new(),
            })
        })
        .unwrap();

    accounts_iter.filter_map(Result::ok).collect()
}
