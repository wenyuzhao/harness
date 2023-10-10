use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn bench(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStruct);
    let name = &input.ident;
    let result = quote! {
        #input

        fn main() {
            let mut bencher = ::harness::Bencher::new(file!(), #name::default());
            bencher.run();
        }
    };
    result.into()
}
