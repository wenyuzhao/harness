use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn entry(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let name = &input.sig.ident;
    let result = quote! {
        fn main() {
            #input
            let mut bencher = ::harness::Bencher::new(file!());
            #name(&mut bencher);
            bencher.run();
        }
    };
    result.into()
}
