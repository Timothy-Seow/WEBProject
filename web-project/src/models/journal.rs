use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
// One journal entry header, such as its date and memo.
pub struct JournalEntry {
    pub id: i64,
    pub entry_no: String,
    pub entry_date: String,
    pub memo: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
// One debit or credit line belonging to a journal entry.
pub struct JournalLine {
    pub id: i64,
    pub journal_entry_id: i64,
    pub account_code: String,
    pub account_name: String,
    pub debit_cents: i64,
    pub credit_cents: i64,
}

#[derive(Debug, Serialize)]
// A journal line with money already formatted for the HTML page.
pub struct JournalLineView {
    pub id: i64,
    pub journal_entry_id: i64,
    pub account_code: String,
    pub account_name: String,
    pub debit_display: String,
    pub credit_display: String,
}

#[derive(Debug, Deserialize)]
// The journal entry details submitted from the new-entry form.
pub struct JournalEntryInput {
    pub entry_date: String,
    pub memo: String,
    pub debit_account_id: i64,
    pub credit_account_id: i64,
    pub amount: String,
}

#[derive(Debug, Serialize)]
// A journal entry with its own debit, credit, and balance summary.
pub struct JournalEntryView {
    pub id: i64,
    pub entry_no: String,
    pub entry_date: String,
    pub memo: String,
    pub debit_total: String,
    pub credit_total: String,
    pub is_balanced: bool,
}
