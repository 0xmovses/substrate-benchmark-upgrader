mod parser;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use regex;
use std::{fmt, fs, iter::Peekable, result::Result};
use std::fmt::{Display, Formatter};
use syn::parse::Parser;
use syn::{braced, parse::{Parse, ParseStream, Result as ParseResult}, parse2, parse_quote, punctuated::Punctuated, token::Semi, visit_mut::VisitMut, Attribute, Block, Expr, File, Generics, Ident, Item, ItemFn, ItemMacro, ItemMod, ReturnType, Signature, Stmt, Token, Visibility, FnArg};

enum RangeEndKind {
    Number(u8),
    Expression(Expr), // check, the DSL might not be valid syn::Expr ?
}

impl ToTokens for RangeEndKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            RangeEndKind::Number(n) => {
                let n = syn::LitInt::new(&n.to_string(), Span::call_site());
                n.to_tokens(tokens);
            }
            RangeEndKind::Expression(expr) => expr.to_tokens(tokens),
        }
    }
}

struct BenchmarkParameter {
    name: Ident,
    range_start: u8,
    range_end: RangeEndKind,
}

impl Display for BenchmarkParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let range_end_tokens = match &self.range_end {
            RangeEndKind::Number(n) => n.to_string(),
            RangeEndKind::Expression(expr) => {
                let expr_tokens = quote! { #expr };
                expr_tokens.to_string()
            }
        };

        // This assumes that the `range_start` is the start and `range_end_tokens` is the end of the `Linear` range.
        write!(
            f,
            "{}: Linear<{}, {{ {} }}>",
            self.name,
            self.range_start,
            range_end_tokens
        )
    }
}

impl ToTokens for BenchmarkParameter {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let range_start = &self.range_start;
        let range_end = &self.range_end;
        tokens.extend(quote! {
            for #name in #range_start..#range_end {
                // benchmark contents
            }
        });
    }
}
struct Items(Vec<Item>);

impl Parse for Items {
    fn parse(input: ParseStream) -> ParseResult<Self> {
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
    fn parse(input: ParseStream) -> ParseResult<Self> {
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
                    let parsed_functions = parse_benchmark_functions(content.clone());

                    // Collect the transformed benchmark functions
                    let mut transformed_functions = Vec::new();
                    for function in parsed_functions.into_iter() {
                        let mut parsed_params = Vec::new();
                        match parse_parameters(function.block.clone()) {
                            Ok(params) => parsed_params.extend(params),
                            Err(e) => eprintln!("warning: {} ", e),
                        }

                        let fn_args: Punctuated<FnArg, Token![,]> = parsed_params
                            .into_iter()
                            .map(|param| {
                                syn::parse_str::<FnArg>(&format!("{}", param)).expect("Failed to parse parameter")
                            })
                            .collect();

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
                                inputs: fn_args,
                                variadic: None,
                                output: ReturnType::Default,
                            },
                            block: Box::new(block),
                        });
                        transformed_functions.push(new_fn);
                    }
                    // Create a new mod item with the transformed functions
                    let new_mod_tokens = quote! {
                        #[instance_benchmarks]
                        mod benchmarks {
                            #(#transformed_functions)*
                        }
                    };

                    new_items.push(Item::Verbatim(new_mod_tokens));
                }
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

fn parse_parameters(input: TokenStream) -> Result<Vec<BenchmarkParameter>, String> {
    let mut iter = input.into_iter();
    let mut params = Vec::new();

    while let Some(token) = iter.next() {
        match token {
            TokenTree::Ident(ident) if ident == "let" => {
                if let Some(TokenTree::Ident(name)) = iter.next() {
                    // Look for the 'in' keyword
                    match iter.next() {
                        Some(TokenTree::Ident(ident)) if ident == "in" => {
                            // Parse range start...
                            let range_start_token = iter.next().ok_or("Expected a range start")?;
                            let range_start = token_tree_to_u8(range_start_token)?;

                            // Parse '..' and range end...
                            iter.next().ok_or("Expected '..' after range start")?;
                            iter.next().ok_or("Expected '..' after range start")?;
                            let range_end = parse_range_end(&mut iter)?;

                            // Now skip all tokens until we find the '=>' operator
                            while let Some(token) = iter.next() {
                                match token {
                                    TokenTree::Punct(ref punct) if punct.as_char() == '>' => {
                                        // Found '=>', now look for ';'
                                        while let Some(token) = iter.next() {
                                            match token {
                                                TokenTree::Punct(ref punct) if punct.as_char() == ';' => {
                                                    // Found ';', terminate the parameter declaration
                                                    break;
                                                },
                                                _ => continue, // Skip other tokens
                                            }
                                        }
                                        break; // Break from the outer loop once ';' is found
                                    },
                                    _ => continue, // Skip other tokens before '=>'
                                }
                            }

                            // Push the parsed parameter
                            params.push(BenchmarkParameter {
                                name,
                                range_start,
                                range_end,
                            });
                        },
                        Some(_) | None => return Err("Expected 'in' keyword".to_string()),
                    }
                } else {
                    return Err("Expected parameter name after 'let'".to_string());
                }
            },
            _ => continue, // Ignore other tokens
        }
    }

    for param in &params {
        let range_end_tokens = quote! { #param.range_end };
        //name range_start range_end
        println!("{} {}", param.name, param.range_start);
        println!("{} {}", param.name, range_end_tokens.to_string());
    }
    Ok(params)
}


// A helper function to try to parse a TokenTree as a u8
fn token_tree_to_u8(token: TokenTree) -> Result<u8, &'static str> {
    if let TokenTree::Literal(lit) = token {
        lit.to_string()
            .parse::<u8>()
            .map_err(|_| "Unable to parse as u8")
    } else {
        Err("Expected a number")
    }
}

// A helper function to parse an expression from a group or an ident
fn parse_range_end(
    iter: &mut impl Iterator<Item = TokenTree>,
) -> Result<RangeEndKind, &'static str> {
    let token = iter.next().ok_or("Expected a token for range end")?;
    match token {
        TokenTree::Literal(lit) => {
            token_tree_to_u8(TokenTree::Literal(lit)).map(RangeEndKind::Number)
        }
        TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => parse2(group.stream())
            .map(RangeEndKind::Expression)
            .map_err(|_| "Unable to parse expression for range end"),
        TokenTree::Ident(ident) => syn::parse_str(&ident.to_string())
            .map(RangeEndKind::Expression)
            .map_err(|_| "Unable to parse expression for range end"),
        _ => Err("Expected a number or an expression for range end"),
    }
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
        let expected_output =
            "#[instance_benchmarks] mod benchmarks { add_registrar { } }".replace(" ", "");
        let sanitized_output = output.replace(" ", "");

        assert_eq!(
            sanitized_output, expected_output,
            "The output did not match the expected output. Actual: '{}', Expected: '{}'",
            output, expected_output
        );

        println!("File AST is: {}", output);
    }
}
