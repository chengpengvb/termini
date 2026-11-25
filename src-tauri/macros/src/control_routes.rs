use proc_macro::TokenStream;
use quote::{format_ident, quote};
use proc_macro2::TokenStream as TokenStream2;
use syn::parse::{Parse, ParseStream, Parser};
use syn::{parse_macro_input, FnArg, Ident, ImplItem, ItemImpl, LitStr, Token, Type};

const HTTP_METHOD_IDENTS: &[&str] = &["get", "post", "put", "delete", "patch", "head", "options", "trace", "route", "routes"];

fn is_http_route_attr(attr: &syn::Attribute) -> bool {
    if let Some(seg) = attr.path().segments.last() {
        let name = seg.ident.to_string();
        return HTTP_METHOD_IDENTS.iter().any(|m| *m == name);
    }
    false
}

fn type_ident(ty: &Type) -> Option<syn::Ident> {
    if let Type::Path(tp) = ty {
        return tp.path.segments.last().map(|s| s.ident.clone());
    }
    None
}

struct Args {
    prefix: Option<String>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut prefix: Option<String> = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "prefix" {
                prefix = Some(input.parse::<LitStr>()?.value());
            } else {
                return Err(syn::Error::new(key.span(), "unsupported #[control_routes(...)] key, expected: prefix = \"...\""));
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { prefix })
    }
}

fn rewrite_route_attr_path(attr: &mut syn::Attribute, prefix: &str) -> syn::Result<()> {
    let list = match &mut attr.meta {
        syn::Meta::List(list) => list,
        _ => {
            return Err(syn::Error::new_spanned(attr, "route attribute must be of form #[method(\"/path\", ...)]"));
        }
    };
    let tokens = list.tokens.clone();
    let parser = move |input: ParseStream| -> syn::Result<TokenStream2> {
        let lit: LitStr = input.parse()?;
        let combined = format!("{}{}", prefix, lit.value());
        let new_lit = LitStr::new(&combined, lit.span());
        let rest: TokenStream2 = input.parse()?;
        Ok(quote!(#new_lit #rest))
    };
    let new_tokens = Parser::parse2(parser, tokens)?;
    list.tokens = new_tokens;
    Ok(())
}

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let parsed_args = parse_macro_input!(args as Args);
    let prefix = parsed_args.prefix.unwrap_or_default();
    let mut item = parse_macro_input!(input as ItemImpl);

    let owner_ident = match type_ident(&item.self_ty) {
        Some(id) => id,
        None => {
            return syn::Error::new_spanned(&item.self_ty, "#[control_routes] only supports `impl Type { ... }` with a named type").to_compile_error().into();
        }
    };

    let mut free_fns: Vec<proc_macro2::TokenStream> = Vec::new();

    for impl_item in item.items.iter_mut() {
        let ImplItem::Fn(method) = impl_item else { continue };

        let mut route_attrs: Vec<syn::Attribute> = method.attrs.iter().filter(|a| is_http_route_attr(a)).cloned().collect();
        if route_attrs.is_empty() {
            continue;
        }

        if !prefix.is_empty() {
            for ra in route_attrs.iter_mut() {
                if let Err(e) = rewrite_route_attr_path(ra, &prefix) {
                    return e.to_compile_error().into();
                }
            }
        }

        method.attrs.retain(|a| !is_http_route_attr(a));

        let method_ident = method.sig.ident.clone();
        let asyncness = method.sig.asyncness;
        let output = &method.sig.output;

        let mut typed_arg_decls: Vec<proc_macro2::TokenStream> = Vec::new();
        let mut typed_arg_idents: Vec<syn::Ident> = Vec::new();
        for (idx, arg) in method.sig.inputs.iter().enumerate() {
            let FnArg::Typed(pat_ty) = arg else { continue };
            let arg_ident = format_ident!("__termini_arg{}", idx);
            let ty = &pat_ty.ty;
            typed_arg_decls.push(quote! { #arg_ident: #ty });
            typed_arg_idents.push(arg_ident);
        }

        let wrapper_ident = format_ident!("__{}_{}", owner_ident, method_ident);

        free_fns.push(quote! {
            #[allow(non_snake_case)]
            #(#route_attrs)*
            #asyncness fn #wrapper_ident(
                ::summer_web::axum::Extension(__termini_state): ::summer_web::axum::Extension<::summer_web::AppState>,
                #(#typed_arg_decls,)*
            ) #output {
                use ::summer::plugin::ComponentRegistry;
                let __termini_ctrl = __termini_state.app.get_expect_component::<#owner_ident>();
                __termini_ctrl.#method_ident(#(#typed_arg_idents,)*).await
            }
        });
    }

    let expanded = quote! {
        #item

        #(#free_fns)*
    };
    expanded.into()
}
