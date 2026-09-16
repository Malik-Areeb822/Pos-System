// src-tauri/src/services/invoice_pdf.rs
use tauri::AppHandle;
use crate::DbPool;
use crate::error::AppError;
use crate::repositories::InvoiceRepository;
use genpdf::{Document, elements, fonts};
use std::fs::File;

const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf");

/// Fonts are embedded at compile time so PDF generation never depends on the
/// working directory or an external ./fonts folder.
fn load_font_family() -> Result<fonts::FontFamily<fonts::FontData>, AppError> {
    let regular = fonts::FontData::new(FONT_REGULAR.to_vec(), None)
        .map_err(|e| AppError::Pdf(format!("Font load failed: {}", e)))?;
    let bold = fonts::FontData::new(FONT_BOLD.to_vec(), None)
        .map_err(|e| AppError::Pdf(format!("Font load failed: {}", e)))?;
    // Italic slots reuse the same faces; invoice content only uses plain paragraphs.
    let italic = regular.clone();
    let bold_italic = bold.clone();
    Ok(fonts::FontFamily { regular, bold, italic, bold_italic })
}

pub async fn generate_invoice_pdf(_app: &AppHandle, pool: &DbPool, invoice_id: &str) -> Result<String, AppError> {
    let repo = InvoiceRepository::new(pool.clone());
    let invoice = repo.get_with_items(invoice_id).await?
        .ok_or(AppError::NotFound("Invoice not found".into()))?;

    let (inv, items) = invoice;

    // Create PDF document
    let font_family = load_font_family()?;

    let mut doc = Document::new(font_family);
    doc.set_title(&format!("Invoice {}", inv.invoice_no));

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(20);
    doc.set_page_decorator(decorator);

    // Build content directly on document
    build_invoice_content(&mut doc, &inv, &items);

    // Render to file
    let config = crate::config::CONFIG.clone();
    std::fs::create_dir_all(&config.app_data_dir)?;
    let output_path = config.app_data_dir.join(format!("invoice_{}.pdf", sanitize_filename(&inv.invoice_no)));

    let mut file = File::create(&output_path)?;
    doc.render(&mut file)
        .map_err(|e| AppError::Pdf(format!("PDF render failed: {}", e)))?;

    Ok(output_path.to_string_lossy().to_string())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

fn build_invoice_content(doc: &mut Document, inv: &crate::repositories::invoices::Invoice, items: &[crate::repositories::invoices::InvoiceItem]) {
    // Header
    doc.push(elements::Paragraph::new("CITY TILES POS"));
    doc.push(elements::Break::new(1));
    doc.push(elements::Paragraph::new("TAX INVOICE"));
    doc.push(elements::Break::new(1));
    doc.push(elements::Paragraph::new("----------------------------------------"));
    doc.push(elements::Break::new(1));
    
    // Invoice details
    doc.push(elements::Paragraph::new(format!("Invoice No: {}", inv.invoice_no)));
    let date_str = chrono::DateTime::parse_from_rfc3339(&inv.created_at)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %I:%M %p").to_string())
        .unwrap_or_else(|_| inv.created_at.split('T').next().unwrap_or("").to_string());
    doc.push(elements::Paragraph::new(format!("Date: {}", date_str)));
    doc.push(elements::Paragraph::new(format!("Customer: {}", inv.customer_name)));
    doc.push(elements::Break::new(1));
    doc.push(elements::Paragraph::new("----------------------------------------"));
    doc.push(elements::Break::new(1));
    
    // Items
    doc.push(elements::Paragraph::new("Items:"));
    for item in items {
        let mut item_text = format!("{} x {} {} @ {} = {}", 
            item.quantity, item.unit, item.product_name, 
            format_price(item.unit_price), format_price(item.line_total));
        if let Some(total_area) = item.total_area {
            if total_area > 0.0 {
                item_text.push_str(&format!(" [{:.3} sqm]", total_area));
            }
        }
        doc.push(elements::Paragraph::new(item_text));
    }
    
    doc.push(elements::Break::new(1));
    doc.push(elements::Paragraph::new("----------------------------------------"));
    doc.push(elements::Break::new(1));
    
    // Totals
    doc.push(elements::Paragraph::new(format!("Subtotal: {}", format_price(inv.subtotal))));
    if inv.discount > 0 {
        doc.push(elements::Paragraph::new(format!("Discount: -{}", format_price(inv.discount))));
    }
    doc.push(elements::Paragraph::new(format!("TOTAL: {}", format_price(inv.total))));
    doc.push(elements::Paragraph::new(format!("Paid: {}", format_price(inv.amount_paid))));
    if inv.amount_paid < inv.total {
        doc.push(elements::Paragraph::new(format!("Balance: {}", format_price(inv.total - inv.amount_paid))));
    }
    doc.push(elements::Paragraph::new(format!("Payment Method: {}", inv.payment_method)));
    
    if let Some(notes) = &inv.notes {
        if !notes.is_empty() {
            doc.push(elements::Break::new(1));
            doc.push(elements::Paragraph::new("Notes:"));
            doc.push(elements::Paragraph::new(notes));
        }
    }
}

/// Amounts are stored and displayed as whole rupees everywhere in the app
/// (see HANDOFF.md locked decisions) — no paise conversion here.
fn format_price(rupees: i64) -> String {
    format!("PKR {}", rupees)
}