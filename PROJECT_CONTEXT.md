	# Accounting System Project Context

This file is the working guide for this repository. It exists to keep development aligned with the CSC1106 Web Programming project specification and the selected project domain: Accounting System.

Important: this is an internal planning and coordination guide only. Do not submit this file, or copy its wording into the final report or slides. The project specification says generative AI content must not be submitted as assessment material. Use this as a checklist and rewrite all submitted explanations in the team's own words.

## Project Direction

Build a Rust, Actix Web, Tera, and SQLite server-side rendered accounting system for small business finance workflows.

The core project should be an accounting system, not a banking system. The existing codebase currently has banking names and seed data, but it can be evolved into an accounting product by replacing bank-transfer concepts with accounting workflows:

- Customer management
- Invoice management
- Expense tracking
- Payment record management
- Double-entry journal posting
- Ledger, trial balance, profit and loss, and balance sheet reporting
- Audit trail and transaction history
- Optional PDF invoice export

The main technical centerpiece should be a double-entry accounting engine. Every posted financial event must create balanced debit and credit journal lines inside one database transaction.

## Assignment Goals To Satisfy

The project specification expects:

- A modern enterprise web application built with Rust and Actix Web.
- Clear OOP-style Rust design through structs, impl blocks, traits where useful, and modular architecture.
- Server-side rendering with Tera templates.
- Relational database integration.
- Realistic business workflows based on the selected enterprise domain.
- Technical depth beyond simple CRUD, especially validation, algorithms, transaction safety, scheduling, reporting, audit logging, or PDF generation.
- Clear documentation, presentation, and demonstration.
- Separately identifiable individual contributions and extended features.

## Current Repository Snapshot

Current app stack:

- Rust 2024
- Actix Web
- Actix Session with cookie session storage
- Tera templates
- SQLx with SQLite
- bcrypt password hashing
- Static CSS in `web-project/static/style.css`

Current files:

- `web-project/src/main.rs`: app state, session helpers, login, logout, dashboard, route registration.
- `web-project/src/db.rs`: schema creation and seed data for users, bank accounts, and transactions.
- `web-project/src/models.rs`: user, login, bank account, and transaction structs.
- `web-project/src/handlers.rs`: transaction listing handler.
- `web-project/templates/base.html`: shared layout and navigation.
- `web-project/templates/login.html`: login page.
- `web-project/templates/dashboard.html`: admin and customer dashboards.
- `web-project/templates/transactions.html`: global transaction history table.
- `web-project/static/style.css`: shared styling.
- `README.md`: currently minimal setup notes.

Current gaps to fix:

- The app is currently closer to a banking prototype than an accounting system.
- `transactions` are global and not connected to users, accounts, invoices, expenses, or ledger entries.
- Monetary amounts use `f64`; accounting amounts should use integer cents.
- There is no double-entry journal model yet.
- There are no customer, invoice, expense, payment, report, or audit modules.
- Branding still contains placeholders such as `LOGO`, `LOGO OR NAME`, and `CareSync`.
- README does not yet explain architecture, workflows, setup, seed users, or demo path.

## Target User Roles

Use role-based access consistently.

- Admin: manage users, chart of accounts, system settings, and view all records.
- Accountant: manage customers, invoices, expenses, payments, journal postings, and reports.
- Customer: optional portal role for viewing invoices and payment status.

If time is limited, implement Admin and Accountant first. A customer portal is useful but less important than the accounting engine and reporting workflow.

## Core Data Model

Prefer normalized tables and explicit relationships.

Minimum recommended tables:

- `users`: login identity, password hash, display name, email, role, created timestamp.
- `customers`: customer profile, contact details, billing address, status.
- `chart_accounts`: account code, name, type, normal balance, active flag.
- `journal_entries`: entry date, source type, source id, memo, posted_by, posted_at, status.
- `journal_lines`: journal entry id, account id, debit_cents, credit_cents, line memo.
- `invoices`: customer id, invoice number, issue date, due date, status, subtotal_cents, tax_cents, total_cents.
- `invoice_items`: invoice id, description, quantity, unit_price_cents, tax_rate, line_total_cents.
- `expenses`: vendor, expense date, category account id, description, amount_cents, tax_cents, status.
- `payments`: customer id, invoice id, payment date, amount_cents, method, reference.
- `audit_logs`: actor user id, action, entity type, entity id, timestamp, details.

Possible extra tables:

- `tax_rates`
- `attachments`
- `recurring_invoices`
- `payment_allocations`

Money rule: store all money as integer cents. Do not use `f64` for persisted accounting values.

## Double-Entry Accounting Rules

The accounting engine should enforce these invariants:

- A posted journal entry must have at least two lines.
- Total debits must equal total credits exactly.
- A line cannot contain both a debit and a credit.
- A line must contain either a debit or a credit greater than zero.
- Posted journal entries are immutable. Corrections should be reversal entries or adjustment entries.
- Posting must happen in a SQL transaction so the journal header, lines, source record status, and audit log commit or roll back together.
- Report balances should be derived from posted journal lines, not manually edited totals.

## Posting And Audit Safeguards

The posting service must also enforce the following rules:

- Each business event may only be posted once. Retrying or refreshing a posting request must not create duplicate journal entries.
- Journal entries should identify their source using `source_type`, `source_id`, and `entry_kind`, protected by an appropriate unique database constraint.
- Only active chart accounts may be used in new journal lines.
- Source-document status transitions must be explicitly defined and validated by Rust services.
- A reversal is a new balanced posted journal entry linked to the original entry. The original entry and its lines must never be deleted.
- Reversal records must include the reversing user, timestamp, and reason.
- Reports must retain both the original and reversal entries so their financial effects net to zero.
- Audit records are append-only and must not be editable or deletable through normal application routes.
- Audit records should store the actor, action, entity, previous values, new values, reason, timestamp, and correlation ID where appropriate.
- Financial calculations, including tax, must not use `f32` or `f64`. Tax rates should use an integer or fixed-precision representation and follow a documented rounding rule.
- Database constraints should enforce valid journal-line debit and credit combinations in addition to service-layer validation.

Typical postings:

- Invoice issued: debit Accounts Receivable, credit Sales Revenue, credit Tax Payable if tax applies.
- Customer payment received: debit Cash or Bank, credit Accounts Receivable.
- Expense paid immediately: debit Expense account, debit Input Tax if tracked, credit Cash or Bank.
- Expense recorded but unpaid: debit Expense account, credit Accounts Payable.

## Main Workflows

Implement these workflows before polishing optional features.

1. Authentication and authorization
   - Login/logout.
   - Role-specific navigation.
   - Protected routes.
   - Remove visible demo credentials before final submission.

2. Customer management
   - List, create, view, edit, and deactivate customers.
   - Show invoice and payment history for each customer.

3. Chart of accounts
   - Seed standard accounts such as Cash, Accounts Receivable, Revenue, Expenses, Tax Payable, Accounts Payable, and Equity.
   - Let admin/accountant view accounts and balances.

4. Invoice workflow
   - Create draft invoice with line items.
   - Validate totals and tax.
   - Post invoice into the journal.
   - Mark invoice as sent or paid.
   - Display invoice detail with its journal entry.

5. Expense workflow
   - Record business expenses.
   - Categorize expense by chart account.
   - Post expense into the journal.
   - Show expense history and totals by category.

6. Payment workflow
   - Record payment against invoice.
   - Prevent overpayment unless explicitly handled.
   - Post payment into the journal.
   - Update invoice status based on paid amount.

7. Reporting workflow
   - Trial balance.
   - Profit and loss statement.
   - Balance sheet.
   - Dashboard totals such as revenue, unpaid invoices, expenses, and cash balance.

8. Audit workflow
   - Log important create/update/post actions.
   - Show who posted each entry and when.
   - Provide enough history to explain data changes during the demo.

## Backend Architecture Guide

Keep modules small and business-focused.

Recommended structure:

- `main.rs`: application setup, shared state, middleware, route registration only.
- `models.rs`: data structs and form input structs.
- `db.rs`: connection setup, schema initialization, seed data, simple shared queries.
- `handlers/`: route handlers grouped by feature if the project grows.
- `services/accounting_engine.rs`: double-entry validation and posting logic.
- `services/reports.rs`: trial balance, profit and loss, and balance sheet calculations.
- `services/audit.rs`: audit log helper.

Use structs and impl blocks for domain concepts where they clarify behavior, for example:

- `Money`
- `JournalEntryDraft`
- `JournalLineDraft`
- `AccountingEngine`
- `InvoiceTotals`
- `ReportPeriod`

Avoid placing business rules directly inside Tera templates. Templates should display data; Rust should validate and compute.

## Frontend And SSR Guide

The frontend should feel like a practical enterprise accounting tool:

- Consistent header and navigation.
- Clear tables for invoices, customers, expenses, journals, and reports.
- Dashboard cards for important financial metrics.
- Forms with validation messages.
- Responsive layouts that work on laptop and mobile widths.
- No placeholder branding in final.
- No visible comments like "remove this later" in final UI.

Recommended pages:

- `/login`
- `/dashboard`
- `/customers`
- `/customers/new`
- `/customers/{id}`
- `/invoices`
- `/invoices/new`
- `/invoices/{id}`
- `/expenses`
- `/payments`
- `/journal`
- `/reports/trial-balance`
- `/reports/profit-loss`
- `/reports/balance-sheet`
- `/audit`

## Rubric Mapping

Use this section as the project checklist.

System Architecture and OOP Design, 15 percent:

- Separate handlers, models, database access, services, and templates.
- Use Rust structs and impl blocks for accounting concepts.
- Keep double-entry posting logic centralized.
- Avoid tightly coupled route handlers with duplicated SQL and validation.

Backend Functionality and Business Logic, 15 percent:

- Implement complete workflows for customers, invoices, expenses, payments, journal entries, and reports.
- Enforce double-entry balance checks.
- Add meaningful validation and error handling.
- Use role-based permissions.

Database Design and Integration, 10 percent:

- Normalize customer, invoice, payment, chart account, journal entry, journal line, and audit data.
- Use foreign keys and meaningful relationships.
- Store money as integer cents.
- Use SQL transactions for posting.

Frontend Design and SSR, 10 percent:

- Use Tera inheritance and reusable layout patterns.
- Keep pages responsive and visually consistent.
- Make forms and tables readable.
- Show clear success and error states.

Documentation, Presentation, and Demonstration, 10 percent:

- Expand README with setup, seed accounts, architecture, module list, and demo workflow.
- Prepare slides that explain the accounting domain, architecture, schema, and advanced features.
- Demo both ordinary workflows and technical safeguards.

Individual Extended Features and Technical Complexity, 40 percent:

- Each member should own a feature that can be independently explained and demonstrated.
- Contributions should show technical depth, not only styling.
- Each member should be able to explain design decisions, edge cases, and code paths.

## Suggested Individual Feature Ownership

Use these as slots and assign real names later.

- Member A: Double-entry accounting engine and journal posting validation.
- Member B: Invoice workflow, invoice totals, tax calculation, and optional PDF export.
- Member C: Financial reports such as trial balance, profit and loss, and balance sheet.
- Member D: Audit logging, role-based access, and dashboard analytics.

If the group has fewer members, combine features. If the group has more members, split customer management, payment allocation, and UI polish into separate ownership areas.

## Implementation Phases

Phase 1: Reframe the app

- Rename product text away from banking and CareSync placeholders.
- Decide final app name.
- Replace banking-specific wording with accounting wording.
- Update database name from `bank.db` to an accounting-oriented name if convenient.

Phase 2: Build accounting foundation

- Add chart of accounts.
- Add journal entries and journal lines.
- Add money-as-cents models.
- Implement accounting engine validation.
- Seed sample accounts and sample business data.

Phase 3: Build business modules

- Customer CRUD.
- Invoice creation and posting.
- Expense recording and posting.
- Payment recording and posting.

Phase 4: Build reports and audit

- Trial balance.
- Profit and loss.
- Balance sheet.
- Audit log list and detail views.

Phase 5: Polish for assessment

- Improve README.
- Remove placeholder text and demo-password hints from UI.
- Check responsive layouts.
- Prepare demo data.
- Prepare slides and report in the team's own wording.
- Verify each member can explain their feature.

## Demo Script Target

A strong 15 minute demo can follow this path:

1. Show login and role-based dashboard.
2. Show customer list and create a customer.
3. Create an invoice with line items and tax.
4. Post the invoice and show the balanced journal entry.
5. Record a customer payment and show invoice status update.
6. Record an expense and show the generated journal entry.
7. Open trial balance and prove debits equal credits.
8. Open profit and loss or balance sheet.
9. Show audit log entries for the actions.
10. Each member explains their individual advanced feature and relevant code/module.

## Engineering Guardrails

- Prefer SQLx queries with bound parameters.
- Avoid `unwrap()` in request handlers once the feature is final; return a useful error page or message.
- Keep session checks consistent across protected routes.
- Use database transactions for multi-step writes.
- Validate form input on the server.
- Avoid duplicate session helper logic across modules.
- Keep seed data realistic but small.
- Do not commit large generated artifacts unless required for submission.

## Definition Of Done

The project is ready when:

- A user can complete the main accounting workflows without manual database edits.
- Posted journal entries are always balanced.
- Reports are calculated from journal data.
- The UI has no placeholder branding or unfinished notes.
- README explains setup and demo workflow clearly.
- The database schema supports meaningful relationships.
- Each individual extended feature is visible in the app and explainable from the code.
- The final submitted report and slides are written by the team, not copied from this guide.
