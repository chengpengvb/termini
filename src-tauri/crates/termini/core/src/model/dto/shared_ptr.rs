use std::sync::Arc;
use serde::{Deserialize, Serialize};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone)]
pub struct Shared<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> Shared<T> {
    pub fn new(v: T) -> Self {
        Self { inner: Arc::new(RwLock::new(v)) }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.inner.write()
    }
}

impl<T: Serialize> Serialize for Shared<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.read().serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Shared<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(Self::new(value))
    }
}
