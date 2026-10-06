// src-tauri/src/test_support.rs
//
// Shared fixtures for the database-backed unit tests. Compiled only under
// `cfg(test)` — nothing here reaches a release build.

use crate::database::DbPool;
use crate::repositories::customers::{CreateCustomerInput, Customer, CustomerRepository};
use crate::repositories::invoices::{
    CreateInvoiceInput, CreateInvoiceItemInput, Invoice, InvoiceRepository,
};
use crate::repositories::suppliers::{
    CreatePurchaseInput, CreatePurchaseItemInput, CreateSupplierInput, Supplier,
    SupplierPurchase, SupplierPurchaseRepository, SupplierRepository,
};
use sqlx::sqlite::SqlitePoolOptions;

/// Database under test: a scratch file in the temp directory, migrated with
/// the real migrations.
///
/// Not `sqlite::memory:`: each in-memory connection gets its own private,
/// empty database, which would silently break any pool holding more than one
/// connection. A file keeps every connection on the same schema — and lets us
/// match the production pool size (5), which matters because `mark_paid`
/// legitimately reads through the pool while its transaction is open.
pub async fn test_pool() -> DbPool {
    let path = std::env::temp_dir().join(format!("moonpipe-test-{}.sqlite", uuid::Uuid::new_v4()));
    let url = format!(
        "sqlite:{}?mode=rwc",
        path.to_string_lossy().replace('\\', "/")
    );

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("connect to scratch test database");
    sqlx::migrate!("./src/database/migrations")
        .run(&pool)
        .await
        .expect("run migrations on test database");
    pool
}

pub async fn make_customer(pool: &DbPool, name: &str) -> Customer {
    CustomerRepository::new(pool.clone())
        .create(CreateCustomerInput {
            name: name.to_string(),
            phone: None,
            email: None,
            address: None,
        })
        .await
        .expect("create customer")
}

/// `returns.processed_by` is a foreign key onto `users`, so returns need one.
pub async fn seed_user(pool: &DbPool) -> String {
    let id = format!("test-user-{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, full_name, status) VALUES (?, ?, ?, ?, 'approved')",
    )
    .bind(&id)
    .bind(format!("{}@example.test", id))
    .bind("not-a-real-hash")
    .bind("Test User")
    .execute(pool)
    .await
    .expect("seed test user");
    id
}

pub fn item(name: &str, qty: i64, unit_price: i64) -> CreateInvoiceItemInput {
    CreateInvoiceItemInput {
        product_id: None,
        product_name: name.to_string(),
        quantity: qty,
        unit: "pcs".to_string(),
        unit_price,
        line_total: qty * unit_price,
        purchase_price: 0,
        total_area: None,
    }
}

/// A credit sale: nothing paid up front, so the whole total is due.
pub async fn credit_invoice(
    pool: &DbPool,
    customer: &Customer,
    items: Vec<CreateInvoiceItemInput>,
) -> Invoice {
    let subtotal = items.iter().map(|i| i.line_total).sum();
    InvoiceRepository::new(pool.clone())
        .create(CreateInvoiceInput {
            customer_id: Some(customer.id.clone()),
            customer_name: customer.name.clone(),
            subtotal,
            discount: 0,
            total: 0, // backend is source of truth; this is ignored
            amount_paid: None,
            payment_method: "credit".to_string(),
            notes: None,
            delivery_date: None,
            items,
        })
        .await
        .expect("create credit invoice")
}

/// Insert an invoice row exactly as it would look in a database that predates
/// the carry-forward fix — an open invoice nothing points at. Used to build
/// the legacy shapes the reconcile path has to survive.
pub async fn raw_open_invoice(
    pool: &DbPool,
    customer_id: Option<&str>,
    customer_name: &str,
    total: i64,
    previous_balance: i64,
    amount_paid: i64,
    carried_to: Option<&str>,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"INSERT INTO invoices
             (id, invoice_no, customer_id, customer_name, subtotal, discount, total,
              previous_balance, amount_paid, payment_method, notes, delivery_date,
              created_at, updated_at, carried_to_invoice_id)
           VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, 'credit', NULL, NULL, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(format!("INV-TEST-{}", &id[..8]))
    .bind(customer_id)
    .bind(customer_name)
    .bind(total)
    .bind(total)
    .bind(previous_balance)
    .bind(amount_paid)
    .bind(&now)
    .bind(&now)
    .bind(carried_to)
    .execute(pool)
    .await
    .expect("insert raw invoice");
    id
}

pub async fn get_invoice(pool: &DbPool, id: &str) -> Invoice {
    sqlx::query_as::<_, Invoice>(
        r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total,
                  previous_balance, amount_paid, payment_method, notes, delivery_date,
                  created_at, updated_at, carried_to_invoice_id
           FROM invoices WHERE id = ?"#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("fetch invoice")
}

pub async fn balance(pool: &DbPool, customer_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT outstanding_balance FROM customers WHERE id = ?")
        .bind(customer_id)
        .fetch_one(pool)
        .await
        .expect("fetch customer balance")
}

/// The sum every reconcile step is supposed to produce: due amount of the
/// open invoices that have *not* been carried away.
pub async fn expected_leaf_balance(pool: &DbPool, customer_id: &str) -> i64 {
    sqlx::query_scalar::<_, Option<i64>>(
        r#"SELECT COALESCE(SUM(total - amount_paid), 0) FROM invoices
           WHERE customer_id = ? AND carried_to_invoice_id IS NULL AND total > amount_paid"#,
    )
    .bind(customer_id)
    .fetch_one(pool)
    .await
    .expect("compute expected leaf balance")
    .unwrap_or(0)
}

pub async fn make_supplier(pool: &DbPool, name: &str) -> Supplier {
    SupplierRepository::new(pool.clone())
        .create(CreateSupplierInput {
            name: name.to_string(),
            phone: None,
            email: None,
            company: None,
            address: None,
            notes: None,
        })
        .await
        .expect("create supplier")
}

pub fn purchase_item(description: &str, qty: i64, unit_price: i64) -> CreatePurchaseItemInput {
    CreatePurchaseItemInput {
        description: description.to_string(),
        quantity: qty,
        unit: "pcs".to_string(),
        unit_price,
        line_total: qty * unit_price,
    }
}

/// A credit purchase: nothing paid up front, so the whole total is due.
pub async fn credit_purchase(
    pool: &DbPool,
    supplier: &Supplier,
    items: Vec<CreatePurchaseItemInput>,
) -> SupplierPurchase {
    let subtotal = items.iter().map(|i| i.line_total).sum();
    SupplierPurchaseRepository::new(pool.clone())
        .create(CreatePurchaseInput {
            supplier_id: supplier.id.clone(),
            supplier_name: supplier.name.clone(),
            subtotal,
            discount: 0,
            total: 0, // backend is source of truth; this is ignored
            amount_paid: None,
            payment_method: "credit".to_string(),
            notes: None,
            items,
        })
        .await
        .expect("create credit purchase")
}

/// Insert a purchase row exactly as it would look in a database that predates
/// the carry-forward fix — an open purchase nothing points at. Used to build
/// the legacy shapes the reconcile path has to survive.
pub async fn raw_open_purchase(
    pool: &DbPool,
    supplier_id: &str,
    supplier_name: &str,
    total: i64,
    previous_balance: i64,
    amount_paid: i64,
    carried_to: Option<&str>,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"INSERT INTO supplier_purchases
             (id, purchase_no, supplier_id, supplier_name, subtotal, discount, total,
              previous_balance, amount_paid, payment_method, notes,
              created_at, updated_at, carried_to_purchase_id)
           VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, 'credit', NULL, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(format!("PO-TEST-{}", &id[..8]))
    .bind(supplier_id)
    .bind(supplier_name)
    .bind(total)
    .bind(total)
    .bind(previous_balance)
    .bind(amount_paid)
    .bind(&now)
    .bind(&now)
    .bind(carried_to)
    .execute(pool)
    .await
    .expect("insert raw purchase");
    id
}

pub async fn get_purchase(pool: &DbPool, id: &str) -> SupplierPurchase {
    sqlx::query_as::<_, SupplierPurchase>(
        r#"SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total,
                  previous_balance, amount_paid, payment_method, notes,
                  created_at, updated_at, carried_to_purchase_id
           FROM supplier_purchases WHERE id = ?"#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("fetch purchase")
}

pub async fn supplier_balance(pool: &DbPool, supplier_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT outstanding_balance FROM suppliers WHERE id = ?")
        .bind(supplier_id)
        .fetch_one(pool)
        .await
        .expect("fetch supplier balance")
}

/// The sum every supplier reconcile step is supposed to produce: due amount
/// of the purchases that have *not* been carried away.
pub async fn expected_leaf_supplier_balance(pool: &DbPool, supplier_id: &str) -> i64 {
    sqlx::query_scalar::<_, Option<i64>>(
        r#"SELECT COALESCE(SUM(total - amount_paid), 0) FROM supplier_purchases
           WHERE supplier_id = ? AND carried_to_purchase_id IS NULL AND total > amount_paid"#,
    )
    .bind(supplier_id)
    .fetch_one(pool)
    .await
    .expect("compute expected leaf supplier balance")
    .unwrap_or(0)
}
