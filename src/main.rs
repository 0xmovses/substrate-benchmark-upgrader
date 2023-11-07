use syn::{visit_mut::VisitMut, parse2, Item, punctuated::Punctuated, parse::{ParseStream, Parse, Result}, token::Semi, ItemMacro, ItemMod, Block, Stmt, Attribute, Ident, File, Token, ItemFn, braced, parse_quote, Visibility, Signature, ReturnType, Generics};
use quote::{quote, ToTokens, format_ident};
use proc_macro2::{Span, TokenStream, TokenTree};
use std::{fs, iter::Peekable};
use regex;
use syn::parse::Parser;

struct Items(Vec<Item>);

impl Parse for Items {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(Items(items))
    }
}

struct BenchmarkFunction {
    name: Ident,
    block: TokenStream,
}

impl Parse for BenchmarkFunction {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let block = if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            // capture the content of the block as TokenStream
            content.cursor().token_stream()
        } else {
            return Err(input.error("Expected a block for the benchmark function"));
        };

        Ok(BenchmarkFunction { name, block })
    }
}
struct RefactorBenchmark;

impl VisitMut for RefactorBenchmark {
    fn visit_file_mut(&mut self, file: &mut File) {
        let mut new_items = Vec::new();
        let mut other_items = Vec::new();

        for item in file.items.drain(..) {
            match item {
                Item::Macro(i_macro) if i_macro.mac.path.is_ident("benchmarks") => {
                    let content = i_macro.mac.tokens.clone();
                    let parsed_functions= parse_benchmark_functions(content);

                    // Iterate over the parsed benchmark functions and transform them.
                    for function in parsed_functions.into_iter() {
                        // Create a new Rust function item with the #[benchmark] attribute.
                        let attrs: Vec<Attribute> = vec![parse_quote!(#[benchmark])];
                        // Construct a raw block from the captured TokenStream
                        let block = Block {
                            brace_token: syn::token::Brace::default(),
                            stmts: vec![Stmt::Item(Item::Verbatim(function.block))], // Insert the raw TokenStream
                        };
                        let new_fn = Item::Fn(ItemFn {
                            attrs,
                            vis: Visibility::Inherited,
                            sig: Signature {
                                constness: None,
                                asyncness: None,
                                unsafety: None,
                                abi: None,
                                fn_token: Default::default(),
                                ident: function.name,
                                generics: Default::default(),
                                paren_token: Default::default(),
                                inputs: Default::default(),
                                variadic: None,
                                output: ReturnType::Default,
                            },
                            block: Box::new(block),
                        });
                        new_items.push(new_fn);
                    }
                },
                _ => new_items.push(item),
            }
        }

        // Create a new mod with the transformed benchmark functions.
            let new_mod_tokens = quote! {
                #[instance_benchmarks]
                mod benchmarks {
                    #(#new_items)*
                }
            };
            // Push the new module to the list of items.
            let new_mod = Item::Verbatim(new_mod_tokens);
            other_items.push(new_mod);

        // Update the items in the file with the new items.
        file.items = new_items;
    }
}

// Assuming the structure of each benchmark function is known:
// identifier { ... }: _<...>(...) verify { ... }

fn parse_benchmark_functions(input: TokenStream) -> Vec<BenchmarkFunction> {
    let mut functions = Vec::new();
    let mut iter = input.into_iter().peekable();

    while let Some(token) = iter.next() {
        if let TokenTree::Ident(ident) = token {
            // We have found an identifier, expect a block next.
            if let Some(TokenTree::Group(group)) = iter.next() {
                if group.delimiter() == proc_macro2::Delimiter::Brace {
                    // We've found the block.
                    functions.push(BenchmarkFunction {
                        name: ident,
                        block: group.stream(),
                    });
                }
            }
        }
    }

    functions
}

fn main() {
    let mut benchmark = RefactorBenchmark;
    let file_contents = fs::read_to_string("src/fixtures/benchmark_v1.rs")
        .expect("Failed to read benchmark_v1.rs file");
    let mut file_ast: File = syn::parse_str(&file_contents).expect("Failed to parse file");
    benchmark.visit_file_mut(&mut file_ast);
    let tokens = quote! { #file_ast };
    println!("File AST is: {}", tokens.to_string());

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

