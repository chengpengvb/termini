use serde::Deserialize;
use store_core::{StorePurchase, STORE_KIND_GOOGLE};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GoogleTxn {
    pub product_id: String,
    pub purchase_token: String,
}

pub(crate) fn store_purchase_from_txn(parsed: GoogleTxn) -> StorePurchase {
    StorePurchase { store: STORE_KIND_GOOGLE.to_string(), product_id: parsed.product_id, transaction_id: parsed.purchase_token.clone(), receipt: parsed.purchase_token }
}

#[cfg(test)]
mod tests {
    use super::{store_purchase_from_txn, GoogleTxn};

    #[test]
    fn play_json_array_maps_token_to_transaction_and_receipt() {
        let parsed: Vec<GoogleTxn> = serde_json::from_str(r#"[{"productId":"termini.pro.monthly","purchaseToken":"tok.1"},{"productId":"termini.pro.yearly","purchaseToken":"tok.2"}]"#).unwrap();
        assert_eq!(parsed.len(), 2);
        let purchase = store_purchase_from_txn(parsed.into_iter().next().unwrap());
        assert_eq!(purchase.store, "google");
        assert_eq!(purchase.product_id, "termini.pro.monthly");
        assert_eq!(purchase.transaction_id, "tok.1");
        assert_eq!(purchase.receipt, "tok.1");
    }

    #[test]
    fn empty_array_is_empty() {
        let parsed: Vec<GoogleTxn> = serde_json::from_str("[]").unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn missing_purchase_token_is_rejected() {
        assert!(serde_json::from_str::<Vec<GoogleTxn>>(r#"[{"productId":"termini.pro.monthly"}]"#).is_err());
    }

    #[test]
    fn missing_product_id_is_rejected() {
        assert!(serde_json::from_str::<Vec<GoogleTxn>>(r#"[{"purchaseToken":"tok.1"}]"#).is_err());
    }
}
