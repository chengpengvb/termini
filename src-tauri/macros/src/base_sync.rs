use quote::quote;
use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, ItemStruct, LitInt, LitStr, Token};

struct Args {
    entity: syn::Path,
    dao_field: Ident,
    skip_id: Option<LitStr>,
    sync_type: i8,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let entity: syn::Path = input.parse()?;
        let mut dao_field: Option<Ident> = None;
        let mut skip_id: Option<LitStr> = None;
        let mut sync_type: Option<i8> = None;

        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "dao" {
                let lit: LitStr = input.parse()?;
                dao_field = Some(Ident::new(&lit.value(), lit.span()));
                continue;
            }
            if key == "skip_id" {
                skip_id = Some(input.parse()?);
                continue;
            }
            if key == "sync" {
                sync_type = Some(input.parse::<LitInt>()?.base10_parse()?);
                continue;
            }
            return Err(syn::Error::new(key.span(), "unsupported #[base_sync(...)] key, expected: dao | skip_id | sync"));
        }

        let sync_type = sync_type.ok_or_else(|| syn::Error::new(input.span(), "#[base_sync(...)] missing required `sync = <i8>`"))?;
        Ok(Self { entity, dao_field: dao_field.unwrap_or_else(|| Ident::new("dao", proc_macro2::Span::call_site())), skip_id, sync_type })
    }
}

pub fn impl_tokens(ident: &Ident, _entity: &syn::Path, skip_id: Option<&LitStr>, sync_type: i8) -> proc_macro2::TokenStream {
    let skip_id_arg = match skip_id {
        Some(lit) => quote! { Some(#lit) },
        None => quote! { None },
    };

    quote! {
        #[::async_trait::async_trait]
        impl ::termini_base::service::service_base_sync::BaseSync for #ident {
            fn sync_type(&self) -> i8 {
                #sync_type
            }

            fn skip_id(&self) -> Option<&'static str> {
                #skip_id_arg
            }

            async fn export_current(&self) -> ::anyhow::Result<::serde_json::Value> {
                ::termini_base::service::service_base_sync::export_current(self).await
            }

            async fn import_cloud(&self, data: ::serde_json::Value) -> ::anyhow::Result<()> {
                ::termini_base::service::service_base_sync::import_cloud(self, data, #skip_id_arg).await
            }
        }

        ::inventory::submit! {
            ::termini_base::service::service_base_sync::SyncEntry {
                fetch: |app| {
                    use ::summer::plugin::ComponentRegistry;
                    app.try_get_component::<#ident>()
                        .ok()
                        .map(|comp| {
                            ::std::sync::Arc::new(comp)
                                as ::std::sync::Arc<dyn ::termini_base::service::service_base_sync::BaseSync>
                        })
                },
            }
        }
    }
}

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let Args { entity, dao_field: _dao_field, skip_id, sync_type } = parse_macro_input!(args as Args);
    let item = parse_macro_input!(input as ItemStruct);
    let ident = &item.ident;
    let sync_impl = impl_tokens(ident, &entity, skip_id.as_ref(), sync_type);

    let expanded = quote! {
        #item
        #sync_impl
    };

    expanded.into()
}
