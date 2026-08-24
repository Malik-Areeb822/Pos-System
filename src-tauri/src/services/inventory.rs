// src-tauri/src/services/inventory.rs
use crate::DbPool;
use crate::error::AppError;
use std::path::Path;

#[derive(serde::Serialize)]
pub struct CsvImportResult {
    pub imported: i64,
    pub errors: Vec<String>,
}

fn parse_price(paise_str: &str) -> i64 {
    let clean = paise_str.replace([' ', '.', ','], "");
    let paise: i64 = clean.parse().unwrap_or(0);
    paise * 100 // Convert to paise (from rupees)
}

fn parse_category(cat: &str) -> String {
    match cat.to_lowercase().trim() {
        "marble" => "marble".to_string(),
        "tiles" => "tiles".to_string(),
        "chips" => "chips".to_string(),
        "sanitary" => "sanitary".to_string(),
        _ => "tiles".to_string(),
    }
}

pub async fn import_products_from_csv(pool: &DbPool, csv_content: &str) -> Result<CsvImportResult, AppError> {
    let mut imported = 0;
    let mut errors = Vec::new();

    let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());

    for result in rdr.records() {
        let row = match result {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("CSV parse error: {}", e));
                continue;
            }
        };

        // Skip empty rows
        if row.iter().all(|field| field.trim().is_empty()) {
            continue;
        }

        // Minimum: name required (column 0)
        if row.is_empty() {
            errors.push(format!("Row {}: Product name is required", imported + 1));
            continue;
        }

        let name = &row[0];

        let sku = if row.len() > 1 && !row[1].is_empty() {
            Some(row[1].to_string())
        } else {
            None
        };

        let category = if row.len() > 2 && !row[2].is_empty() {
            Some(parse_category(&row[2]))
        } else {
            Some("tiles".to_string())
        };

        let description = if row.len() > 3 && !row[3].is_empty() {
            Some(row[3].to_string())
        } else {
            None
        };

        let color = if row.len() > 4 && !row[4].is_empty() {
            Some(row[4].to_string())
        } else {
            None
        };

        let size = if row.len() > 5 && !row[5].is_empty() {
            Some(row[5].to_string())
        } else {
            None
        };

        let finish = if row.len() > 6 && !row[6].is_empty() {
            Some(row[6].to_string())
        } else {
            None
        };

        let unit = if row.len() > 7 && !row[7].is_empty() {
            row[7].to_string()
        } else {
            "pcs".to_string()
        };

        let price = if row.len() > 8 && !row[8].is_empty() {
            parse_price(&row[8])
        } else {
            0
        };

        let pieces_per_carton = if row.len() > 9 && !row[9].is_empty() {
            row[9].parse::<i64>().ok()
        } else {
            None
        };

        let stock_qty = if row.len() > 10 && !row[10].is_empty() {
            row[10].parse::<i64>().unwrap_or(0)
        } else {
            0
        };

        let low_stock_threshold = if row.len() > 11 && !row[11].is_empty() {
            row[11].parse::<i64>().unwrap_or(5)
        } else {
            5
        };

        let image_url = if row.len() > 12 && !row[12].is_empty() {
            Some(row[12].to_string())
        } else {
            None
        };

        let is_published = if row.len() > 13 && !row[13].is_empty() {
            row[13].to_lowercase().trim() == "true" || row[13].parse::<i64>().unwrap_or(1) == 1
        } else {
            true
        } as i64;

        // Create product via repository
        let repo = crate::repositories::ProductRepository::new(pool.clone());
        let input = crate::repositories::CreateProductInput {
            name: name.to_string(),
            sku,
            category: category.unwrap_or_else(|| "tiles".to_string()),
            description,
            color,
            size,
            finish,
            company: None,
            unit,
            price,
            pieces_per_carton,
            stock_qty,
            low_stock_threshold,
            image_url,
            is_published,
        };

        match repo.create(input).await {
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("Row {}: {}", imported + 1, e)),
        }
    }

    Ok(CsvImportResult {
        imported,
        errors,
    })
}