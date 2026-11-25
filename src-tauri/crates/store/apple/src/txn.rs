use serde::Deserialize;
use store_core::{StorePurchase, STORE_KIND_APPLE};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppleTxn {
    pub product_id: String,
    pub original_transaction_id: String,
    pub jws: String,
}

pub(crate) fn store_purchase_from_txn(parsed: AppleTxn) -> StorePurchase {
    StorePurchase { store: STORE_KIND_APPLE.to_string(), product_id: parsed.product_id, transaction_id: parsed.original_transaction_id, receipt: parsed.jws }
}

#[cfg(test)]
mod tests {
    use super::{store_purchase_from_txn, AppleTxn};

    #[test]
    fn storekit_json_maps_original_id_and_jws() {
        let parsed: AppleTxn = serde_json::from_str(r#"{"productId":"termini.pro.monthly","originalTransactionId":"1","jws":"abc.def.ghi"}"#).unwrap();
        let purchase = store_purchase_from_txn(parsed);
        assert_eq!(purchase.store, "apple");
        assert_eq!(purchase.product_id, "termini.pro.monthly");
        assert_eq!(purchase.transaction_id, "1");
        assert_eq!(purchase.receipt, "abc.def.ghi");
    }

    #[test]
    fn plain_transaction_id_is_rejected() {
        assert!(serde_json::from_str::<AppleTxn>("900000123").is_err());
    }
}
