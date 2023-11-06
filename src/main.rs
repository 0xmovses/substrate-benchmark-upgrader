use quote::quote;
use syn::{visit_mut::VisitMut, File, Item};
use std::fs;

struct RefactorBenchmark;

impl VisitMut for RefactorBenchmark {
    fn visit_file_mut(&mut self, file: &mut File) {
        let mut new_items = Vec::new();

        for item in file.items.drain(..) {
            match item {
                Item::Macro(i_macro) if i_macro.mac.path.is_ident("benchmarks") => {
                    let tokens = i_macro.mac.tokens;

                    let new_mod_tokens= quote! {
                        #[instance_benchmarks]
                        mod benchmarks {
                            #tokens
                        }
                    };

                    let new_mod = Item::Verbatim(new_mod_tokens);

                    new_items.push(new_mod);
                }
                _ => new_items.push(item),
            }
        }

        file.items = new_items;
    }
}

fn main() {
    let mut refactor_visitor = RefactorBenchmark;
    let file_contents = fs::read_to_string("src/fixtures/benchmark_v1.rs")
        .expect("Failed to read benchmark_v1.rs file");
    let mut file_ast: File = syn::parse_str(&file_contents).expect("Failed to parse file");
    refactor_visitor.visit_file_mut(&mut file_ast);

    let new_tokens = quote!(#file_ast);
    println!("{}", new_tokens);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refactor_benchmark() {
        let input = r#"
            benchmarks! {
                add_registrar {
                    // benchmark contents
                }
            }
        "#;

        let mut file_ast: File = syn::parse_str(input).expect("Failed to parse input");
        let mut refactor_visitor = RefactorBenchmark;
        refactor_visitor.visit_file_mut(&mut file_ast);

        let tokens = quote! { #file_ast };
        let output = tokens.to_string();
        let expected_output = "#[instance_benchmarks] mod benchmarks { add_registrar { } }".replace(" ", "");
        let sanitized_output = output.replace(" ", "");

        assert_eq!(
            sanitized_output, expected_output,
            "The output did not match the expected output. Actual: '{}', Expected: '{}'",
            output, expected_output
        );

        println!("File AST is: {}", output);
    }
}

