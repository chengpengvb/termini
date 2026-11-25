use proc_macro::TokenStream;
use std::collections::HashSet;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, ItemStruct, LitInt, LitStr, Token, Type};

struct Args {
    sort: i16,
    phase: i16,
    entity: Type,
    prefix: String,
    skip: HashSet<String>,
}

const SKIPPABLE: &[&str] = &["select_all", "select", "insert", "insert_batch", "update", "delete"];

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let entity: Type = input.parse()?;
        let mut prefix: Option<String> = None;
        let mut sort: i16 = 450;
        let mut phase: i16 = 40;
        let mut skip: HashSet<String> = HashSet::new();

        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "prefix" {
                prefix = Some(input.parse::<LitStr>()?.value());
                continue;
            }
            if key == "sort" {
                sort = input.parse::<LitInt>()?.base10_parse()?;
                continue;
            }
            if key == "phase" {
                phase = input.parse::<LitInt>()?.base10_parse()?;
                continue;
            }
            if key == "skip" {
                let lit: LitStr = input.parse()?;
                for name in lit.value().split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    if !SKIPPABLE.contains(&name) {
                        return Err(syn::Error::new(lit.span(), format!("unsupported skip name `{name}`, expected one of: {}", SKIPPABLE.join(", "))));
                    }
                    skip.insert(name.to_string());
                }
                continue;
            }
            return Err(syn::Error::new(key.span(), "unsupported #[base_control(...)] key, expected: prefix | sort | phase | skip"));
        }

        let prefix = prefix.ok_or_else(|| syn::Error::new(input.span(), "#[base_control(..)] missing required `prefix = \"...\"`"))?;

        Ok(Self { entity, prefix, sort, phase, skip })
    }
}

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let Args { entity, prefix, sort, phase, skip } = parse_macro_input!(args as Args);
    let item = parse_macro_input!(input as ItemStruct);
    let ident = &item.ident;

    let path_select_all = format!("{}/select_all", prefix);
    let path_select = format!("{}/select/{{id}}", prefix);
    let path_insert = format!("{}/insert", prefix);
    let path_insert_batch = format!("{}/insert_batch", prefix);
    let path_update = format!("{}/update", prefix);
    let path_delete = format!("{}/delete/{{id}}", prefix);

    let fn_select_all = format_ident!("__{}_select_all", ident);
    let fn_select = format_ident!("__{}_select", ident);
    let fn_insert = format_ident!("__{}_insert", ident);
    let fn_insert_batch = format_ident!("__{}_insert_batch", ident);
    let fn_update = format_ident!("__{}_update", ident);
    let fn_delete = format_ident!("__{}_delete", ident);

    let select_all_route = if skip.contains("select_all") {
        quote! {}
    } else {
        quote! {
            #[allow(non_snake_case)]
            #[::summer_web::get(#path_select_all)]
            async fn #fn_select_all(
                ::summer_web::axum::Extension(__state): ::summer_web::axum::Extension<::summer_web::AppState>,
            ) -> ::termini_core::model::resp::resp_model::ResultModel<::std::vec::Vec<::termini_core::model::dto::shared_ptr::Shared<#entity>>> {
                use ::summer::plugin::ComponentRegistry;
                use ::termini_base::web::control_base::BaseControl;
                let __ctrl = __state.app.get_expect_component::<#ident>();
                __ctrl.select_all().await
            }
        }
    };

    let select_route = if skip.contains("select") {
        quote! {}
    } else {
        quote! {
            #[allow(non_snake_case)]
            #[::summer_web::get(#path_select)]
            async fn #fn_select(
                ::summer_web::axum::Extension(__state): ::summer_web::axum::Extension<::summer_web::AppState>,
                ::summer_web::axum::extract::Path(__id): ::summer_web::axum::extract::Path<::std::string::String>,
            ) -> ::termini_core::model::resp::resp_model::ResultModel<::std::option::Option<::termini_core::model::dto::shared_ptr::Shared<#entity>>> {
                use ::summer::plugin::ComponentRegistry;
                use ::termini_base::web::control_base::BaseControl;
                let __ctrl = __state.app.get_expect_component::<#ident>();
                __ctrl.select(__id).await
            }
        }
    };

    let insert_route = if skip.contains("insert") {
        quote! {}
    } else {
        quote! {
            #[allow(non_snake_case)]
            #[::summer_web::post(#path_insert)]
            async fn #fn_insert(
                ::summer_web::axum::Extension(__state): ::summer_web::axum::Extension<::summer_web::AppState>,
                ::summer_web::axum::Json(__body): ::summer_web::axum::Json<#entity>,
            ) -> ::termini_core::model::resp::resp_model::ResultModel<::termini_core::model::dto::shared_ptr::Shared<#entity>> {
                use ::summer::plugin::ComponentRegistry;
                use ::termini_base::web::control_base::BaseControl;
                let __ctrl = __state.app.get_expect_component::<#ident>();
                __ctrl.insert(__body).await
            }
        }
    };

    let insert_batch_route = if skip.contains("insert_batch") {
        quote! {}
    } else {
        quote! {
            #[allow(non_snake_case)]
            #[::summer_web::post(#path_insert_batch)]
            async fn #fn_insert_batch(
                ::summer_web::axum::Extension(__state): ::summer_web::axum::Extension<::summer_web::AppState>,
                ::summer_web::axum::Json(__body): ::summer_web::axum::Json<::std::vec::Vec<#entity>>,
            ) -> ::termini_core::model::resp::resp_model::ResultModel<::std::vec::Vec<::termini_core::model::dto::shared_ptr::Shared<#entity>>> {
                use ::summer::plugin::ComponentRegistry;
                use ::termini_base::web::control_base::BaseControl;
                let __ctrl = __state.app.get_expect_component::<#ident>();
                __ctrl.insert_batch(__body).await
            }
        }
    };

    let update_route = if skip.contains("update") {
        quote! {}
    } else {
        quote! {
            #[allow(non_snake_case)]
            #[::summer_web::post(#path_update)]
            async fn #fn_update(
                ::summer_web::axum::Extension(__state): ::summer_web::axum::Extension<::summer_web::AppState>,
                ::summer_web::axum::Json(__body): ::summer_web::axum::Json<#entity>,
            ) -> ::termini_core::model::resp::resp_model::ResultModel<::termini_core::model::dto::shared_ptr::Shared<#entity>> {
                use ::summer::plugin::ComponentRegistry;
                use ::termini_base::web::control_base::BaseControl;
                let __ctrl = __state.app.get_expect_component::<#ident>();
                __ctrl.update(__body).await
            }
        }
    };

    let delete_route = if skip.contains("delete") {
        quote! {}
    } else {
        quote! {
            #[allow(non_snake_case)]
            #[::summer_web::delete(#path_delete)]
            async fn #fn_delete(
                ::summer_web::axum::Extension(__state): ::summer_web::axum::Extension<::summer_web::AppState>,
                ::summer_web::axum::extract::Path(__id): ::summer_web::axum::extract::Path<::std::string::String>,
            ) -> ::termini_core::model::resp::resp_model::ResultModel<()> {
                use ::summer::plugin::ComponentRegistry;
                use ::termini_base::web::control_base::BaseControl;
                let __ctrl = __state.app.get_expect_component::<#ident>();
                __ctrl.delete_by_id(__id).await
            }
        }
    };

    let expanded = quote! {
        #item

        #[::async_trait::async_trait]
        impl ::termini_base::base::base_app::BaseApp for #ident {
            fn phase(&self) -> i16 {
                #phase
            }

            fn name(&self) -> &'static str {
                ::std::stringify!(#ident)
            }
        }

        #select_all_route
        #select_route
        #insert_route
        #insert_batch_route
        #update_route
        #delete_route

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
