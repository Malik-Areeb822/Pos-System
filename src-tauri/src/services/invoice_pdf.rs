// src-tauri/src/services/invoice_pdf.rs
use tauri::AppHandle;
use crate::DbPool;
use crate::error::AppError;
use crate::repositories::InvoiceRepository;
use genpdf::{Document, elements, fonts};
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

pub async fn generate_invoice_pdf(app: &AppHandle, pool: &DbPool, invoice_id: &str) -> Result<String, AppError> {
    let repo = InvoiceRepository::new(pool.clone());
    let invoice = repo.get_with_items(invoice_id).await?
        .ok_or(AppError::NotFound("Invoice not found".into()))?;
    
    let (inv, items) = invoice;
    
    // Create PDF document
    let font_family = fonts::from_files("./fonts", "DejaVuSans", None)
        .map_err(|e| AppError::Pdf(format!("Font load failed: {}", e)))?;
    
    let mut doc = Document::new(font_family);
    doc.set_title(&format!("Invoice {}", inv.invoice_no));
    
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(20);
    doc.set_page_decorator(decorator);
    
    // Build content directly on document
    build_invoice_content(&mut doc, &inv, &items);
    
    // Render to file
    let config = crate::config::CONFIG.clone();
    let output_path = config.app_data_dir.join(format!("invoice_{}.pdf", inv.invoice_no));
    
    let mut file = File::create(&output_path)?;
    doc.render(&mut file)
        .map_err(|e| AppError::Pdf(format!("PDF render failed: {}", e)))?;
    
    Ok(output_path.to_string_lossy().to_string())
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
    doc.push(elements::Paragraph::new(format!("Date: {}", inv.created_at.split('T').next().unwrap_or(""))));
    doc.push(elements::Paragraph::new(format!("Customer: {}", inv.customer_name)));
    doc.push(elements::Break::new(1));
    doc.push(elements::Paragraph::new("----------------------------------------"));
    doc.push(elements::Break::new(1));
    
    // Items
    doc.push(elements::Paragraph::new("Items:"));
    for item in items {
        let item_text = format!("{} x {} {} @ {} = {}", 
            item.quantity, item.unit, item.product_name, 
            format_price(item.unit_price), format_price(item.line_total));
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

fn format_price(paise: i64) -> String {
    let rupees = paise / 100;
    let paise_rem = paise % 100;
    format!("PKR {}.{:02}", rupees, paise_rem)
}