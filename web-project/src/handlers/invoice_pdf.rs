use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, web};
use tera::Context;

use crate::{
    AppState, helpers::add_user_to_ctx, invoice_pdf::InvoicePdfData,
    pdf_generator::generate_invoice_pdf,
};

// Generates and downloads a PDF copy of one saved invoice.
#[get("/invoices/{id}/pdf")]
pub async fn download_invoice_pdf(
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

    let invoice_id = path.into_inner();

    let invoice: Option<InvoicePdfData> = sqlx::query_as(
        "SELECT
            i.invoice_no,
            c.name AS customer_name,
            c.email AS customer_email,
            c.phone AS customer_phone,
            i.invoice_date,
            i.due_date,
            i.description,
            i.amount_cents,
            i.status
         FROM invoices i
         JOIN customers c ON c.id = i.customer_id
         WHERE i.id = ?1",
    )
    .bind(invoice_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let Some(invoice) = invoice else {
        return HttpResponse::Found()
            .append_header(("Location", "/invoices"))
            .finish();
    };

    let filename = format!("{}.pdf", invoice.invoice_no);

    // Run the blocking Python process outside Actix's async worker.
    match web::block(move || generate_invoice_pdf(&invoice)).await {
        Ok(Ok(pdf_bytes)) => HttpResponse::Ok()
            .content_type("application/pdf")
            .append_header((
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ))
            .append_header(("Cache-Control", "no-store"))
            .body(pdf_bytes),
        Ok(Err(error)) => {
            eprintln!("Invoice PDF generation failed: {error}");
            HttpResponse::InternalServerError()
                .content_type("text/plain; charset=utf-8")
                .body("Could not generate invoice PDF.")
        }
        Err(error) => {
            eprintln!("Invoice PDF generation task failed: {error}");
            HttpResponse::InternalServerError()
                .content_type("text/plain; charset=utf-8")
                .body("Could not generate invoice PDF.")
        }
    }
}
