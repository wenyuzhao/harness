use proc_macro::TokenStream;
use quote::quote;

/// Annotation for the harness benchmark struct.
#[proc_macro_attribute]
pub fn bench(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let name = &input.sig.ident;
    let result = quote! {
        #input

        fn main() {
            ::harness::run(file!(), #name);
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
