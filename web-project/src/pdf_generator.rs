use std::io::Write;
use std::process::{Command, Stdio};

use crate::invoice_pdf::InvoicePdfData;

// Sends one invoice to ReportLab and returns the generated PDF bytes.
pub(crate) fn generate_invoice_pdf(invoice: &InvoicePdfData) -> Result<Vec<u8>, String> {
    let input = serde_json::to_vec(invoice)
        .map_err(|_| "Could not prepare invoice PDF data.".to_string())?;

    let script_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/scripts/generate_invoice_pdf.py"
    );
    let python_command = std::env::var("PDF_PYTHON").unwrap_or_else(|_| "python3".to_string());

    let mut child = Command::new(python_command)
        .arg(script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "Could not start PDF generator with '{}': {error}",
                std::env::var("PDF_PYTHON").unwrap_or_else(|_| "python3".to_string())
            )
        })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Could not send invoice data to the PDF generator.".to_string())?;

    stdin
        .write_all(&input)
        .map_err(|_| "Could not send invoice data to the PDF generator.".to_string())?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .map_err(|_| "Could not read the generated PDF.".to_string())?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("PDF generation failed: {}", error.trim()));
    }

    if !output.stdout.starts_with(b"%PDF-") {
        return Err("PDF generator returned an invalid file.".to_string());
    }

    Ok(output.stdout)
}
