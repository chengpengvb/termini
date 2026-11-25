pub use store_core::{StoreError, StoreProduct, StorePurchase, StoreResult};

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod txn;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod platform;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use platform::{finish, list_products, purchase, restore, unfinished, updates};

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod stub;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub use stub::{list_products, purchase, restore};