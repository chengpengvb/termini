use store_core::{StoreError, StoreProduct, StorePurchase, StoreResult, STORE_KIND_MICROSOFT};

pub async fn list_products(_product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    Err(StoreError::Unsupported(STORE_KIND_MICROSOFT))
}

pub async fn purchase(_hwnd: isize, _product_id: String) -> StoreResult<StorePurchase> {
    Err(StoreError::Unsupported(STORE_KIND_MICROSOFT))
}

pub async fn restore() -> StoreResult<Vec<StorePurchase>> {
    Err(StoreError::Unsupported(STORE_KIND_MICROSOFT))
}

pub async fn purchase_id(_service_ticket: String, _publisher_user_id: String) -> StoreResult<String> {
    Err(StoreError::Unsupported(STORE_KIND_MICROSOFT))
}
