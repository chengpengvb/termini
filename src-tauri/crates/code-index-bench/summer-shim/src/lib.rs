#[macro_export]
#[doc(hidden)]
macro_rules! __summer_warn {
    ($($arg:tt)*) => { eprintln!($($arg)*) };
}

pub mod tracing {
    pub use crate::__summer_warn as warn;
}
