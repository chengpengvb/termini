use quote::quote;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{parse_macro_input, parse_quote, Expr, ItemStruct, Lit, LitInt, LitStr, Meta, WhereClause};

struct AiFieldPlan {
    zero: bool,
    hidden: bool,
    ty: syn::Type,
    ident: syn::Ident,
    redact_base: Option<syn::Path>,
}

fn redact_base_type(field: &syn::Field) -> Option<syn::Path> {
    field.attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("redact_base") {
            return None;
        }
        let Meta::NameValue(nv) = &attr.meta else {
            return None;
        };
        let Expr::Lit(el) = &nv.value else {
            return None;
        };
        let Lit::Str(s) = &el.lit else {
            return None;
        };
        syn::parse_str::<syn::Path>(&s.value()).ok()
    })
}

pub fn expand(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut entity_type: Option<i32> = None;
    let mut notify_entity_type: Option<i32> = None;
    let mut explicit_table_name: Option<String> = None;
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("table") {
            let lit = meta.value()?.parse::<LitStr>()?;
            explicit_table_name = Some(lit.value());
            return Ok(());
        }
        if meta.path.is_ident("entity_type") {
            let lit = meta.value()?.parse::<LitInt>()?;
            entity_type = Some(lit.base10_parse()?);
            return Ok(());
        }
        if meta.path.is_ident("notify") {
            let lit = meta.value()?.parse::<LitInt>()?;
            notify_entity_type = Some(lit.base10_parse()?);
            return Ok(());
        }
        Err(meta.error("unsupported #[base_target(...)] key, expected: table | entity_type | notify"))
    });
    if let Err(err) = syn::parse::Parser::parse(parser, _args) {
        return err.to_compile_error().into();
    }
    let Some(entity_type) = entity_type else {
        return syn::Error::new(proc_macro2::Span::call_site(), "#[base_target] requires entity_type = <i32>").to_compile_error().into();
    };
    let notify_entity_type = notify_entity_type.unwrap_or(entity_type);

    let mut item = parse_macro_input!(input as ItemStruct);
    let ident = item.ident.clone();
    let table_name = explicit_table_name.unwrap_or_else(|| ident.to_string().to_lowercase());

    let named = match &mut item.fields {
        syn::Fields::Named(named) => named,
        _ => {
            return syn::Error::new_spanned(&item.ident, "#[base_target] only supports structs with named fields").to_compile_error().into();
        }
    };

    let has_id = named.named.iter().any(|f| f.ident.as_ref().map(|i| i == "id").unwrap_or(false));
    if !has_id {
        let id_field: syn::Field = parse_quote! {
            #[sea_orm(primary_key, auto_increment = false)]
            #[serde(default, deserialize_with = "termini_core::model::base_target::deserialize_null_default")]
            pub id: String
        };
        named.named.insert(0, id_field);
    }

    let mut encrypt_fields: Vec<syn::Ident> = Vec::new();
    let mut ai_fields: Vec<AiFieldPlan> = Vec::new();
    for field in named.named.iter_mut() {
        let fname = field.ident.clone().expect("field ident");
        let has_encrypt = field.attrs.iter().any(|a| a.path().is_ident("encrypt"));
        let has_redact = field.attrs.iter().any(|a| a.path().is_ident("redact"));
        let redact_base = redact_base_type(field);
        if has_encrypt {
            field.attrs.retain(|a| !a.path().is_ident("encrypt"));
            encrypt_fields.push(fname.clone());
        }
        let hidden = has_redact || (has_encrypt && redact_base.is_none());
        let zero = (has_redact || has_encrypt) && redact_base.is_none();
        ai_fields.push(AiFieldPlan { ident: fname, ty: field.ty.clone(), hidden, zero, redact_base });
    }
    let encrypt_impl = if encrypt_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            fn encrypt_sensitive(&mut self) {
                #( ::termini_core::model::base_target::EncryptField::encrypt_field(&mut self.#encrypt_fields); )*
            }

            fn decrypt_sensitive(&mut self) {
                #( ::termini_core::model::base_target::EncryptField::decrypt_field(&mut self.#encrypt_fields); )*
            }
        }
    };

    let field_metas: Vec<TokenStream2> = ai_fields
        .iter()
        .map(|field| {
            let name = field.ident.to_string();
            let ty = &field.ty;
            let hidden = field.hidden;
            quote! {
                ::termini_core::model::base_target::AiFieldMeta {
                    name: #name,
                    ty: stringify!(#ty),
                    hidden: #hidden,
                }
            }
        })
        .collect();

    let zero_idents: Vec<syn::Ident> = ai_fields.iter().filter(|field| field.zero).map(|field| field.ident.clone()).collect();
    let redact_base_updates: Vec<TokenStream2> = ai_fields
        .iter()
        .filter_map(|field| {
            let fname = &field.ident;
            let base_type = field.redact_base.as_ref()?;
            Some(quote! {
                if !result.#fname.is_null() {
                    let inner = ::serde_json::from_value::<#base_type>(result.#fname.clone()).expect("for_ai redact_base deserialize failed");
                    result.#fname = ::serde_json::to_value(::termini_core::model::base_target::Redact::redact(inner)).expect("for_ai redact_base serialize failed");
                }
            })
        })
        .collect();

    let for_ai_impl = quote! {
        fn ai_fields() -> &'static [::termini_core::model::base_target::AiFieldMeta] {
            &[ #(#field_metas),* ]
        }

        fn for_ai(self) -> Self {
            let mut result = self;
            #( result.#zero_idents = ::std::default::Default::default(); )*
            #(#redact_base_updates)*
            result
        }
    };

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut where_clause: WhereClause = where_clause.map(|w| parse_quote!(#w)).unwrap_or_else(|| parse_quote! { where });
    for param in item.generics.params.iter() {
        if let syn::GenericParam::Type(tp) = param {
            let ty_ident = &tp.ident;
            where_clause.predicates.push(parse_quote! {
                #ty_ident: ::std::clone::Clone + ::std::marker::Send + ::std::marker::Sync + ::serde::Serialize + ::serde::de::DeserializeOwned + 'static
            });
        }
    }

    let expanded = quote! {
        use ::sea_orm::entity::prelude::{
            EnumIter,
            DeriveColumn,
            DeriveEntity,
            DeriveRelation,
            PrimaryKeyTrait,
            DerivePrimaryKey,
            PrimaryKeyToColumn,
        };

        #[derive(::sea_orm::entity::prelude::DeriveEntityModel, std::fmt::Debug, std::cmp::PartialEq)]
        #[sea_orm(table_name = #table_name)]
        #item

        #[derive(Copy, Clone, std::fmt::Debug, ::sea_orm::entity::prelude::EnumIter, ::sea_orm::entity::prelude::DeriveRelation)]
        pub enum Relation {}

        impl ::sea_orm::entity::prelude::ActiveModelBehavior for ActiveModel {}

        impl #impl_generics ::termini_core::model::base_target::BaseTarget for #ident #ty_generics #where_clause {
            fn get_id(&self) -> Option<String> {
                if self.id.is_empty() {
                    None
                } else {
                    Some(self.id.clone())
                }
            }

            fn set_id(&mut self, id: String) {
                self.id = id;
            }

            fn entity_type() -> i32 {
                #entity_type
            }

            fn notify_entity_type() -> i32 {
                #notify_entity_type
            }

            #encrypt_impl

            #for_ai_impl
        }
    };
    expanded.into()
}
