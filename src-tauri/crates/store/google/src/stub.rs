use store_core::{StoreError, StoreProduct, StorePurchase, StoreResult, STORE_KIND_GOOGLE};

pub async fn list_products(_product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    Err(StoreError::Unsupported(STORE_KIND_GOOGLE))
}

pub async fn purchase(_hwnd: isize, _product_id: String) -> StoreResult<StorePurchase> {
    Err(StoreError::Unsupported(STORE_KIND_GOOGLE))
}

pub async fn restore() -> StoreResult<Vec<StorePurchase>> {
    Err(StoreError::Unsupported(STORE_KIND_GOOGLE))
}
