use quote::quote;
use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, parse_quote, Ident, ItemStruct, LitBool, LitInt, LitStr, Token};

struct Args {
    sort: i16,
    phase: i16,
    cache_all: bool,
    conn_field: Ident,
    custom_start: bool,
    entity: syn::Path,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let entity: syn::Path = input.parse()?;
        let mut sort: i16 = 200;
        let mut phase: i16 = 20;
        let mut conn_field: Option<Ident> = None;
        let mut cache_all: Option<bool> = None;
        let mut custom_start: bool = false;

        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "sort" {
                sort = input.parse::<LitInt>()?.base10_parse()?;
                continue;
            }
            if key == "phase" {
                phase = input.parse::<LitInt>()?.base10_parse()?;
                continue;
            }
            if key == "conn" {
                let lit: LitStr = input.parse()?;
                conn_field = Some(Ident::new(&lit.value(), lit.span()));
                continue;
            }
            if key == "cache_all" {
                cache_all = Some(input.parse::<LitBool>()?.value());
                continue;
            }
            if key == "custom_start" {
                custom_start = input.parse::<LitBool>()?.value();
                continue;
            }
            return Err(syn::Error::new(key.span(), "unsupported #[base_dao(...)] key, expected: sort | phase | conn | cache_all | custom_start"));
        }

        let cache_all = cache_all.ok_or_else(|| {
            syn::Error::new(input.span(), "#[base_dao(..)] missing required `cache_all = true|false`")
        })?;

        Ok(Self {
            entity,
            sort,
            phase,
            cache_all,
            custom_start,
            conn_field: conn_field
                .unwrap_or_else(|| Ident::new("data_conn_factory", proc_macro2::Span::call_site())),
        })
    }
}

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let Args { entity, sort, phase, conn_field, cache_all, custom_start } = parse_macro_input!(args as Args);
    let mut item = parse_macro_input!(input as ItemStruct);
    let ident = &item.ident;

    if conn_field == "data_conn_factory" {
        let has_conn = match &item.fields {
            syn::Fields::Named(named) => named.named.iter().any(|f| f.ident.as_ref().map(|i| i == &conn_field).unwrap_or(false)),
            _ => false,
        };

        if !has_conn {
            let named = match &mut item.fields {
                syn::Fields::Named(named) => named,
                _ => {
                    return syn::Error::new_spanned(&item.ident, "#[base_dao] only supports structs with named fields").to_compile_error().into();
                }
            };

            let injected_field: syn::Field = parse_quote! {
                #[inject(component)]
                data_conn_factory: ::termini_base::dao::conn_factory::data_conn_factory::DataConnFactory
            };
            named.named.insert(0, injected_field);
        }
    }

    if let syn::Fields::Named(named) = &mut item.fields {
        let cache_field: syn::Field = parse_quote! {
            #[inject(func = ::termini_base::dao::dao_base::new_cache_map::<<#entity as ::sea_orm::EntityTrait>::Model>())]
            __cache: ::std::sync::Arc<
                ::dashmap::DashMap<
                    ::std::string::String,
                    ::termini_core::model::dto::shared_ptr::Shared<<#entity as ::sea_orm::EntityTrait>::Model>,
                >,
            >
        };
        named.named.push(cache_field);
    }

    let base_app_impl = if custom_start {
        quote! {}
    } else {
        quote! {
            #[::async_trait::async_trait]
            impl ::termini_base::base::base_app::BaseApp for #ident {
                fn phase(&self) -> i16 {
                    #phase
                }

                fn name(&self) -> &'static str {
                    ::std::stringify!(#ident)
                }

                async fn start(&self, _ctx: &::termini_base::base::base_app::AppContext) -> ::anyhow::Result<()> {
                    <Self as ::termini_base::dao::dao_base::BaseDao>::init(self).await?;
                    Ok(())
                }
            }
        }
    };

    let expanded = quote! {
        #item

        #[::async_trait::async_trait]
        impl ::termini_base::dao::dao_base::BaseDao for #ident {
            type Entity = #entity;

            fn cache_all(&self) -> bool {
                #cache_all
            }

            fn cache_map(&self) -> &::dashmap::DashMap<::std::string::String, ::termini_core::model::dto::shared_ptr::Shared<<#entity as ::sea_orm::EntityTrait>::Model>> {
                &self.__cache
            }

            fn get_conn(&self) -> ::anyhow::Result<::sea_orm::DatabaseConnection> {
                Ok(self.#conn_field.conn())
            }
        }

        #base_app_impl

        ::inventory::submit! {
            ::termini_base::system::bootstrap::BootstrapEntry {
                sort: #sort,
                fetch: |app| {
                    use ::summer::plugin::ComponentRegistry;
                    app.try_get_component::<#ident>()
                        .ok()
                        .map(|comp| {
                            ::std::sync::Arc::new(comp)
                                as ::std::sync::Arc<dyn ::termini_base::base::base_app::BaseApp>
                        })
                },
            }
        }
    };

    expanded.into()
}
