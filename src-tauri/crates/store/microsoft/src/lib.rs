pub use store_core::{StoreError, StoreProduct, StorePurchase, StoreResult};

#[cfg(target_os = "windows")]
mod platform;

#[cfg(target_os = "windows")]
pub use platform::{list_products, purchase, purchase_id, restore};

#[cfg(not(target_os = "windows"))]
mod stub;

#[cfg(not(target_os = "windows"))]
pub use stub::{list_products, purchase, purchase_id, restore};