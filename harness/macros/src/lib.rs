use darling::ast::NestedMeta;
use darling::{Error, FromMeta};
use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug, FromMeta)]
struct BenchMacroArgs {
    #[darling(default)]
    oneshot: bool,
    #[darling(default)]
    startup: Option<syn::Path>,
    #[darling(default)]
    teardown: Option<syn::Path>,
}

/// Annotation for the benchmark function.
///
/// The annotated function will be invoked for **N** iterations at a time in a loop.
///
/// Iterations **0** ~ **N-2** will be used for warm-up, and the last iteration (**N-1**) will be used for measurement.
///
/// Each iteration has three phases:
/// 1. **Prepare**: Prepare any data or resources needed for this iteration.
/// 2. **Timing**: Perform the actual work. This should be wrapped in a call to `bencher.time()`.
/// 3. **Release**: Clean up any data or resources, and perform any necessary result checks.
///
/// **Note:**: Each benchmark file should contain exactly one benchmark function.
///
/// # Example
///
/// ```rust
/// use harness::{bench, Bencher, black_box};
///
/// const LEN: usize = 10000000;
///
/// #[bench]
/// fn example(bencher: &Bencher) {
///     // Prepare the inputs
///     let mut list = black_box((0..1000).collect::<Vec<_>>());
///     // Actual work. For the last timing iteration only this part will be measured.
///     let result = bencher.time(|| {
///         // Do some work here
///         list.iter().sum::<usize>()
///     });
///     // Release the resources and check the result
///     assert_eq!(result, LEN * (LEN - 1) / 2)
/// }
/// ```
///
/// # Startup and Teardown
///
/// To run some extra code _once_ before and after the benchmark, please use the `startup` and `teardown` hooks in the attributes.
///
/// `startup` is called ones before all the iterations, and `teardown` is called once after all the iterations.
///
/// ```rust
/// use harness::{bench, Bencher, black_box};
///
/// fn example_startup() {
///     // TODO: Pre-benchmark initialization. e.g. Download data for the benchmark.
/// }
///
/// fn example_teardown() {
///     // TODO: After benchmark cleanups. e.g. Delete the downloaded data.
/// }
///
/// const LEN: usize = 10000000;
///
/// #[bench(startup = example_startup, teardown = example_teardown)]
/// fn example(bencher: &Bencher) {
///     // Prepare the inputs
///     let mut list = black_box((0..LEN).collect::<Vec<_>>());
///     // Actual work. For the last timing iteration only this part will be measured.
///     let result = bencher.time(|| {
///         // Do some work here
///         list.iter().sum::<usize>()
///     });
///     // Release the resources and check the result
///     assert_eq!(result, LEN * (LEN - 1) / 2)
/// }
/// ````
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
    let startup = &args.startup;
    let teardown = &args.teardown;
    let result = if args.oneshot {
        quote! {
            #input

            fn main() {
                #startup();
                ::harness::run(file!(), #name, true);
                #teardown();
            }
        }
    } else {
        quote! {
            #input

            fn main() {
                #startup();
                ::harness::run(file!(), #name, false);
                #teardown();
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
