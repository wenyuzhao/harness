use darling::ast::NestedMeta;
use darling::{Error, FromMeta};
use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug, FromMeta)]
struct BenchMacroArgs {
    #[darling(default)]
    single_shot: bool,
}

/// Annotation for the harness benchmark struct.
#[proc_macro_attribute]
pub fn bench(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let name = &input.sig.ident;
    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };
    let args = match BenchMacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };
    let result = if args.single_shot {
        quote! {
            #input

            fn main() {
                ::harness::run(file!(), #name, true);
            }
        }
    } else {
        quote! {
            #input

            fn main() {
                ::harness::run(file!(), #name, false);
            }
        }
    };
    result.into()
}

/// Annotation for the harness probe struct.
#[proc_macro_attribute]
pub fn probe(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStruct);
    let name = &input.ident;
    let result = quote! {
        #input

        #[no_mangle]
        pub extern "C" fn harness_register_probe(probes: &mut ProbeManager) {
            probes.register(Box::new(#name::default()));
        }
    };
    result.into()
}
