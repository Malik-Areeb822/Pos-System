// src-tauri/src/services/print.rs
use rusb::UsbContext as _;
use crate::DbPool;
use crate::error::AppError;
use crate::repositories::InvoiceRepository;
use std::time::Duration;

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;

/// Characters per line on an 80mm roll at default font (Font A).
/// 58mm printers typically fit 32 — adjust here if ever needed.
const RECEIPT_WIDTH: usize = 48;

/// Fallback business identity printed on every receipt. Mirrors
/// `src/lib/business.ts`; the frontend normally sends this explicitly.
const DEFAULT_BUSINESS_NAME: &str = "Moon Pipe";
const DEFAULT_BUSINESS_ADDRESS: &str = "Mansehra Road, Abbottabad, Khyber Pakhtunkhwa";
const DEFAULT_BUSINESS_PHONE: &str = "0334 5333447";

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReceiptBusiness {
    pub name: String,
    pub address: String,
    pub phone: String,
}

impl Default for ReceiptBusiness {
    fn default() -> Self {
        Self {
            name: DEFAULT_BUSINESS_NAME.to_string(),
            address: DEFAULT_BUSINESS_ADDRESS.to_string(),
            phone: DEFAULT_BUSINESS_PHONE.to_string(),
        }
    }
}

pub async fn print_receipt(
    pool: &DbPool,
    invoice_id: &str,
    business: Option<ReceiptBusiness>,
) -> Result<(), AppError> {
    let repo = InvoiceRepository::new(pool.clone());
    let invoice = repo.get_with_items(invoice_id).await?
        .ok_or(AppError::NotFound("Invoice not found".into()))?;

    let (inv, items) = invoice;
    let biz = business.unwrap_or_default();

    // Preferred: raster receipt rendered with the POS's real typefaces
    // (Karla + Cormorant Garamond) via the GS v 0 raster command.
    // Fallback: classic ESC/POS text mode if font rendering is unavailable.
    let data = match crate::services::receipt_bitmap::render_receipt(&inv, &items, &biz) {
        Some(bytes) => bytes,
        None => {
            tracing::warn!("Receipt font rendering unavailable; using ESC/POS text mode");
            build_escpos_receipt(&inv, &items, &biz)
        }
    };

    // Path 1 (Windows): Print Spooler API with RAW datatype — the printer is
    // installed as a Windows printer, so this is the predictable route. RAW
    // bypasses driver pagination entirely; the roll only advances for the
    // content we actually send.
    #[cfg(target_os = "windows")]
    {
        match print_via_spooler(&data).await {
            Ok(()) => return Ok(()),
            Err(e) => tracing::warn!(
                "Spooler RAW print failed ({}); falling back to direct USB ESC/POS",
                e
            ),
        }
        // Path 2 (Windows): direct USB ESC/POS — works even without a driver.
        print_escpos_usb(&data).await
    }

    // Non-Windows: lp/lpr with the plain-text twin (no RAW passthrough API).
    #[cfg(not(target_os = "windows"))]
    {
        let _ = &data;
        print_via_spooler_unix(&inv, &items).await
    }
}

async fn print_escpos_usb(data: &[u8]) -> Result<(), AppError> {
    use rusb::{Context, Direction, TransferType};

    // Accept known thermal-printer VID/PIDs (see is_known_printer below),
    // plus any device claiming the standard USB printer class (0x07) —
    // covers budget 80mm/58mm models not on the list.
    let ctx = Context::new()?;
    let devices = ctx.devices()?;

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

        let handle = match device.open() {
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

        match handle.write_bulk(endpoint_addr, data, Duration::from_secs(5)) {
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

/// Send raw bytes to the default Windows printer using the Print Spooler API
/// with the RAW datatype.
#[cfg(target_os = "windows")]
async fn print_via_spooler(data: &[u8]) -> Result<(), AppError> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Graphics::Printing::{
        ClosePrinter, EndDocPrinter, GetDefaultPrinterW, OpenPrinterW, StartDocPrinterW,
        WritePrinter, DOC_INFO_1W,
    };

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    // 1. Resolve the default printer name.
    let mut len: u32 = 0;
    // First call with null buffer fails with INSUFFICIENT_BUFFER and sets len.
    let _ = unsafe { GetDefaultPrinterW(PWSTR::null(), &mut len) };
    if len == 0 {
        return Err(AppError::Internal(
            "No default printer installed — set your 80mm printer as the Windows default printer"
                .into(),
        ));
    }
    let mut name_buf = vec![0u16; len as usize];
    if !unsafe { GetDefaultPrinterW(PWSTR(name_buf.as_mut_ptr()), &mut len) }.as_bool() {
        return Err(AppError::Internal("GetDefaultPrinter call failed".into()));
    }
    let name_end = name_buf.iter().position(|&c| c == 0).unwrap_or(len as usize);
    let printer_name = String::from_utf16_lossy(&name_buf[..name_end]);

    // 2. Open a handle to it.
    let mut handle = HANDLE::default();
    unsafe { OpenPrinterW(PCWSTR(name_buf.as_ptr()), &mut handle, None) }
        .map_err(|e| AppError::Internal(format!("Could not open printer '{}': {}", printer_name, e)))?;

    // 3. Start a RAW job.
    let mut doc_name = wide("MoonPipe Receipt");
    let mut datatype = wide("RAW");
    let doc = DOC_INFO_1W {
        pDocName: PWSTR(doc_name.as_mut_ptr()),
        pOutputFile: PWSTR::null(),
        pDatatype: PWSTR(datatype.as_mut_ptr()),
    };
    let job = unsafe { StartDocPrinterW(handle, 1, &doc) };
    if job == 0 {
        let _ = unsafe { ClosePrinter(handle) };
        return Err(AppError::Internal(format!(
            "Spooler rejected print job for '{}'",
            printer_name
        )));
    }

    // 4. Stream the ESC/POS bytes.
    let mut written: u32 = 0;
    let ok = unsafe {
        WritePrinter(handle, data.as_ptr().cast(), data.len() as u32, &mut written)
    }
    .as_bool()
        && written as usize == data.len();
    let ended = unsafe { EndDocPrinter(handle) }.as_bool();
    let _ = unsafe { ClosePrinter(handle) };

    if !ok {
        return Err(AppError::Internal(
            "Print spooling failed or the job was truncated".into(),
        ));
    }
    if !ended {
        return Err(AppError::Internal("Failed to finish print job".into()));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn print_via_spooler_unix(
    inv: &crate::repositories::invoices::Invoice,
    items: &[crate::repositories::invoices::InvoiceItem],
) -> Result<(), AppError> {
    use std::process::Command;

    let temp_path = std::env::temp_dir().join(format!("receipt_{}.txt", inv.id));
    tokio::fs::write(&temp_path, build_text_receipt(inv, items)).await?;

    #[cfg(target_os = "macos")]
    Command::new("lpr").arg(temp_path.to_str().unwrap()).spawn()?;
    #[cfg(all(unix, not(target_os = "macos")))]
    Command::new("lp").arg(temp_path.to_str().unwrap()).spawn()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ESC/POS receipt builder
// ---------------------------------------------------------------------------

/// Right-aligned two-column row; wraps the left column when it doesn't fit.
fn two_col(left: &str, right: &str) -> String {
    let left_chars: Vec<char> = left.chars().collect();
    let right_len = right.chars().count();
    let max_left = RECEIPT_WIDTH.saturating_sub(right_len + 1);

    let mut out = String::new();
    if left_chars.len() <= max_left {
        out.push_str(left);
        for _ in 0..(RECEIPT_WIDTH - left_chars.len() - right_len) {
            out.push(' ');
        }
        out.push_str(right);
        out.push('\n');
    } else {
        // Wrap the left column; the amount rides on the final chunk.
        let mut start = 0;
        while start < left_chars.len() {
            let end = (start + max_left).min(left_chars.len());
            let chunk: String = left_chars[start..end].iter().collect();
            if end < left_chars.len() {
                out.push_str(&chunk);
                out.push('\n');
            } else {
                let pad = RECEIPT_WIDTH - chunk.chars().count() - right_len;
                out.push_str(&chunk);
                for _ in 0..pad {
                    out.push(' ');
                }
                out.push_str(right);
                out.push('\n');
            }
            start = end;
        }
    }
    out
}

/// Word-wrapped, manually centered line for the plain-text twin only.
/// (On the thermal path the printer's own ESC a 1 center mode is used.)
#[cfg_attr(windows, allow(dead_code))]
fn center_wrapped(text: &str) -> String {
    fn flush(out: &mut String, line: &str) {
        let pad = RECEIPT_WIDTH.saturating_sub(line.chars().count()) / 2;
        for _ in 0..pad {
            out.push(' ');
        }
        out.push_str(line);
        out.push('\n');
    }

    let mut out = String::new();
    for raw_line in text.lines() {
        let words: Vec<&str> = raw_line.split_whitespace().collect();
        if words.is_empty() {
            out.push('\n');
            continue;
        }
        let mut current = String::new();
        for word in words {
            let next_len = current.chars().count()
                + if current.is_empty() { 0 } else { 1 }
                + word.chars().count();
            if next_len > RECEIPT_WIDTH && !current.is_empty() {
                flush(&mut out, &current);
                current.clear();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        if !current.is_empty() {
            flush(&mut out, &current);
        }
    }
    out
}

fn rule(c: char) -> String {
    let mut s = c.to_string().repeat(RECEIPT_WIDTH);
    s.push('\n');
    s
}

/// Invoice timestamp as local "YYYY-MM-DD HH:MM AM/PM"; created_at is stored ISO-8601.
/// Falls back to the bare date slice when parsing fails.
fn format_timestamp(created_at: &str) -> String {
    let parsed = chrono::DateTime::parse_from_rfc3339(created_at)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %I:%M %p").to_string())
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(created_at, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        })
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(created_at, "%Y-%m-%dT%H:%M:%S")
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        });

    match parsed {
        Ok(ts) => ts,
        Err(_) => created_at.split('T').next().unwrap_or(created_at).to_string(),
    }
}

fn build_escpos_receipt(
    inv: &crate::repositories::invoices::Invoice,
    items: &[crate::repositories::invoices::InvoiceItem],
    business: &ReceiptBusiness,
) -> Vec<u8> {
    let mut data = Vec::new();

    // Initialize printer state, enter center mode for the whole header.
    data.extend_from_slice(&[ESC, b'@']);
    data.extend_from_slice(&[ESC, b'a', 1]);

    // Business name large and centered.
    data.extend_from_slice(&[GS, b'!', 0x11]); // double height + width
    data.extend_from_slice(business.name.trim_end().as_bytes());
    data.push(b'\n');
    data.extend_from_slice(&[GS, b'!', 0x00]); // normal size

    // Address + phone — the printer centers them (no manual padding).
    data.extend_from_slice(business.address.trim().as_bytes());
    data.push(b'\n');
    data.extend_from_slice(business.phone.trim().as_bytes());
    data.push(b'\n');

    data.extend_from_slice(rule('=').as_bytes());

    // Left-aligned meta block.
    data.extend_from_slice(&[ESC, b'a', 0]);
    data.extend_from_slice(two_col("Invoice", &inv.invoice_no).as_bytes());
    data.extend_from_slice(two_col("Date", &format_timestamp(&inv.created_at)).as_bytes());
    data.extend_from_slice(two_col("Customer", &inv.customer_name).as_bytes());
    data.extend_from_slice(rule('-').as_bytes());

    // Items: "qty x unit name" left, line total right; rate underneath.
    for item in items {
        data.extend_from_slice(
            two_col(
                &format!("{} x {} {}", item.quantity, item.unit, item.product_name),
                &format_price(item.line_total),
            )
            .as_bytes(),
        );
        data.extend_from_slice(
            format!("      @ {} / {}\n", format_price(item.unit_price), item.unit).as_bytes(),
        );
    }
    data.extend_from_slice(rule('-').as_bytes());

    // Totals — TOTAL row emphasised with bold.
    data.extend_from_slice(two_col("Subtotal", &format_price(inv.subtotal)).as_bytes());
    if inv.discount > 0 {
        data.extend_from_slice(
            two_col("Discount", &format!("-{}", format_price(inv.discount))).as_bytes(),
        );
    }
    data.extend_from_slice(&[ESC, b'E', 1]); // bold on
    data.extend_from_slice(two_col("TOTAL", &format_price(inv.total)).as_bytes());
    data.extend_from_slice(&[ESC, b'E', 0]); // bold off
    data.extend_from_slice(rule('=').as_bytes());
    data.extend_from_slice(two_col("Paid", &format_price(inv.amount_paid)).as_bytes());
    if inv.amount_paid < inv.total {
        data.extend_from_slice(
            two_col("Balance", &format_price(inv.total - inv.amount_paid)).as_bytes(),
        );
    }
    data.extend_from_slice(format!("Method: {}\n", inv.payment_method).as_bytes());

    if let Some(notes) = &inv.notes {
        if !notes.is_empty() {
            data.extend_from_slice(b"\nNotes:\n");
            data.extend_from_slice(notes.as_bytes());
            data.push(b'\n');
        }
    }

    // Centered footer: thanks + developer credit.
    data.extend_from_slice(&[ESC, b'a', 1]);
    data.extend_from_slice(b"\nThank you!\nDeveloped by AZ Solutions\n03311203090\n");

    // Short trailing feed + auto cut (no page-sized waste — the printer only
    // feeds what we ask for).
    data.extend_from_slice(b"\n\n\n");
    data.extend_from_slice(&[GS, b'V', 1]);

    data
}

/// Plain-text twin used by the non-Windows spooler fallback only.
#[cfg(not(target_os = "windows"))]
fn build_text_receipt(
    inv: &crate::repositories::invoices::Invoice,
    items: &[crate::repositories::invoices::InvoiceItem],
) -> String {
    let mut out = String::new();
    out.push_str(&center_wrapped(DEFAULT_BUSINESS_NAME));
    out.push_str(&center_wrapped(DEFAULT_BUSINESS_ADDRESS));
    out.push_str(&center_wrapped(DEFAULT_BUSINESS_PHONE));
    out.push_str(&rule('='));
    out.push_str(&two_col("Invoice", &inv.invoice_no));
    out.push_str(&two_col("Date", &format_timestamp(&inv.created_at)));
    out.push_str(&two_col("Customer", &inv.customer_name));
    out.push_str(&rule('-'));
    for item in items {
        out.push_str(&two_col(
            &format!("{} x {} {}", item.quantity, item.unit, item.product_name),
            &format_price(item.line_total),
        ));
        out.push_str(&format!("      @ {} / {}\n", format_price(item.unit_price), item.unit));
    }
    out.push_str(&rule('-'));
    out.push_str(&two_col("Subtotal", &format_price(inv.subtotal)));
    if inv.discount > 0 {
        out.push_str(&two_col("Discount", &format!("-{}", format_price(inv.discount))));
    }
    out.push_str(&two_col("TOTAL", &format_price(inv.total)));
    out.push_str(&rule('='));
    out.push_str(&two_col("Paid", &format_price(inv.amount_paid)));
    if inv.amount_paid < inv.total {
        out.push_str(&two_col("Balance", &format_price(inv.total - inv.amount_paid)));
    }
    out.push_str(&format!("Method: {}\n", inv.payment_method));
    if let Some(notes) = &inv.notes {
        if !notes.is_empty() {
            out.push_str("\nNotes:\n");
            out.push_str(notes);
            out.push('\n');
        }
    }
    out.push_str(&center_wrapped("Thank you!"));
    out.push_str(&center_wrapped("Developed by AZ Solutions"));
    out.push_str(&center_wrapped("03311203090"));
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_col_aligns_amounts_right() {
        let row = two_col("Subtotal", "PKR 100");
        assert_eq!(row.trim_end().chars().count(), RECEIPT_WIDTH);
        assert!(row.ends_with("PKR 100\n"));
    }

    #[test]
    fn center_wraps_long_addresses_on_word_boundaries() {
        let out = center_wrapped(DEFAULT_BUSINESS_ADDRESS);
        for line in out.lines() {
            assert!(line.chars().count() <= RECEIPT_WIDTH);
        }
        assert_eq!(out.split_whitespace().collect::<Vec<_>>().join(" "), DEFAULT_BUSINESS_ADDRESS);
    }

    #[test]
    fn timestamp_parses_iso_and_keeps_fallback() {
        assert_eq!(format_timestamp("2026-08-23T14:35:00"), "2026-08-23 14:35");
        let expected = chrono::DateTime::parse_from_rfc3339("2026-08-23T14:35:00.123456+00:00")
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %I:%M %p")
            .to_string();
        assert_eq!(
            format_timestamp("2026-08-23T14:35:00.123456+00:00"),
            expected
        );
        assert_eq!(format_timestamp("2026-08-23"), "2026-08-23");
    }
}
