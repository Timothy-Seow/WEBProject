# Development Log

Short notes for tracking project changes, feature ownership, and next steps. Keep entries brief and write them in your own words if reused for presentation or report preparation.

## 2026-06-18 - Chart of Accounts

### Added
- Added `chart_accounts` database table in `web-project/src/db.rs`.
- Seeded default accounting accounts such as Cash, Accounts Receivable, Accounts Payable, Sales Revenue, and Expenses.
- Added `ChartAccount` model in `web-project/src/models/chart_account.rs`.
- Added `/accounts` route handler in `web-project/src/handlers/chart_accounts.rs`.
- Added `accounts.html` template to display the chart of accounts.
- Registered the chart accounts model and handler in `web-project/src/main.rs`.
- Added `Accounts` link to the navbar.
- Updated navbar user display to use session role and session name.
- Linked navbar logout control to `/logout`.

### Why
- Provides the accounting foundation needed for double-entry journal entries, invoices, payments, expenses, and financial reports.
- Helps move the project from a banking-style prototype toward the required accounting system.

### Verified
- `cargo check` passes.
- Remaining warning is unrelated: `User` struct is currently unused.
- Manually tested `/accounts` list page.
- Manually tested navbar link to `/accounts`.
- Manually tested logout/session redirect behavior.

### Next
- Improve `/accounts` table styling.
- Add edit/deactivate account forms if time allows.
- Connect future journal entries to `chart_accounts`.

## 2026-06-18 - Create Chart Account Form

### Added
- Added `/accounts/new` page for creating chart accounts.
- Added `POST /accounts` form submission.
- Added duplicate-code error handling.
- Added link from `/accounts` to the new account form.

### Why
- Completes the first basic create workflow for chart accounts.
- Allows custom accounts to be added for future journal entries and reports.

### Verified
- `cargo check` passes.
- Manually tested creating a valid chart account.
- Manually tested duplicate account code handling.

### Next
- Add edit/deactivate account workflow.

## 2026-06-18 - Chart Account Validation

### Added
- Added server-side validation for chart account code ranges.
- Enforced account type based on the first digit of the account code.
- Enforced normal balance based on account type.
- Added a small form hint for valid account code ranges.

### Why
- Prevents invalid setup such as using an Asset code for an Expense account.
- Makes the chart of accounts follow common accounting conventions.
- Adds business rule validation for a stronger accounting workflow.

### Verified
- `cargo check` passes.
- Manually tested invalid code/type validation.
- Manually tested invalid normal balance validation.

### Next
- Improve account form styling.
- Add edit/deactivate account workflow.

## 2026-06-18 - Chart Account UI Styling

### Added
- Styled `/accounts` with a clearer page header, action button, and reusable table classes.
- Styled `/accounts/new` with a form card, grouped inputs, action buttons, and reusable alert classes.
- Added shared table, form, button, and alert styles in `web-project/static/style.css`.

### Why
- Makes the Chart of Accounts workflow easier to read and demo.
- Keeps the accounting pages visually consistent with the rest of the app.

### Verified
- `cargo check` passes.
- Manually tested `/accounts`.
- Manually tested `/accounts/new`.

### Next
- Add edit/deactivate account workflow.
- Connect future journal entries to `chart_accounts`.

## 2026-06-18 - Edit And Deactivate Chart Accounts

### Added
- Added `/accounts/{id}/edit` page for editing chart accounts.
- Added `POST /accounts/{id}/edit` to save chart account updates.
- Added `POST /accounts/{id}/deactivate` to mark chart accounts inactive.
- Added edit and deactivate actions to the `/accounts` table.
- Added `accounts_edit.html` template for the edit form.
- Added status badge and row action styles in `web-project/static/style.css`.

### Why
- Completes more of the chart account management workflow.
- Allows account details to be corrected while keeping code/type validation rules.
- Keeps old accounts available for history by deactivating instead of deleting.

### Verified
- `cargo check` passes.
- `cargo run` starts successfully after approval to run the local server.
- Manually tested editing a chart account.
- Manually tested invalid edit validation.
- Manually tested duplicate account code handling during edit.
- Manually tested deactivating an account.

### Next
- Connect future journal entries to `chart_accounts`.

## 2026-06-19 - Journal Entry Listing

### Added
- Added `journal_entries` and `journal_lines` database tables.
- Added `JournalEntry`, `JournalLine`, and `JournalLineView` models.
- Seeded a balanced opening journal entry: debit Cash and credit Owner Equity.
- Added `/journal` route handler to list journal entries and lines.
- Added `journal.html` template to display journal entry rows.
- Added `Journal` link to the navbar.

### Why
- Starts the double-entry accounting workflow required by the project specification.
- Connects journal lines to the existing chart of accounts.
- Demonstrates balanced debit and credit records using integer cents.

### Verified
- `cargo check` passes.
- `cargo run` starts successfully after approval to run the local server.
- Manually tested `/journal` in the browser.
- Manually tested debit and credit totals on `/journal`.
- Manually confirmed the journal status shows Balanced.

### Next
- Later, add a form for creating journal entries.

## 2026-06-19 - Create Journal Entry Form

### Added
- Added `/journal/new` page for creating manual journal entries.
- Added `POST /journal` form submission for one debit line and one credit line.
- Added active chart account dropdowns for debit and credit accounts.
- Added validation to prevent zero/negative amounts.
- Added validation to prevent using the same account for both debit and credit.
- Wrapped journal entry creation in a database transaction.

### Why
- Supports the core double-entry accounting workflow.
- Ensures each manual journal entry creates equal debit and credit lines.
- Prevents partially saved journal entries if one line fails.

### Verified
- `cargo check` passes.
- Manually tested creating a journal entry.
- Manually tested validation for matching debit and credit accounts.
- Confirmed journal totals remain balanced after creation.

### Next
- Improve amount input so users can type dollars instead of cents.
- Add per-entry debit and credit subtotals.
- Later, connect invoices and expenses to automatic journal posting.

## 2026-06-20 - Dollar Amount Input And Code Comments

### Added
- Updated the journal form to accept dollar amounts such as `25.50`.
- Added exact dollar-to-cents validation without floating-point calculations.
- Added short comments to the chart-account and journal models, handlers, routes, and database setup.

### Why
- Makes manual journal entries easier to enter correctly.
- Helps group members quickly understand the purpose of the accounting code.

### Verified
- `cargo check` passes.

### Next
- Manually test valid and invalid dollar amounts on `/journal/new`.
- Add per-entry debit and credit subtotals.

## 2026-06-20 - Per-Entry Journal Totals

### Added
- Added debit and credit totals for each individual journal entry.
- Added a per-entry `Balanced` or `Out of Balance` status.
- Added a highlighted total row below each journal entry's lines.

### Why
- Verifies every journal entry independently, not only the journal's overall total.
- Prepares the journal for future entries with more than two debit or credit lines.

### Verified
- `cargo check` passes.
- Manually confirmed per-entry totals and balance status in the browser.

### Next
- Add a general ledger page showing the activity and balance of each chart account.

## 2026-06-20 - General Ledger

### Added
- Added a `/ledger` page that lists debit totals, credit totals, and balances for every chart account.
- Calculated balances using each account's normal Debit or Credit side.
- Kept accounts with no journal activity visible with zero totals.
- Added the Ledger link to the shared navigation bar.

### Why
- Lets users review the current accounting position of each account in one place.
- Builds on journal entries to provide a basic accounting report.

### Verified
- `cargo check` passes.
- Manually confirmed the Ledger page opens and displays account balances.

### Next
- Add a transaction history view for one selected ledger account.

## 2026-06-20 - Ledger Account History

### Added
- Made account names on the General Ledger page clickable.
- Added `/ledger/{id}` to show the journal history for one selected account.
- Added a detail page with the account type, normal balance, and related debit and credit lines.
- Added a safe redirect back to `/ledger` when an account ID does not exist.

### Why
- Lets users trace a ledger balance back to the journal entries that created it.
- Makes the double-entry records easier to review and explain.

### Verified
- `cargo check` passes.
- Manually confirmed an account link opens the correct history page and the back link returns to the General Ledger.

### Next
- Add a running balance column to the individual account-history page.

## 2026-06-20 - Ledger Running Balance

### Added
- Added a running balance column to each account's journal-history page.
- Calculated each row's balance according to the account's normal Debit or Credit side.
- Displayed the balance side when activity moves an account to its opposite side.

### Why
- Lets users see how every journal entry changes an account over time.
- Makes account histories easier to verify without manually adding each row.

### Verified
- `cargo check` passes.
- Manually confirmed running balances on the account-history page.

### Next
- Add a basic profit and loss report using Revenue and Expense account balances.

## 2026-06-20 - Profit And Loss Report

### Added
- Added an all-time Profit and Loss report at `/reports/profit-loss`.
- Added separate Revenue and Expense account sections with calculated totals.
- Added Net Profit or Net Loss based on Revenue minus Expenses.
- Added a Profit & Loss link to the shared navigation bar.

### Why
- Provides a core accounting report using the journal and chart-of-accounts data.
- Shows how Revenue and Expense accounts affect the business result.

### Verified
- `cargo check` passes.
- Manually confirmed Revenue, Expenses, and Net Profit display correctly in the browser.

### Next
- Add an all-time Balance Sheet using Asset, Liability, and Equity account balances.

## 2026-06-20 - Balance Sheet Report

### Added
- Added an all-time Balance Sheet report at `/reports/balance-sheet`.
- Added Asset, Liability, and Equity account sections with calculated totals.
- Included current Profit or Loss in Total Equity so the accounting equation remains balanced.
- Added a Balance Sheet link to the shared navigation bar.

### Why
- Completes a second core financial report alongside Profit and Loss.
- Verifies the accounting equation: Assets = Liabilities + Equity.

### Verified
- `cargo check` passes.
- Manually confirmed the Balance Sheet displays matching Assets and Liabilities plus Equity totals.

### Next
- Build an expense-recording workflow that automatically creates a balanced journal entry.

## 2026-06-20 - Expense Workflow

### Added
- Added an `expenses` database table linked to chart accounts and journal entries.
- Added an Expenses list page and a new-expense form.
- Added server-side validation for amount, active accounts, and account types.
- Automatically creates one debit Expense line and one credit payment line for every saved expense.
- Wrapped expense and journal creation in one database transaction.
- Added an Expenses link to the shared navigation bar.

### Why
- Provides a real business workflow rather than requiring every expense to be entered manually as a journal entry.
- Keeps expense records, the General Ledger, Profit and Loss, and Balance Sheet consistent.

### Verified
- `cargo check` passes.
- Confirmed the `expenses` table exists in SQLite.
- Manually confirmed an expense creates the expected expense row and balanced journal entry.
- Manually confirmed accounting reports update after recording an expense.

### Next
- Build an invoice or customer-payment workflow that automatically posts Revenue and Cash/Receivable entries.

## 2026-06-21 - Customer Workflow

### Added
- Added `customers` and `invoices` database tables for future sales workflows.
- Added customer list and new-customer pages.
- Added server-side validation for customer names.
- Added Customers navigation.

### Why
- Establishes customer records before invoices and payments are created.
- Provides the relationship needed for one customer to have many invoices.

### Verified
- `cargo check` passes.
- Confirmed the customer and invoice tables exist in SQLite.
- Manually confirmed creating a customer and displaying it in the customer list.

### Next
- Build invoice creation that posts Debit Accounts Receivable and Credit Sales Revenue automatically.

## 2026-06-21 - Invoice Workflow

### Added
- Added invoice list and create pages linked to customers.
- Added automatic Debit Accounts Receivable and Credit Sales Revenue journal posting.
- Added invoice status tracking with new invoices marked unpaid.
- Added Invoices navigation.

### Verified
- `cargo check` passes.
- Manually confirmed invoice creation updates Journal, Ledger, Profit and Loss, and Balance Sheet.

### Next
- Add customer payment recording to settle unpaid invoices.

## 2026-06-21 - Customer Payment Workflow

### Added
- Added customer payment storage linked to invoices and journal entries.
- Added a Receive Payment form for unpaid invoices.
- Automatically posts Debit Cash and Credit Accounts Receivable.
- Marks a fully paid invoice as `paid`.

### Verified
- `cargo check` passes.
- Manually confirmed payment posting and invoice status update.

### Next
- Add partial-payment support or date-range filters for reports.

## 2026-06-22 - Profit And Loss Date Filter

### Added
- Added optional From and To date filters to Profit and Loss.
- Filtered Revenue and Expense journal activity by journal-entry date.
- Added clear and filter controls while preserving selected dates.

### Verified
- `cargo check` passes.
- Manually confirmed Profit and Loss totals change for a selected date range.

### Next
- Reuse the date-filter pattern for Journal, Ledger, Expenses, and invoice reporting.

## 2026-06-22 - Journal Date Filter

### Added
- Added From and To date filtering to the Journal page.
- Filtered journal headers, lines, totals, and balance status consistently.
- Added filter and clear controls while preserving selected dates.

### Verified
- `cargo check` passes.
- Manually confirmed Journal entries and totals change for a selected date range.

### Next
- Add date filters to Ledger account history or Expenses.

## 2026-06-22 - Expense Date Filter

### Added
- Added From and To date filtering to the Expenses page.
- Preserved selected dates and added a Clear action.

### Verified
- `cargo check` passes.
- Manually confirmed expense records filter by selected date range.

### Next
- Add date filters to Ledger account history or invoice reporting.

## 2026-06-22 - Ledger History Date Filter

### Added
- Added From and To date filtering to individual Ledger account histories.
- Preserved selected dates and added a Clear action.

### Verified
- `cargo check` passes.
- Manually confirmed selected account activity filters by date range.

### Next
- Add invoice date filtering or improve Ledger filters with opening balances.

## 2026-06-22 - Ledger Filter Opening Balance

### Added
- Added an opening-balance row for filtered Ledger account histories.
- Calculates activity before the selected start date and carries it into the running balance.

### Verified
- `cargo check` passes.
- Manually confirmed filtered Ledger running balances include opening activity.

### Next
- Add invoice date filtering or partial-payment support.

## 2026-06-22 - Invoice Date Filter

### Added
- Added From and To date filtering to the Invoices page.
- Preserved selected dates and added a Clear action.

### Verified
- `cargo check` passes.
- Manually confirmed invoices filter by invoice date range.

### Next
- Add partial-payment support or a dashboard accounting summary.

## 2026-06-22 - Accounting Dashboard Summary

### Added
- Added admin dashboard metrics for Cash, receivables, unpaid invoices, monthly revenue, and monthly expenses.
- Calculated values from journal entries, invoices, and the chart of accounts.

### Verified
- `cargo check` passes.
- Manually confirmed all five accounting summary metrics display for admin users.

### Next
- Add partial-payment support or consolidate shared helper functions.

## 2026-06-23 - Role Permissions And Role-Aware UI

### Added
- Added `admin`, `accountant`, and `viewer` roles with demo login accounts for permission testing.
- Restricted Chart of Accounts changes to admins and accounting record creation to admins and accountants.
- Hid unavailable navigation links and create or account-management actions in the server-rendered UI.

### Verified
- `cargo check` passes.
- Viewer-facing journal, invoice, expense, customer, and account pages remain readable without creation actions.

### Next
- Add an audit trail that records who created important accounting records and when.

## 2026-06-23 - Audit Trail Foundation

### Added
- Added the `audit_logs` table and `AuditLog` model for append-only action history.
- Added a shared transaction-safe audit writer that records the acting user, action, affected record, details, and timestamp.
- Recorded an audit event whenever a manual journal entry is successfully created.

### Verified
- `cargo check` passes.
- Manually created a journal entry as an accountant and confirmed its audit row in SQLite.

### Next
- Add audit events to expenses, invoices, payments, customers, and Chart of Accounts changes.

## 2026-06-23 - Expense Audit Event

### Added
- Recorded a `created` audit event when an expense and its automatic journal entry are saved.
- Linked the audit event to the new expense record within the same database transaction.

### Verified
- `cargo check` passes.
- Confirmed the expense audit code records the acting user, expense ID, description, and timestamp before commit.

### Next
- Add an audit event when an invoice is created.

## 2026-06-23 - Invoice Audit Event

### Added
- Recorded a `created` audit event when an invoice and its automatic journal entry are saved.
- Linked the audit event to the newly created invoice within the same database transaction.

### Verified
- `cargo check` passes.
- Confirmed invoice creation records an audit row with the invoice number and description.

### Next
- Add an audit event when a customer payment settles an invoice.

## 2026-06-23 - Customer Payment Audit Event

### Added
- Recorded a `created` audit event when a customer payment settles an invoice.
- Saved the payment, invoice paid-status change, journal entry, and audit event in one transaction.

### Verified
- `cargo check` passes.
- Confirmed successful payment processing records a payment audit row with its invoice number.

### Next
- Add an audit event when a customer is created.

## 2026-06-24 - Customer Audit Event

### Added
- Moved customer creation into a database transaction.
- Recorded a `created` audit event linked to the newly created customer.

### Verified
- `cargo fmt` and `cargo check` pass.
- Confirmed the customer audit insert occurs before transaction commit.

### Next
- Add audit events for Chart of Accounts creation, edits, and deactivation.

## 2026-06-24 - Chart Account Creation Audit Event

### Added
- Moved Chart of Accounts creation into a database transaction.
- Recorded a `created` audit event with the new account ID, code, and name.

### Verified
- `cargo check` passes.
- Confirmed admin account creation records an audit row before commit.

### Next
- Add an audit event when an existing Chart of Accounts record is edited.

## 2026-06-24 - Chart Account Edit Audit Event

### Added
- Moved Chart of Accounts edits into a database transaction.
- Recorded an `updated` audit event with the resulting account code and name.

### Verified
- `cargo check` passes.
- Confirmed successful account edits record an audit row before commit.

### Next
- Add an audit event when an account is deactivated.

## 2026-06-24 - Chart Account Deactivation Audit Event

### Added
- Moved account deactivation into a database transaction.
- Recorded a `deactivated` audit event with the affected account code and name.

### Verified
- `cargo check` passes.
- Confirmed deactivation records an audit event before the account becomes inactive.

### Next
- Build an admin-only Audit Trail page to review recorded actions.

## 2026-06-24 - Admin Audit Trail Page

### Added
- Added an admin-only Audit Trail route and server-rendered table of recent audit events.
- Added conditional navigation so only admins see the Audit Trail link.
- Protected the route itself so non-admin users are redirected to the dashboard.

### Verified
- `cargo check` passes.
- Confirmed audit events display for admins and remain unavailable to accountant and viewer roles.

### Next
- Review remaining accounting requirements, test edge cases, and prepare presentation-ready data.

## 2026-06-24 - Monthly Balance Sheet

### Added
- Added a month selector that generates the Balance Sheet as at the selected month end.
- Filtered journal balances to include only activity before the following month.
- Prevented future-month selection in both the browser control and the server-side query handling.

### Verified
- `cargo check` passes.
- Manually confirmed the selected month changes the report and future months are capped at the current month.

### Next
- Build invoice PDF export or complete final workflow and report testing.

## 2026-06-24 - Invoice PDF Export

### Added
- Added a protected invoice PDF download route backed by a ReportLab generator.
- Added PDF data modelling, a configurable `PDF_PYTHON` executable, and Download PDF actions on invoice rows.
- Added setup instructions and a Python dependency requirement for the development container.

### Verified
- `cargo check` passes.
- Python generator syntax passes and its rendered output was visually reviewed.
- Manually confirmed invoice PDFs download successfully through the web application.

### Next
- Complete final workflow testing and prepare presentation-ready sample data.

## 2026-06-25 - Partial Customer Payments

### Added
- Allowed multiple payments per invoice by removing the old one-payment-per-invoice rule.
- Added payment amount input so invoices can be partly paid before becoming fully paid.
- Added invoice list columns for paid amount and balance due.

### Verified
- `cargo check` passes.
- Started the app successfully and confirmed the payment schema migration initializes.

### Next
- Test partial payment behavior in the browser with one invoice paid in two separate payments.

## 2026-06-25 - Demo Database Cleanup

### Added
- Backed up the existing database before cleanup.
- Reset the database to standard roles, standard chart accounts, and one opening balance entry.
- Removed old test customers, invoices, payments, expenses, audit rows, and legacy banking data.

### Verified
- `cargo check` passes.
- Confirmed the database now has clean demo-ready starting data.

### Next
- Create polished demo records for customer, invoice, partial payment, reports, PDF export, and audit trail.

## 2026-06-26 - Demo Workflow Test

### Verified
- `cargo check` passes.
- Confirmed the clean demo data has one customer, one invoice, and two payments.
- Confirmed the invoice moved from partial payment to paid after both payments.
- Confirmed journal entries and audit trail records were created for the demo workflow.

### Next
- Test role permissions and prepare a short setup/demo guide for groupmates.

## 2026-06-26 - Setup and Demo Guide

### Added
- Created a setup and demo guide for running the app, PDF setup, demo accounts, role permissions, and presentation flow.

### Verified
- `cargo check` passes.
- Reviewed the guide contents for groupmate handoff.

### Next
- Do final browser walkthrough and polish any confusing page text before presentation.
