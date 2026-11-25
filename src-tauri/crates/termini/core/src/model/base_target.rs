use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, Serialize)]
pub struct AiFieldMeta {
    pub hidden: bool,
    pub ty: &'static str,
    pub name: &'static str,
}

pub trait BaseTarget: Clone + Send + Sync + Serialize + DeserializeOwned + 'static {
    fn entity_type() -> i32;
    fn notify_entity_type() -> i32;
    fn set_id(&mut self, id: String);
    fn get_id(&self) -> Option<String>;
    fn encrypt_sensitive(&mut self) {}
    fn decrypt_sensitive(&mut self) {}
    fn ai_fields() -> &'static [AiFieldMeta] {
        &[]
    }

    fn for_ai(self) -> Self
    where
        Self: Sized,
    {
        self
    }
}

pub trait Redact {
    fn redact(self) -> Self;
}

pub trait EncryptField {
    fn encrypt_field(&mut self);
    fn decrypt_field(&mut self);
}

impl EncryptField for String {
    fn encrypt_field(&mut self) {
        *self = crate::util::crypto_util::encrypt_text(self);
    }

    fn decrypt_field(&mut self) {
        *self = crate::util::crypto_util::decrypt_text(self);
    }
}

impl<T: EncryptField> EncryptField for Option<T> {
    fn encrypt_field(&mut self) {
        if let Some(value) = self {
            value.encrypt_field();
        }
    }

    fn decrypt_field(&mut self) {
        if let Some(value) = self {
            value.decrypt_field();
        }
    }
}

impl EncryptField for serde_json::Value {
    fn encrypt_field(&mut self) {
        if self.is_null() {
            return;
        }
        if let serde_json::Value::String(text) = self {
            if text.starts_with(crate::util::crypto_util::ENC_PREFIX) {
                return;
            }
        }
        let text = serde_json::to_string(self).expect("failed to serialize encrypted field");
        *self = serde_json::Value::String(crate::util::crypto_util::encrypt_text(&text));
    }

    fn decrypt_field(&mut self) {
        let serde_json::Value::String(text) = &*self else {
            return;
        };
        if !text.starts_with(crate::util::crypto_util::ENC_PREFIX) {
            return;
        }
        let plain = crate::util::crypto_util::decrypt_text(text);
        *self = serde_json::from_str(&plain).expect("failed to deserialize decrypted field");
    }
}

pub fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    match Option::<T>::deserialize(deserializer)? {
        Some(value) => Ok(value),
        None => Ok(T::default()),
    }
}
