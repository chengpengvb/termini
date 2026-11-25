mod redact;
mod base_app;
mod base_dao;
mod base_sync;
mod base_target;
mod base_control;
mod base_service;
mod control_routes;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn base_app(args: TokenStream, input: TokenStream) -> TokenStream {
    base_app::expand(args, input)
}

#[proc_macro_attribute]
pub fn base_control(args: TokenStream, input: TokenStream) -> TokenStream {
    base_control::expand(args, input)
}

#[proc_macro_attribute]
pub fn base_dao(args: TokenStream, input: TokenStream) -> TokenStream {
    base_dao::expand(args, input)
}

#[proc_macro_attribute]
pub fn base_target(args: TokenStream, input: TokenStream) -> TokenStream {
    base_target::expand(args, input)
}

#[proc_macro_attribute]
pub fn base_service(args: TokenStream, input: TokenStream) -> TokenStream {
    base_service::expand(args, input)
}

#[proc_macro_attribute]
pub fn base_sync(args: TokenStream, input: TokenStream) -> TokenStream {
    base_sync::expand(args, input)
}

#[proc_macro_attribute]
pub fn control_routes(args: TokenStream, input: TokenStream) -> TokenStream {
    control_routes::expand(args, input)
}

#[proc_macro_derive(Redact, attributes(redact, redact_base))]
pub fn derive_redact(input: TokenStream) -> TokenStream {
    redact::expand(input)
}