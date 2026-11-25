use quote::quote;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{parse_macro_input, Expr, Fields, ItemStruct, Lit, Meta};

pub fn expand(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let named = match &item.fields {
        Fields::Named(n) => n,
        _ => {
            return syn::Error::new_spanned(ident, "#[derive(Redact)] only supports structs with named fields").to_compile_error().into();
        }
    };

    let mut field_updates: Vec<TokenStream2> = Vec::new();

    for field in &named.named {
        let fname = field.ident.as_ref().unwrap();

        let is_redact = field.attrs.iter().any(|a| a.path().is_ident("redact"));

        let redact_base_type: Option<syn::Path> = field.attrs.iter().find_map(|a| {
            if !a.path().is_ident("redact_base") {
                return None;
            }
            if let Meta::NameValue(nv) = &a.meta {
                if let Expr::Lit(el) = &nv.value {
                    if let Lit::Str(s) = &el.lit {
                        return syn::parse_str::<syn::Path>(&s.value()).ok();
                    }
                }
            }
            None
        });

        if is_redact {
            field_updates.push(quote! {
                result.#fname = ::std::default::Default::default();
            });
        } else if let Some(base_type) = redact_base_type {
            field_updates.push(quote! {
                if let ::std::result::Result::Ok(inner) = ::serde_json::from_value::<#base_type>(result.#fname.clone()) {
                    if let ::std::result::Result::Ok(safe_val) = ::serde_json::to_value(::termini_core::model::base_target::Redact::redact(inner)) {
                        result.#fname = safe_val;
                    }
                }
            });
        }
    }

    let expanded = quote! {
        impl #impl_generics ::termini_core::model::base_target::Redact for #ident #ty_generics #where_clause {
            fn redact(self) -> Self {
                let mut result = self;
                #(#field_updates)*
                result
            }
        }
    };
    expanded.into()
}
