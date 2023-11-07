use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use regex;
use std::fmt::{Debug, Display, Formatter};
use std::{fmt, fs, iter::Peekable, result::Result};
use syn::parse::Parser;
use syn::{braced,  parse::{Parse, ParseStream, Result as ParseResult}, parse2, parse_quote, punctuated::Punctuated, token::Semi, visit_mut::VisitMut, Attribute, Block, Expr, File, Generics, Ident, Item, ItemFn, ItemMacro, ItemMod, ReturnType, Signature, Stmt, Token, Visibility, Type, Pat, PatIdent, FnArg, PatType, VisRestricted};
use syn::token::{Colon, Comma};

enum RangeEndKind {
    Number(u8),
    Expression(Expr), // check, the DSL might not be valid syn::Expr ?
}

impl ToTokens for RangeEndKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            RangeEndKind::Number(num) => {
                tokens.extend(quote! { #num });
            }
            RangeEndKind::Expression(expr) => {
                // Since `Expr` already implements `ToTokens`, we can directly use it.
                expr.to_tokens(tokens);
            }
        }
    }
}

struct BenchmarkParameter {
    name: Ident,
    range_start: u8,
    range_end: RangeEndKind,
}

impl ToTokens for BenchmarkParameter {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let range_start = &self.range_start;
        // range_end needs special handling based on its variant
        match &self.range_end {
            RangeEndKind::Number(num) => {
                let range_end_tokens = quote! { #num };
                tokens.extend(quote! {
                    #name: Linear<#range_start, #range_end_tokens>,
                });
            },
            RangeEndKind::Expression(expr) => {
                // Since `Expr` already implements `ToTokens`, we can directly use it.
                let range_end_tokens = quote! { { #expr } };
                tokens.extend(quote! {
                    #name: Linear<#range_start, #range_end_tokens>,
                });
            },
        }
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

        for item in file.items.drain(..) {
            match item {
                Item::Macro(i_macro) if i_macro.mac.path.is_ident("benchmarks") => {
                    let content = i_macro.mac.tokens.clone();
                    let parsed_functions = parse_benchmark_functions(content);

                    for function in parsed_functions {
                        let parsed_params = parse_parameters(function.block.clone()).unwrap(); // Assuming this returns a Vec<BenchmarkParameter>

                        // Construct the function arguments from parsed_params
                        let mut fn_args = Punctuated::new();
                        for param in parsed_params {
                            // Construct the tokens for each parameter based on the ToTokens implementation
                            let ty = match param.range_end {
                                RangeEndKind::Number(num) => quote! { Linear<#param.range_start, #num> },
                                RangeEndKind::Expression(ref expr) => quote! { Linear<#param.range_start, { #expr }> },
                            };

                            let pat = Pat::Ident(PatIdent {
                                attrs: Vec::new(),
                                by_ref: None,
                                mutability: None,
                                ident: param.name.clone(),
                                subpat: None,
                            });

                            let arg_tokens = quote! { #pat: #ty };

                            // Parse the generated tokens into a FnArg
                            let arg: FnArg = parse2(arg_tokens)
                                .expect("Failed to parse tokens into FnArg");

                            fn_args.push_value(arg);
                            fn_args.push_punct(Comma::default());
                        }

                        // Create the new function signature with the arguments
                        let new_signature = Signature {
                            constness: None,
                            asyncness: None,
                            unsafety: None,
                            abi: None,
                            fn_token: Default::default(),
                            ident: function.name.clone(),
                            generics: Default::default(),
                            paren_token: Default::default(),
                            inputs: Default::default(),
                            variadic: None,
                            output: ReturnType::Default,
                        };

                        let block: Block = parse2(function.block).expect("Failed to parse block");

                        // Create the new ItemFn with the modified signature and the parsed block
                        let new_fn = ItemFn {
                            attrs: Vec::new(), // Preserve the original attributes
                            vis: Visibility::Inherited, // Preserve the original visibility
                            sig: new_signature,
                            block: Box::new(block), // Use the parsed Block
                        };

                        // Convert new_fn into an Item and push it to new_items
                        new_items.push(Item::Fn(new_fn));
                    }
                }
                _ => new_items.push(item), // Preserve other items as they are
            }
        }

        // After processing all items, add them back into the file
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
                                                TokenTree::Punct(ref punct)
                                                    if punct.as_char() == ';' =>
                                                {
                                                    // Found ';', terminate the parameter declaration
                                                    break;
                                                }
                                                _ => continue, // Skip other tokens
                                            }
                                        }
                                        break; // Break from the outer loop once ';' is found
                                    }
                                    _ => continue, // Skip other tokens before '=>'
                                }
                            }

                            // Push the parsed parameter
                            params.push(BenchmarkParameter {
                                name,
                                range_start,
                                range_end,
                            });
                        }
                        Some(_) | None => return Err("Expected 'in' keyword".to_string()),
                    }
                } else {
                    return Err("Expected parameter name after 'let'".to_string());
                }
            }
            _ => continue, // Ignore other tokens
        }
    }

    for param in &params {
        println!("Param name: {}", param.name);
        println!("Param range start: {}", param.range_start);
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
    //println!("File AST is: {}", tokens.to_string());
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
