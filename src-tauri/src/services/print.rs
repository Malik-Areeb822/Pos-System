// src-tauri/src/services/print.rs
use tauri::AppHandle;
use crate::DbPool;
use crate::error::AppError;
use crate::repositories::InvoiceRepository;
use std::process::Command;
use rusb::UsbContext;
use std::time::Duration;

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;

pub async fn print_receipt(app: &AppHandle, pool: &DbPool, invoice_id: &str) -> Result<(), AppError> {
    let repo = InvoiceRepository::new(pool.clone());
    let invoice = repo.get_with_items(invoice_id).await?
        .ok_or(AppError::NotFound("Invoice not found".into()))?;
    
    let (inv, items) = invoice;
    
    // Try ESC/POS via USB first
    if let Err(e) = print_escpos_usb(&inv, &items).await {
        tracing::warn!("ESC/POS USB print failed, falling back to spooler: {}", e);
        print_via_spooler(&inv, &items).await?;
    }
    
    Ok(())
}

async fn print_escpos_usb(inv: &crate::repositories::invoices::Invoice, items: &[crate::repositories::invoices::InvoiceItem]) -> Result<(), AppError> {
    use rusb::{Context, Direction, TransferType};

    // Accept known thermal-printer VID/PIDs (see is_known_printer below),
    // plus any device claiming the standard USB printer class (0x07) —
    // covers budget 80mm/58mm models not on the list.
    let ctx = Context::new()?;
    let devices = ctx.devices()?;
    let data = build_escpos_receipt(inv, items);

    for device in devices.iter() {
        let desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };
        let is_printer = desc.class_code() == 0x07
            || is_known_printer(desc.vendor_id(), desc.product_id());
        if !is_printer {
            continue;
        }

        let mut handle = match device.open() {
            Ok(h) => h,
            Err(_) => continue,
        };
        if handle.kernel_driver_active(0).unwrap_or(false) {
            let _ = handle.detach_kernel_driver(0);
        }

        // Discover the real bulk OUT endpoint instead of assuming 0x01.
        let mut target: Option<(u8, u8)> = None; // (interface_number, endpoint_address)
        if let Ok(cfg) = device.config_descriptor(0) {
            'outer: for iface in cfg.interfaces() {
                for setting in iface.descriptors() {
                    for ep in setting.endpoint_descriptors() {
                        if ep.transfer_type() == TransferType::Bulk
                            && ep.direction() == Direction::Out
                        {
                            target = Some((setting.interface_number(), ep.address()));
                            break 'outer;
                        }
                    }
                }
            }
        }
        let Some((interface_num, endpoint_addr)) = target else {
            continue;
        };

        if handle.claim_interface(interface_num).is_err() {
            continue;
        }

        match handle.write_bulk(endpoint_addr, &data, Duration::from_secs(5)) {
            Ok(_) => {
                let _ = handle.release_interface(interface_num);
                return Ok(());
            }
            Err(e) => {
                let _ = handle.release_interface(interface_num);
                tracing::warn!(
                    "ESC/POS write failed on {:04x}:{:04x}: {}",
                    desc.vendor_id(),
                    desc.product_id(),
                    e
                );
            }
        }
    }

    Err(AppError::Usb(rusb::Error::NotFound))
}

async fn print_via_spooler(inv: &crate::repositories::invoices::Invoice, items: &[crate::repositories::invoices::InvoiceItem]) -> Result<(), AppError> {
    let content = build_text_receipt(inv, items);
    
    #[cfg(target_os = "windows")]
    {
        // Write to temp file and print via notepad /p
        let temp_path = std::env::temp_dir().join(format!("receipt_{}.txt", inv.id));
        tokio::fs::write(&temp_path, &content).await?;
        
        Command::new("notepad")
            .args(["/p", temp_path.to_str().unwrap()])
            .spawn()?;
    }
    
    #[cfg(target_os = "macos")]
    {
        let temp_path = std::env::temp_dir().join(format!("receipt_{}.txt", inv.id));
        tokio::fs::write(&temp_path, &content).await?;
        
        Command::new("lpr")
            .arg(temp_path.to_str().unwrap())
            .spawn()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        let temp_path = std::env::temp_dir().join(format!("receipt_{}.txt", inv.id));
        tokio::fs::write(&temp_path, &content).await?;
        
        Command::new("lp")
            .arg(temp_path.to_str().unwrap())
            .spawn()?;
    }
    
    Ok(())
}

fn build_escpos_receipt(inv: &crate::repositories::invoices::Invoice, items: &[crate::repositories::invoices::InvoiceItem]) -> Vec<u8> {
    let mut data = Vec::new();
    
    // Initialize
    data.extend_from_slice(&[ESC, b'@']);
    
    // Center align
    data.extend_from_slice(&[ESC, b'a', 1]);
    
    // Header
    data.extend_from_slice(b"CITY TILES POS\n");
    data.extend_from_slice(b"================\n\n");
    
    // Invoice info
    data.extend_from_slice(&[ESC, b'a', 0]); // Left align
    data.extend_from_slice(format!("Invoice: {}\n", inv.invoice_no).as_bytes());
    data.extend_from_slice(format!("Date: {}\n", inv.created_at.split('T').next().unwrap_or("")).as_bytes());
    data.extend_from_slice(format!("Customer: {}\n", inv.customer_name).as_bytes());
    data.extend_from_slice(b"----------------\n");
    
    // Items
    for item in items {
        data.extend_from_slice(format!("{} x {} {}\n", item.quantity, item.unit, item.product_name).as_bytes());
        data.extend_from_slice(format!("  {} @ {} = {}\n", item.unit, format_price(item.unit_price), format_price(item.line_total)).as_bytes());
    }
    
    data.extend_from_slice(b"----------------\n");
    
    // Totals
    data.extend_from_slice(format!("Subtotal: {}\n", format_price(inv.subtotal)).as_bytes());
    if inv.discount > 0 {
        data.extend_from_slice(format!("Discount: -{}\n", format_price(inv.discount)).as_bytes());
    }
    data.extend_from_slice(format!("TOTAL: {}\n", format_price(inv.total)).as_bytes());
    data.extend_from_slice(format!("Paid: {}\n", format_price(inv.amount_paid)).as_bytes());
    if inv.amount_paid < inv.total {
        data.extend_from_slice(format!("Balance: {}\n", format_price(inv.total - inv.amount_paid)).as_bytes());
    }
    data.extend_from_slice(format!("Method: {}\n", inv.payment_method).as_bytes());
    
    if let Some(notes) = &inv.notes {
        if !notes.is_empty() {
            data.extend_from_slice(b"\nNotes:\n");
            data.extend_from_slice(format!("{}\n", notes).as_bytes());
        }
    }
    
    data.extend_from_slice(b"\n================\n");
    data.extend_from_slice(b"Thank you!\n\n\n");
    
    // Cut paper
    data.extend_from_slice(&[GS, b'V', 1]);
    
    data
}

fn build_text_receipt(inv: &crate::repositories::invoices::Invoice, items: &[crate::repositories::invoices::InvoiceItem]) -> String {
    let mut output = String::new();
    
    output.push_str("CITY TILES POS\n");
    output.push_str("================\n\n");
    output.push_str(&format!("Invoice: {}\n", inv.invoice_no));
    output.push_str(&format!("Date: {}\n", inv.created_at.split('T').next().unwrap_or("")));
    output.push_str(&format!("Customer: {}\n", inv.customer_name));
    output.push_str("----------------\n");
    
    for item in items {
        output.push_str(&format!("{} x {} {}\n", item.quantity, item.unit, item.product_name));
        output.push_str(&format!("  {} @ {} = {}\n", item.unit, format_price(item.unit_price), format_price(item.line_total)));
    }
    
    output.push_str("----------------\n");
    output.push_str(&format!("Subtotal: {}\n", format_price(inv.subtotal)));
    if inv.discount > 0 {
        output.push_str(&format!("Discount: -{}\n", format_price(inv.discount)));
    }
    output.push_str(&format!("TOTAL: {}\n", format_price(inv.total)));
    output.push_str(&format!("Paid: {}\n", format_price(inv.amount_paid)));
    if inv.amount_paid < inv.total {
        output.push_str(&format!("Balance: {}\n", format_price(inv.total - inv.amount_paid)));
    }
    output.push_str(&format!("Method: {}\n", inv.payment_method));
    
    if let Some(notes) = &inv.notes {
        if !notes.is_empty() {
            output.push_str("\nNotes:\n");
            output.push_str(&format!("{}\n", notes));
        }
    }
    
    output.push_str("\n================\n");
    output.push_str("Thank you!\n\n\n");
    
    output
}

/// Amounts are stored and displayed as whole rupees everywhere in the app
/// (see HANDOFF.md locked decisions) — no paise conversion here.
fn format_price(rupees: i64) -> String {
    format!("PKR {}", rupees)
}

pub async fn list_usb_printers() -> Result<Vec<crate::commands::print::UsbPrinterInfo>, AppError> {
    use rusb::Context;
    
    let ctx = Context::new()?;
    let devices = ctx.devices()?;
    let mut printers = Vec::new();
    
    for device in devices.iter() {
        let desc = device.device_descriptor()?;
        // Filter for printer class (0x07) or known VID/PIDs
        if desc.class_code() == 0x07 || is_known_printer(desc.vendor_id(), desc.product_id()) {
            let mut info = crate::commands::print::UsbPrinterInfo {
                vid: desc.vendor_id(),
                pid: desc.product_id(),
                manufacturer: None,
                product: None,
                serial_number: None,
            };
            
            // Try to get strings
            if let Ok(handle) = device.open() {
                let timeout = Duration::from_secs(5);
                if let Ok(languages) = handle.read_languages(timeout) {
                    if let Some(lang) = languages.first() {
                        if let Ok(s) = handle.read_manufacturer_string(*lang, &desc, timeout) {
                            info.manufacturer = Some(s);
                        }
                        if let Ok(s) = handle.read_product_string(*lang, &desc, timeout) {
                            info.product = Some(s);
                        }
                        if let Ok(s) = handle.read_serial_number_string(*lang, &desc, timeout) {
                            info.serial_number = Some(s);
                        }
                    }
                }
            }
            
            printers.push(info);
        }
    }
    
    Ok(printers)
}

fn is_known_printer(vid: u16, pid: u16) -> bool {
    const KNOWN_PRINTERS: &[(u16, u16)] = &[
        (0x0416, 0x5011), // Winic/Epson-compatible (common in budget thermal printers)
        (0x0483, 0x5740), // STMicroelectronics CDC (common thermal board)
        (0x0FE6, 0x811E), // Xprinter and clones
        (0x1FC9, 0x2016), // Posiflex
        (0x04B8, 0x0202), // Seiko Epson TM series
        (0x0499, 0x0032), // Zjiang / Goojprt-style boards
    ];
    KNOWN_PRINTERS.contains(&(vid, pid))
}