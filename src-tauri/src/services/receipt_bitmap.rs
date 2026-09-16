// src-tauri/src/services/receipt_bitmap.rs
//! Rasterized thermal-receipt renderer.
//!
//! ESC/POS *text* mode can only print the printer's built-in bitmap fonts —
//! the "typewriter" look. To print with the POS's real typefaces we render
//! the entire receipt to a monochrome bitmap and send it with the standard
//! `GS v 0` raster command, which every 80mm ESC/POS printer supports.
//!
//! Typefaces mirror the POS UI (`src/styles.css`): Cormorant Garamond Bold
//! for the brand banner, Karla for everything else.

use ab_glyph::{Font, FontVec, ScaleFont};
use once_cell::sync::Lazy;

/// Printable width of an 80mm roll at 203 dpi.
pub const PRINT_WIDTH_PX: usize = 576;
/// Packed bytes per raster row (576 / 8).
pub const ROW_BYTES: usize = PRINT_WIDTH_PX / 8;
/// Side padding (~3 mm).
const MARGIN_X: i32 = 24;
/// Blank rows above the first content and below the last content (~6 mm) —
/// kept equal so the receipt has symmetric top/bottom whitespace.
const MARGIN_Y_ROWS: usize = 48;
/// Raster data is sent in horizontal bands so even small-printer RAM copes.
const MAX_ROWS_PER_BAND: usize = 800;

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;

struct ReceiptFonts {
    karla_regular: FontVec,
    karla_bold: FontVec,
    cormorant_bold: FontVec,
}

static FONTS: Lazy<Option<ReceiptFonts>> = Lazy::new(|| {
    fn load(bytes: &[u8]) -> Option<FontVec> {
        FontVec::try_from_vec(bytes.to_vec()).ok()
    }
    Some(ReceiptFonts {
        karla_regular: load(include_bytes!("../assets/fonts/Karla-Regular.ttf"))?,
        karla_bold: load(include_bytes!("../assets/fonts/Karla-Bold.ttf"))?,
        cormorant_bold: load(include_bytes!(
            "../assets/fonts/CormorantGaramond-Bold.ttf"
        ))?,
    })
});

// ---------------------------------------------------------------------------
// Canvas — a dynamically growing packed-bitmap column
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Canvas {
    rows: Vec<[u8; ROW_BYTES]>,
}

impl Canvas {
    fn new() -> Self {
        Self::default()
    }

    fn push_blank(&mut self, n: usize) {
        self.rows.resize(self.rows.len() + n, [0u8; ROW_BYTES]);
    }

    /// Plot a single dot; silently ignores out-of-bounds coordinates so the
    /// layout math can stay branch-free.
    fn set_px(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= PRINT_WIDTH_PX || y >= self.rows.len() {
            return;
        }
        self.rows[y][x / 8] |= 0x80 >> (x % 8);
    }

    /// Horizontal rule: dashes on for 3 px / off for 3 px, two rows thick.
    fn dashed_rule(&mut self) {
        let y0 = self.rows.len() as i32;
        self.push_blank(2);
        let mut x = MARGIN_X;
        while x < PRINT_WIDTH_PX as i32 - MARGIN_X {
            if ((x - MARGIN_X) % 6) < 3 {
                self.set_px(x, y0);
                self.set_px(x, y0 + 1);
            }
            x += 1;
        }
    }

    /// Full-width solid rule, two rows thick (used around TOTAL).
    fn solid_rule(&mut self) {
        let y0 = self.rows.len() as i32;
        self.push_blank(2);
        for x in MARGIN_X..PRINT_WIDTH_PX as i32 - MARGIN_X {
            self.set_px(x, y0);
            self.set_px(x, y0 + 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Text measurement & drawing
// ---------------------------------------------------------------------------

fn measure(font: &FontVec, size: f32, text: &str) -> f32 {
    let scaled = font.as_scaled(size);
    text.chars()
        .map(|c| scaled.h_advance(scaled.glyph_id(c)))
        .sum()
}

/// Draw one run of text with its baseline at `baseline_y`, pen starting at
/// pen_x. Kerning is skipped — irrelevant at receipt sizes.
fn draw_string(canvas: &mut Canvas, font: &FontVec, size: f32, mut pen_x: f32, baseline_y: f32, text: &str) {
    let scaled = font.as_scaled(size);
    for ch in text.chars() {
        let gid = scaled.glyph_id(ch);
        let positioned = gid.with_scale_and_position(size, ab_glyph::point(pen_x, baseline_y));
        if let Some(outlined) = font.outline_glyph(positioned) {
            // OutlinedGlyph::draw yields offsets from the glyph's bounding-box
            // origin, so translate by px_bounds.min to get absolute pixels.
            let bounds = outlined.px_bounds();
            let bx = bounds.min.x as i32;
            let by = bounds.min.y as i32;
            outlined.draw(|px, py, alpha| {
                if alpha >= 0.4 {
                    canvas.set_px(bx + px as i32, by + py as i32);
                }
            });
        }
        pen_x += scaled.h_advance(gid);
    }
}

fn usable_width() -> f32 {
    (PRINT_WIDTH_PX as i32 - 2 * MARGIN_X) as f32
}

/// Word-wrap by measured pixel widths.
fn wrap(font: &FontVec, size: f32, text: &str, max_w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in text.lines() {
        let words: Vec<&str> = raw.split_whitespace().collect();
        if words.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in words {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current, word)
            };
            if measure(font, size, &candidate) > max_w && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current = word.to_string();
            } else {
                current = candidate;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

struct TextStyle<'a> {
    font: &'a FontVec,
    size: f32,
    /// Extra pixels added after each line on top of the natural line box —
    /// generous line spacing was an explicit client requirement.
    leading: f32,
}

impl<'a> TextStyle<'a> {
    fn line_advance(&self) -> f32 {
        let scaled = self.font.as_scaled(self.size);
        (scaled.ascent() - scaled.descent()) + self.leading
    }

    fn ascent(&self) -> f32 {
        self.font.as_scaled(self.size).ascent()
    }
}

/// Reserve one line of vertical space, then draw `text` at `x`.
fn draw_line_at(canvas: &mut Canvas, style: &TextStyle, text: &str, x: f32) {
    let top = canvas.rows.len();
    canvas.push_blank(style.line_advance().ceil() as usize);
    draw_string(canvas, style.font, style.size, x, top as f32 + style.ascent(), text);
}

fn draw_centered(canvas: &mut Canvas, style: &TextStyle, text: &str) {
    for line in wrap(style.font, style.size, text, usable_width()) {
        let w = measure(style.font, style.size, &line);
        let x = ((PRINT_WIDTH_PX as f32 - w) / 2.0).max(MARGIN_X as f32);
        draw_line_at(canvas, style, &line, x);
    }
}

fn draw_left(canvas: &mut Canvas, style: &TextStyle, text: &str, indent_x: f32) {
    let max_w = PRINT_WIDTH_PX as f32 - indent_x - MARGIN_X as f32;
    for line in wrap(style.font, style.size, text, max_w) {
        draw_line_at(canvas, style, &line, indent_x);
    }
}

/// Label (bold) at the left margin, value (regular) starting at `value_x`,
/// wrapped within the remaining width.
fn draw_meta_row(canvas: &mut Canvas, fonts: &ReceiptFonts, label: &str, value: &str, value_x: f32) {
    let label_style = TextStyle { font: &fonts.karla_bold, size: 28.0, leading: 10.0 };
    let value_style = TextStyle { font: &fonts.karla_regular, size: 28.0, leading: 10.0 };

    let max_w = PRINT_WIDTH_PX as f32 - value_x - MARGIN_X as f32;
    let lines = wrap(&fonts.karla_regular, value_style.size, value, max_w);

    // First line shares its row with the bold label.
    let top = canvas.rows.len();
    canvas.push_blank(label_style.line_advance().ceil() as usize);
    let base_y = top as f32 + label_style.ascent();
    draw_string(canvas, &fonts.karla_bold, label_style.size, MARGIN_X as f32, base_y, label);
    if let Some(first) = lines.first() {
        draw_string(canvas, &fonts.karla_regular, value_style.size, value_x, base_y, first);
    }
    for line in lines.iter().skip(1) {
        draw_line_at(canvas, &value_style, line, value_x);
    }
}

/// Two-column money row: left text (wrapped when long), amount right-aligned
/// on the final line. The amount is always drawn bold for scannability.
fn draw_two_col(canvas: &mut Canvas, fonts: &ReceiptFonts, left: &str, right: &str, bold_left: bool, size: f32) {
    let left_font = if bold_left { &fonts.karla_bold } else { &fonts.karla_regular };
    let right_w = measure(&fonts.karla_bold, size, right);
    let gap = 16.0;
    let max_left = usable_width() - right_w - gap;

    let lines = wrap(left_font, size, left, max_left);
    let style = TextStyle { font: left_font, size, leading: 10.0 };
    let last = lines.len().saturating_sub(1);

    for (i, line) in lines.into_iter().enumerate() {
        let top = canvas.rows.len();
        canvas.push_blank(style.line_advance().ceil() as usize);
        let base_y = top as f32 + style.ascent();

        draw_string(canvas, left_font, size, MARGIN_X as f32, base_y, &line);
        if i == last {
            let rx = PRINT_WIDTH_PX as f32 - MARGIN_X as f32 - right_w;
            draw_string(canvas, &fonts.karla_bold, size, rx, base_y, right);
        }
    }
}

// ---------------------------------------------------------------------------
// Receipt composition
// ---------------------------------------------------------------------------

type Invoice = crate::repositories::invoices::Invoice;
type InvoiceItem = crate::repositories::invoices::InvoiceItem;

/// Whole rupees are canonical everywhere (HANDOFF locked decision).
fn price(rupees: i64) -> String {
    format!("PKR {}", rupees)
}

/// ISO-8601 stored timestamp -> local "YYYY-MM-DD HH:MM AM/PM".
fn timestamp(created_at: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(created_at)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %I:%M %p").to_string())
        .unwrap_or_else(|_| created_at.split('T').next().unwrap_or(created_at).to_string())
}

/// Render the full receipt to an ESC/POS byte stream (init + raster bands +
/// cut). Returns `None` only when the bundled fonts cannot be parsed, so the
/// caller can fall back to classic ESC/POS text mode.
pub fn render_receipt(
    inv: &Invoice,
    items: &[InvoiceItem],
    business: &crate::services::print::ReceiptBusiness,
) -> Option<Vec<u8>> {
    let fonts = FONTS.as_ref()?;
    let mut c = Canvas::new();
    c.push_blank(MARGIN_Y_ROWS);

    // --- Header -----------------------------------------------------------
    let banner = TextStyle { font: &fonts.cormorant_bold, size: 72.0, leading: 14.0 };
    draw_centered(&mut c, &banner, business.name.trim());

    let small = TextStyle { font: &fonts.karla_regular, size: 26.0, leading: 12.0 };
    draw_centered(&mut c, &small, business.address.trim());
    draw_centered(&mut c, &small, business.phone.trim());

    c.push_blank(14);
    c.dashed_rule();
    c.push_blank(12);

    // --- Meta -------------------------------------------------------------
    draw_meta_row(&mut c, fonts, "Invoice", &inv.invoice_no, 170.0);
    draw_meta_row(&mut c, fonts, "Date", &timestamp(&inv.created_at), 170.0);
    if !inv.customer_name.trim().is_empty() {
        draw_meta_row(&mut c, fonts, "Customer", inv.customer_name.trim(), 170.0);
    }

    c.push_blank(12);
    c.dashed_rule();
    c.push_blank(12);

    // --- Items ------------------------------------------------------------
    let body_size = 29.0;
    for item in items {
        let head = format!("{} {} \u{00D7} {}", item.quantity, item.unit, item.product_name);
        draw_two_col(&mut c, fonts, &head, &price(item.line_total), false, body_size);
        draw_left(
            &mut c,
            &TextStyle { font: &fonts.karla_regular, size: 25.0, leading: 8.0 },
            &format!("@ {} / {}", price(item.unit_price), item.unit),
            56.0,
        );
        if let Some(total_area) = item.total_area {
            if total_area > 0.0 {
                draw_left(
                    &mut c,
                    &TextStyle { font: &fonts.karla_bold, size: 25.0, leading: 8.0 },
                    &format!("Area: {:.3} sqm", total_area),
                    56.0,
                );
            }
        }
        c.push_blank(6);
    }
    c.push_blank(6);
    c.dashed_rule();
    c.push_blank(12);

    // --- Totals -----------------------------------------------------------
    draw_two_col(&mut c, fonts, "Subtotal", &price(inv.subtotal), false, body_size);
    if inv.discount > 0 {
        draw_two_col(&mut c, fonts, "Discount", &format!("-{}", price(inv.discount)), false, body_size);
    }
    c.push_blank(8);
    c.solid_rule();
    c.push_blank(10);
    draw_two_col(&mut c, fonts, "TOTAL", &price(inv.total), true, 36.0);
    c.push_blank(4);
    c.solid_rule();
    c.push_blank(12);

    draw_two_col(&mut c, fonts, "Paid", &price(inv.amount_paid), false, body_size);
    if inv.amount_paid < inv.total {
        draw_two_col(&mut c, fonts, "Balance due", &price(inv.total - inv.amount_paid), true, body_size);
    }
    draw_two_col(&mut c, fonts, "Method", &inv.payment_method, false, body_size);

    if let Some(notes) = inv.notes.as_deref().filter(|n| !n.trim().is_empty()) {
        c.push_blank(12);
        c.dashed_rule();
        c.push_blank(10);
        draw_left(
            &mut c,
            &TextStyle { font: &fonts.karla_regular, size: body_size, leading: 10.0 },
            notes.trim(),
            MARGIN_X as f32,
        );
    }

    // --- Footer -----------------------------------------------------------
    c.push_blank(20);
    draw_centered(&mut c, &TextStyle { font: &fonts.karla_regular, size: 28.0, leading: 12.0 }, "Thank you!");
    draw_centered(&mut c, &small, "Developed by AZ Solutions");
    draw_centered(&mut c, &TextStyle { font: &fonts.karla_regular, size: 22.0, leading: 10.0 }, "03311203090");

    // Symmetric bottom margin (mirrors MARGIN_Y_ROWS at the top).
    c.push_blank(MARGIN_Y_ROWS);

    Some(encode_raster_stream(&c))
}

// ---------------------------------------------------------------------------
// ESC/POS raster encoding
// ---------------------------------------------------------------------------

/// Wrap the canvas into `ESC @` + one or more `GS v 0` band commands +
/// paper-cut. `GS v 0` layout: `GS v 0 \0 xL xH yL yH <row data>`.
fn encode_raster_stream(c: &Canvas) -> Vec<u8> {
    let mut out = Vec::with_capacity(c.rows.len() * ROW_BYTES + 64);
    out.extend_from_slice(&[ESC, b'@']); // initialize

    let total_rows = c.rows.len();
    let mut start = 0usize;
    while start < total_rows {
        let end = (start + MAX_ROWS_PER_BAND).min(total_rows);
        let band_rows = end - start;

        out.extend_from_slice(&[GS, b'v', b'0', 0]);
        out.extend_from_slice(&(ROW_BYTES as u16).to_le_bytes()); // xL xH
        out.extend_from_slice(&(band_rows as u16).to_le_bytes()); // yL yH
        for row in &c.rows[start..end] {
            out.extend_from_slice(row);
        }
        start = end;
    }

    // Brief feed then full cut — the image already carries symmetric margins,
    // so only enough feed to clear the cutter is added.
    out.extend_from_slice(b"\n\n\n");
    out.extend_from_slice(&[GS, b'V', 1]);
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::GlyphId;

    #[test]
    fn fonts_load_and_cover_ascii() {
        let fonts = FONTS.as_ref().expect("bundled receipt fonts must parse");
        for ch in "ABCabc0123 PKR".chars() {
            assert_ne!(fonts.karla_regular.glyph_id(ch), GlyphId(0), "karla missing {:?}", ch);
        }
        assert_ne!(fonts.cormorant_bold.glyph_id('C'), GlyphId(0));
    }

    #[test]
    fn wrap_respects_pixel_budget() {
        let fonts = FONTS.as_ref().unwrap();
        let long_addr = "Mansehra Road, Abbottabad, Khyber Pakhtunkhwa, Pakistan";
        let lines = wrap(&fonts.karla_regular, 26.0, long_addr, usable_width());
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(measure(&fonts.karla_regular, 26.0, line) <= usable_width() + 0.5);
        }
    }

    #[test]
    fn raster_stream_header_is_valid() {
        let mut c = Canvas::new();
        c.push_blank(MARGIN_Y_ROWS * 2);
        let stream = encode_raster_stream(&c);
        assert_eq!(&stream[0..2], &[ESC, b'@']);
        assert_eq!(&stream[2..6], &[GS, b'v', b'0', 0]);
        assert_eq!(&stream[6..8], &(ROW_BYTES as u16).to_le_bytes());
        assert_eq!(&stream[8..10], &((MARGIN_Y_ROWS * 2) as u16).to_le_bytes());
        let expected_len = 2 + 8 + ROW_BYTES * (MARGIN_Y_ROWS * 2) + 3 + 3;
        assert_eq!(stream.len(), expected_len);
    }

    fn sample_invoice() -> (Invoice, Vec<InvoiceItem>) {
        let inv = Invoice {
            id: "t".into(),
            invoice_no: "INV-2026-0042".into(),
            customer_id: None,
            customer_name: "Walk-in customer".into(),
            subtotal: 4500,
            discount: 200,
            total: 4300,
            amount_paid: 3000,
            payment_method: "cash".into(),
            notes: Some("Deliver before Friday".into()),
            delivery_date: None,
            created_at: "2026-08-23T14:35:00+00:00".into(),
            updated_at: "2026-08-23T14:35:00+00:00".into(),
        };
        let items = vec![InvoiceItem {
            id: "i".into(),
            invoice_id: "t".into(),
            product_id: None,
            product_name: "Glazed Ceramic Floor Tile 60x60".into(),
            quantity: 2,
            unit: "sqft".into(),
            unit_price: 2250,
            line_total: 4500,
            total_area: None,
            created_at: String::new(),
        }];
        (inv, items)
    }

    #[test]
    fn full_receipt_renders_to_reasonable_bitmap() {
        use crate::services::print::ReceiptBusiness;
        let (inv, items) = sample_invoice();
        let stream = render_receipt(&inv, &items, &ReceiptBusiness::default()).expect("render ok");
        assert!(stream.len() > ROW_BYTES * 400); // real content, not an empty sliver
        assert_eq!(&stream[2..6], &[GS, b'v', b'0', 0]);
        assert_eq!(*stream.last().unwrap(), 0x01); // GS V 1 cut terminator
    }

    #[test]
    fn canvas_set_px_bounds_safe() {
        let mut c = Canvas::new();
        c.push_blank(4);
        c.set_px(-1, 0);
        c.set_px(0, -1);
        c.set_px(PRINT_WIDTH_PX as i32, 0);
        c.set_px(0, 9999);
        c.set_px(0, 0);
        assert_eq!(c.rows[0][0], 0x80);
        c.set_px(7, 0);
        assert_eq!(c.rows[0][0], 0x81);
    }
}
