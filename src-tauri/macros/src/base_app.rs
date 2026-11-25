use quote::quote;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemStruct, LitInt};

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut sort: i16 = 200;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("sort") {
            sort = meta.value()?.parse::<LitInt>()?.base10_parse()?;
            return Ok(());
        }
        Err(meta.error("unsupported #[base_app(...)] key, expected: sort = <i16>"))
    });
    if let Err(err) = syn::parse::Parser::parse(parser, args) {
        return err.to_compile_error().into();
    }

    let item = parse_macro_input!(input as ItemStruct);
    let ident = &item.ident;

    let expanded = quote! {
        #item

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
