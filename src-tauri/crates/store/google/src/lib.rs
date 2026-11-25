pub use store_core::{StoreError, StoreProduct, StorePurchase, StoreResult};

#[cfg(any(target_os = "android", test))]
mod txn;

#[cfg(target_os = "android")]
mod platform;

#[cfg(target_os = "android")]
pub use platform::{finish, list_products, purchase, restore};

#[cfg(not(target_os = "android"))]
mod stub;

#[cfg(not(target_os = "android"))]
pub use stub::{list_products, purchase, restore};