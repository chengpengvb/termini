use quote::quote;
use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, ItemStruct, LitBool, LitInt, LitStr, Token};

struct Args {
    entity: syn::Path,
    dao_field: Ident,
    sync_type: Option<i8>,
    skip_id: Option<LitStr>,
    emit_service: bool,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut emit_service = true;
        let mut sync_type: Option<i8> = None;
        let entity: syn::Path = input.parse()?;
        let mut dao_field: Option<Ident> = None;
        let mut skip_id: Option<LitStr> = None;

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
            if key == "service" {
                emit_service = input.parse::<LitBool>()?.value();
                continue;
            }
            return Err(syn::Error::new(key.span(), "unsupported #[base_service(...)] key, expected: dao | sync | skip_id | service"));
        }

        Ok(Self { entity, dao_field: dao_field.unwrap_or_else(|| Ident::new("dao", proc_macro2::Span::call_site())), sync_type, skip_id, emit_service })
    }
}

fn entity_table_name(entity: &syn::Path) -> syn::Result<String> {
    let names: Vec<String> = entity.segments.iter().map(|segment| segment.ident.to_string()).collect();
    if names.last().map(|name| name.as_str()) != Some("Entity") || names.len() < 2 {
        return Err(syn::Error::new_spanned(entity, "#[base_service] entity path must be like foo::Entity"));
    }
    Ok(names[names.len() - 2].clone())
}

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let Args { entity, dao_field, sync_type, skip_id, mut emit_service } = parse_macro_input!(args as Args);
    if sync_type.is_some() {
        emit_service = false;
    }
    let table_name = match entity_table_name(&entity) {
        Ok(name) => name,
        Err(err) => return err.to_compile_error().into(),
    };
    let item = parse_macro_input!(input as ItemStruct);
    let ident = &item.ident;
    let sync_impl = match sync_type {
        Some(sync_type) => crate::base_sync::impl_tokens(ident, &entity, skip_id.as_ref(), sync_type),
        None => quote! {},
    };

    let base_service_impl = if emit_service {
        quote! {
            #[::async_trait::async_trait]
            impl ::termini_base::service::service_base::BaseService for #ident {
                type Entity = #entity;

                fn get_dao(
                    &self,
                ) -> ::anyhow::Result<
                    ::std::boxed::Box<
                        dyn ::termini_base::dao::dao_base::BaseDao<Entity = Self::Entity>,
                    >,
                > {
                    Ok(::std::boxed::Box::new(self.#dao_field.clone()))
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #item

        #base_service_impl

        #[::async_trait::async_trait]
        impl ::termini_base::service::service_ai_crud::AiCrud for #ident {
            fn entity(&self) -> &'static str {
                #table_name
            }

            fn notify_entity_type(&self) -> i32 {
                <<#entity as ::sea_orm::EntityTrait>::Model as ::termini_core::model::base_target::BaseTarget>::notify_entity_type()
            }

            fn fields(&self) -> &'static [::termini_core::model::base_target::AiFieldMeta] {
                <<#entity as ::sea_orm::EntityTrait>::Model as ::termini_core::model::base_target::BaseTarget>::ai_fields()
            }

            async fn list(&self, offset: usize, limit: usize, keyword: Option<String>) -> ::anyhow::Result<::serde_json::Value> {
                let rows = <Self as ::termini_base::service::service_base::BaseService>::find_all(self).await?;
                let mut values = ::std::vec::Vec::with_capacity(rows.len());
                for row in rows {
                    let model: <#entity as ::sea_orm::EntityTrait>::Model = row.read().clone();
                    values.push(::termini_base::service::service_ai_crud::to_ai_row(model)?);
                }
                Ok(::termini_base::service::service_ai_crud::page_values(values, offset, limit, keyword.as_deref()))
            }

            async fn get(&self, id: String) -> ::anyhow::Result<::serde_json::Value> {
                let Some(row) = <Self as ::termini_base::service::service_base::BaseService>::find_by_id(self, id.clone()).await? else {
                    ::anyhow::bail!("{} not found: {}", #table_name, id);
                };
                let model: <#entity as ::sea_orm::EntityTrait>::Model = row.read().clone();
                ::termini_base::service::service_ai_crud::to_ai_row(model)
            }

            async fn insert(&self, row: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
                let model: <#entity as ::sea_orm::EntityTrait>::Model = ::serde_json::from_value(row)?;
                let saved = <Self as ::termini_base::service::service_base::BaseService>::insert(self, model).await?;
                let saved_model: <#entity as ::sea_orm::EntityTrait>::Model = saved.read().clone();
                ::termini_base::service::service_ai_crud::to_ai_row(saved_model)
            }

            async fn insert_many(&self, rows: Vec<::serde_json::Value>) -> ::anyhow::Result<::serde_json::Value> {
                if rows.is_empty() {
                    ::anyhow::bail!("{} insert rows is empty", #table_name);
                }
                let mut models = ::std::vec::Vec::with_capacity(rows.len());
                for row in rows {
                    models.push(::serde_json::from_value(row)?);
                }
                let saved = <Self as ::termini_base::service::service_base::BaseService>::insert_batch(self, models).await?;
                let mut ids = ::std::vec::Vec::with_capacity(saved.len());
                for item in saved {
                    let model: <#entity as ::sea_orm::EntityTrait>::Model = item.read().clone();
                    let Some(id) = ::termini_core::model::base_target::BaseTarget::get_id(&model) else {
                        ::anyhow::bail!("{} insert got empty id", #table_name);
                    };
                    ids.push(id);
                }
                Ok(::serde_json::json!({ "inserted": ids.len(), "ids": ids }))
            }

            async fn update(&self, row: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
                let Some(id) = row.get("id").and_then(|value| value.as_str()).map(|value| value.to_string()) else {
                    ::anyhow::bail!("{} update requires id", #table_name);
                };
                let Some(existing) = <Self as ::termini_base::service::service_base::BaseService>::find_by_id(self, id.clone()).await? else {
                    ::anyhow::bail!("{} not found: {}", #table_name, id);
                };
                let hidden = ::termini_base::service::service_ai_crud::hidden_names::<<#entity as ::sea_orm::EntityTrait>::Model>();
                let current = {
                    let guard = existing.read();
                    ::serde_json::to_value(&*guard)?
                };
                let merged = ::termini_base::service::service_ai_crud::merge_row(current, row, &hidden)?;
                let model: <#entity as ::sea_orm::EntityTrait>::Model = ::serde_json::from_value(merged)?;
                let saved = <Self as ::termini_base::service::service_base::BaseService>::update(self, model).await?;
                let saved_model: <#entity as ::sea_orm::EntityTrait>::Model = saved.read().clone();
                ::termini_base::service::service_ai_crud::to_ai_row(saved_model)
            }

            async fn delete(&self, id: String) -> ::anyhow::Result<::serde_json::Value> {
                let deleted = <Self as ::termini_base::service::service_base::BaseService>::delete_by_id(self, id.clone()).await?;
                if deleted == 0 {
                    ::anyhow::bail!("{} not found: {}", #table_name, id);
                }
                Ok(::serde_json::json!({ "deleted": deleted }))
            }

            async fn delete_many(&self, ids: Vec<String>) -> ::anyhow::Result<::serde_json::Value> {
                if ids.is_empty() {
                    ::anyhow::bail!("{} delete ids is empty", #table_name);
                }
                let mut deleted = 0u64;
                for id in ids {
                    let n = <Self as ::termini_base::service::service_base::BaseService>::delete_by_id(self, id.clone()).await?;
                    if n == 0 {
                        ::anyhow::bail!("{} not found: {}", #table_name, id);
                    }
                    deleted += n;
                }
                Ok(::serde_json::json!({ "deleted": deleted }))
            }
        }

        ::inventory::submit! {
            ::termini_base::service::service_ai_crud::AiCrudEntry {
                fetch: |app| {
                    use ::summer::plugin::ComponentRegistry;
                    app.try_get_component::<#ident>()
                        .ok()
                        .map(|comp| {
                            ::std::sync::Arc::new(comp)
                                as ::std::sync::Arc<dyn ::termini_base::service::service_ai_crud::AiCrud>
                        })
                },
            }
        }

        #sync_impl
    };

    expanded.into()
}
