extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn upgrade_benchmark(input: TokenStream) -> TokenStream {
    input
}