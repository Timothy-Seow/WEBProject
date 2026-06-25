# Setup and Demo Guide

## Run the App

From the `web-project` folder:

```bash
PDF_PYTHON="$PWD/.venv/bin/python" cargo run
```

Then open:

```text
http://127.0.0.1:9876
```

If PDF export is not needed for a quick check, the app can also run with:

```bash
cargo run
```

## PDF Setup

Invoice PDF export uses Python and ReportLab.

One-time setup from the `web-project` folder:

```bash
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
```

After setup, use this run command when testing PDF downloads:

```bash
PDF_PYTHON="$PWD/.venv/bin/python" cargo run
```

## Demo Accounts

| Role | Username | Password |
|---|---|---|
| Admin | `admin` | `admin123` |
| Accountant | `accountant` | `accountant123` |
| Viewer | `viewer` | `viewer123` |

## Current Demo Data

The cleaned demo database starts with:

- Standard Chart of Accounts.
- One opening balance journal entry.
- One demo customer.
- One demo invoice.
- Two customer payments that fully settle the invoice.
- Audit trail records for the demo workflow.

## Suggested Demo Flow

1. Log in as `admin`.
2. Show the Dashboard summary cards.
3. Show Chart of Accounts and explain account codes/types.
4. Show Customers and the demo customer.
5. Show Invoices and the demo invoice.
6. Explain the partial payment workflow:
   - first payment marks the invoice as `partial`
   - second payment marks the invoice as `paid`
7. Show Journal entries created automatically from invoice and payment actions.
8. Show Ledger and ledger detail running balances.
9. Show Profit and Loss report.
10. Show monthly Balance Sheet report.
11. Download the invoice PDF.
12. Show Audit Trail as admin.
13. Log in as `accountant` to show accounting record creation permissions.
14. Log in as `viewer` to show read-only access.

## Role Permissions

| Feature | Admin | Accountant | Viewer |
|---|---|---|---|
| View dashboard and reports | Yes | Yes | Yes |
| View Chart of Accounts | Yes | Yes | Yes |
| Create journal entries | Yes | Yes | No |
| Create expenses | Yes | Yes | No |
| Create customers | Yes | Yes | No |
| Create invoices | Yes | Yes | No |
| Record customer payments | Yes | Yes | No |
| Manage Chart of Accounts | Yes | No | No |
| View Audit Trail | Yes | No | No |

## Important Notes

- `bank.db` is the SQLite database file used by the app.
- Keep `bank.db` if you want to preserve demo data.
- If the database is deleted, the app recreates the schema and starter accounts on startup.
- PDF download requires the app to be started with `PDF_PYTHON`.
- The only known compile warning is the unused `User` struct, which is being kept for now.

## Quick Verification Checklist

- `cargo check` passes.
- Login works for all three demo accounts.
- Dashboard loads after login.
- Invoice page shows paid and balance amounts.
- Partial and full payments update invoice status correctly.
- Journal and ledger reflect invoice/payment entries.
- Profit and Loss and Balance Sheet load successfully.
- Invoice PDF downloads successfully.
- Audit Trail shows admin activity.
