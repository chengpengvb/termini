use windows_collections::IIterable;
use windows::Win32::Foundation::HWND;
use windows::core::{Interface, HSTRING};
use windows::Win32::UI::Shell::IInitializeWithWindow;
use windows::Services::Store::{StoreContext, StorePurchaseStatus};
use store_core::{StoreError, StoreProduct, StorePurchase, StoreResult, STORE_KIND_MICROSOFT};

const STORE_ID_PRO_MONTHLY: &str = "9NV9HKP6T723";
const PRODUCT_PRO_MONTHLY: &str = "termini.pro.monthly";

fn resolve_store_id(product_id: &str) -> StoreResult<String> {
    match product_id {
        PRODUCT_PRO_MONTHLY => Ok(STORE_ID_PRO_MONTHLY.to_string()),
        other => Err(StoreError::Failed(format!("product not registered in Microsoft Store: {}", other))),
    }
}

pub async fn list_products(product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    spawn_blocking(move || list_products_blocking(product_ids)).await
}

pub async fn purchase(hwnd: isize, product_id: String) -> StoreResult<StorePurchase> {
    spawn_blocking(move || purchase_blocking(hwnd, product_id)).await
}

pub async fn restore() -> StoreResult<Vec<StorePurchase>> {
    spawn_blocking(restore_blocking).await
}

pub async fn purchase_id(service_ticket: String, publisher_user_id: String) -> StoreResult<String> {
    spawn_blocking(move || purchase_id_blocking(service_ticket, publisher_user_id)).await
}

async fn spawn_blocking<T: Send + 'static>(task: impl FnOnce() -> StoreResult<T> + Send + 'static) -> StoreResult<T> {
    tokio::task::spawn_blocking(task).await.map_err(|error| StoreError::Failed(error.to_string()))?
}

fn default_context() -> StoreResult<StoreContext> {
    StoreContext::GetDefault().map_err(|error| StoreError::NotReady(error.message()))
}

fn context_for_window(hwnd: isize) -> StoreResult<StoreContext> {
    let context = default_context()?;
    let init: IInitializeWithWindow = context.cast().map_err(|error| StoreError::NotReady(error.message()))?;
    unsafe { init.Initialize(HWND(hwnd as *mut core::ffi::c_void)) }.map_err(|error| StoreError::NotReady(error.message()))?;
    Ok(context)
}

fn list_products_blocking(product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    let context = default_context()?;
    let kinds: Vec<HSTRING> = vec![HSTRING::from("Durable"), HSTRING::from("Subscription")];
    let store_ids = product_ids.iter().map(|id| resolve_store_id(id)).collect::<StoreResult<Vec<String>>>()?;
    let ids: Vec<HSTRING> = store_ids.iter().map(|id| HSTRING::from(id.as_str())).collect();
    let kinds: IIterable<HSTRING> = kinds.into();
    let ids: IIterable<HSTRING> = ids.into();
    let result = context.GetStoreProductsAsync(&kinds, &ids).map_err(|error| StoreError::Failed(error.message()))?.join().map_err(|error| StoreError::Failed(error.message()))?;
    let map = result.Products().map_err(|error| StoreError::Failed(error.message()))?;
    let mut products = Vec::new();
    for pair in &map {
        let product = pair.Value().map_err(|error| StoreError::Failed(error.message()))?;
        products.push(StoreProduct {
            product_id: product.StoreId().map_err(|error| StoreError::Failed(error.message()))?.to_string(),
            title: product.Title().map(|value| value.to_string()).unwrap_or_default(),
            description: product.Description().map(|value| value.to_string()).unwrap_or_default(),
            price_text: product.Price().and_then(|price| price.FormattedPrice()).map(|value| value.to_string()).unwrap_or_default(),
        });
    }
    Ok(products)
}

fn purchase_blocking(hwnd: isize, product_id: String) -> StoreResult<StorePurchase> {
    let store_id = resolve_store_id(&product_id)?;
    let context = context_for_window(hwnd)?;
    let result = context.RequestPurchaseAsync(&HSTRING::from(store_id.as_str())).map_err(|error| StoreError::Failed(error.message()))?.join().map_err(|error| StoreError::Failed(error.message()))?;
    let status = result.Status().map_err(|error| StoreError::Failed(error.message()))?;
    if status == StorePurchaseStatus::NotPurchased {
        return Err(StoreError::Cancelled);
    }
    if status != StorePurchaseStatus::Succeeded && status != StorePurchaseStatus::AlreadyPurchased {
        return Err(StoreError::Failed(format!("unexpected purchase status: {:?}", status)));
    }
    let license = fetch_license_json(&context)?;
    Ok(StorePurchase { store: STORE_KIND_MICROSOFT.to_string(), product_id: store_id, transaction_id: String::new(), receipt: license })
}

fn restore_blocking() -> StoreResult<Vec<StorePurchase>> {
    let context = default_context()?;
    let license = context.GetAppLicenseAsync().map_err(|error| StoreError::Failed(error.message()))?.join().map_err(|error| StoreError::Failed(error.message()))?;
    let addons = license.AddOnLicenses().map_err(|error| StoreError::Failed(error.message()))?;
    let mut purchases = Vec::new();
    for pair in &addons {
        let addon = pair.Value().map_err(|error| StoreError::Failed(error.message()))?;
        if !addon.IsActive().unwrap_or(false) {
            continue;
        }
        purchases.push(StorePurchase {
            store: STORE_KIND_MICROSOFT.to_string(),
            product_id: addon.SkuStoreId().map(|value| value.to_string()).unwrap_or_default(),
            transaction_id: String::new(),
            receipt: addon.ExtendedJsonData().map(|value| value.to_string()).unwrap_or_default(),
        });
    }
    Ok(purchases)
}

fn purchase_id_blocking(service_ticket: String, publisher_user_id: String) -> StoreResult<String> {
    let context = default_context()?;
    let key = context
        .GetCustomerPurchaseIdAsync(&HSTRING::from(service_ticket.as_str()), &HSTRING::from(publisher_user_id.as_str()))
        .map_err(|error| StoreError::Failed(error.message()))?
        .join()
        .map_err(|error| StoreError::Failed(error.message()))?;
    let key = key.to_string();
    if key.is_empty() {
        return Err(StoreError::Failed("Store ID key is empty".to_string()));
    }
    Ok(key)
}

fn fetch_license_json(context: &StoreContext) -> StoreResult<String> {
    let license = context.GetAppLicenseAsync().map_err(|error| StoreError::Failed(error.message()))?.join().map_err(|error| StoreError::Failed(error.message()))?;
    license.ExtendedJsonData().map(|value| value.to_string()).map_err(|error| StoreError::Failed(error.message()))
}
