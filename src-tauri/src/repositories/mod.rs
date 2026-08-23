// src-tauri/src/repositories/mod.rs
pub mod products;
pub mod customers;
pub mod invoices;
pub mod returns;
pub mod cashiers;
pub mod users;

use sqlx::SqlitePool;

pub use products::{ProductRepository, Product, CreateProductInput};
pub use customers::{CustomerRepository, Customer, CreateCustomerInput};
pub use invoices::{InvoiceRepository, Invoice, InvoiceItem, CreateInvoiceInput, CreateInvoiceItemInput};
pub use returns::{ReturnRepository, Return, CreateReturnInput};
pub use cashiers::{CashierRepository, Cashier};
pub use users::{UserRepository, User, CreateUserInput};