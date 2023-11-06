use proc_macro2::{TokenStream, Span};
use quote::quote;
use syn::{visit_mut::VisitMut, File, Item, ItemMacro, parse2, Ident};

struct RefactorBenchmark;

impl VisitMut for RefactorBenchmark {
    fn visit_item_macro_mut(&mut self, i_macro: &mut ItemMacro) {
        if i_macro.mac.path.is_ident("benchmarks") {
            let tokens = i_macro.mac.tokens.clone();
            let mod_ident = Ident::new("benchmarks", Span::call_site());
            let new_tokens: TokenStream = quote! {
                #[instance_benchmarks]
                mod #mod_ident {
                    #tokens
                }
            };
            i_macro.mac.tokens = new_tokens;
            i_macro.mac.path = syn::Path::from(Ident::new("dummy", Span::call_site()));
        }
    }
}

fn main() {
    let mut file_ast: File = syn::parse_str(r#"
        benchmarks! {
            add_registrar {
                // ... (other code)
            }
        }
    "#).unwrap();

    let mut refactor_visitor = RefactorBenchmark;
    refactor_visitor.visit_file_mut(&mut file_ast);

    // Convert the modified AST back to a token stream (or to a string, if you want to output it as code).
    let new_tokens = quote!(#file_ast);
    println!("{}", new_tokens);
}
