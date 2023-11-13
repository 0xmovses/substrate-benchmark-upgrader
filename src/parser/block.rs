use crate::lexer::{BenchmarkLine, Lexer, LineKind};
use crate::parser::{extrinsic::ExtrinsicCall, param::ParamParser};
use anyhow::{anyhow, Result};
use nom::combinator::{cut, map_parser, not, peek, recognize};
use nom::error::context;
use nom::multi::{many0, many0_count, many1, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, separated_pair, terminated, tuple};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::{alpha1, char, multispace0, multispace1},
    combinator::map,
    sequence::preceded,
    IResult,
};
use quote::quote;
use syn::{parse_quote, Block, Item, ItemFn, ItemMod, Stmt, parse2, ExprMacro, Expr};
use proc_macro2::TokenStream;
use syn::punctuated::Pair::Punctuated;

pub struct BlockParser;

impl BlockParser {
    pub fn dispatch(line: &str, lexer: &Lexer) -> Result<BenchmarkLine> {
        let trimmed_line = line.trim_start();
        println!("trimmed_line: {:?}", trimmed_line);

        match trimmed_line {
            _ if trimmed_line.starts_with("benchmarks") => match Self::benchmark(line) {
                Ok((_remaining, parsed)) => Ok(BenchmarkLine {
                    head: Some(parsed.to_string()),
                    kind: LineKind::Mod,
                    content: None,
                    param_content: None,
                    fn_body: None,
                }),
                Err(e) => Err(anyhow!("Error parsing benchmark: {:?}", e)),
            },
            _ if trimmed_line.starts_with("let") => match ParamParser::dispatch(line) {
                Ok(parameter) => Ok(parameter),
                Err(e) => Err(anyhow!("Error parsing parameter: {:?}", e)),
            },
            _ if trimmed_line.starts_with("ensure!") => match Self::ensure(line) {
                Ok((_remaining, parsed)) => Ok(BenchmarkLine {
                    head: None,
                    kind: LineKind::Ensure,
                    content: Some(parsed.to_string()),
                    param_content: None,
                    fn_body: None,
                }),
                Err(e) => Err(anyhow!("Error parsing ensure: {:?}", e)),
            },
            _ if Self::is_function_declaration(trimmed_line) => match Self::function(line) {
                Ok((_remaining, parsed)) => {
                    println!("matches function: {:?}", parsed);
                    let (_remaining, fn_body) = Self::fn_body(parsed, lexer.0.as_str()).unwrap();
                    Ok(BenchmarkLine {
                        head: Some(parsed.to_string()),
                        kind: LineKind::Fn,
                        content: None,
                        param_content: None,
                        fn_body: Some(fn_body.to_string()),
                    })
                }
                Err(e) => Err(anyhow!("Error parsing function: {:?}", e)),
            },
            _ if trimmed_line.starts_with("verify") => {
                match Self::function(line) {
                    Ok((_remaining, parsed)) => {
                        let (_remaining, fn_body) = Self::fn_body(parsed, lexer.0.as_str()).unwrap();

                        println!("matches verify");
                        Ok(BenchmarkLine {
                            head: Some(parsed.to_string()),
                            kind: LineKind::Verify,
                            content: None,
                            param_content: None,
                            fn_body: Some(fn_body.to_string()),
                        })
                    }
                    Err(e) => Err(anyhow!("Error parsing function: {:?}", e)),
                }
            }
            _ if trimmed_line.starts_with("}:") => Ok(BenchmarkLine {
                head: None,
                kind: LineKind::Extrinsic,
                content: Some(line.to_string()),
                param_content: None,
                fn_body: None,
            }),
            _ if trimmed_line.starts_with("(")
                || trimmed_line.starts_with("T::")
                || trimmed_line.starts_with("}") =>
            {
               println!("matches other chars");
                Ok(BenchmarkLine {
                    head: None,
                    kind: LineKind::Content,
                    content: Some(line.to_string()),
                    param_content: None,
                    fn_body: None,
                })
            }
            _ if trimmed_line.starts_with("for i") =>  {
                println!("matches for line: {:?}", trimmed_line);
                Ok(BenchmarkLine {
                head: None,
                kind: LineKind::Content,
                content: Some(line.to_string()),
                param_content: None,
                fn_body: None,
            }) },
            _ => {
                println!("matches other line: {:?}", trimmed_line);
                Ok(BenchmarkLine {
                    head: None,
                    kind: LineKind::Content,
                    content: Some(line.to_string()),
                    param_content: None,
                    fn_body: None,
                })
            }
        }
    }

    fn is_function_declaration(line: &str) -> bool {
        let trimmed_line = line.trim();
        if trimmed_line.contains("for") {
            return false;
        }
        let without_whitespace = trimmed_line.split_whitespace().collect::<String>();
        without_whitespace.ends_with("{")
    }

    pub fn benchmark(input: &str) -> IResult<&str, &str> {
        preceded(
            multispace0, // Optional whitespace
            alt((
                map(tag("benchmarks!"), |_| "benchmarks"),
                map(tag("benchmarks_instance_pallet!"), |_| {
                    "benchmarks_instance_pallet"
                }),
            )),
        )(input)
    }

    pub fn function(input: &str) -> IResult<&str, &str> {
        terminated(
            preceded(multispace0, recognize(separated_list1(tag("_"), alpha1))),
            preceded(multispace0, char('{')),
        )(input)
    }

    pub fn fn_body<'a>(fn_name: &'a str, input: &'a str) -> IResult<&'a str, &'a str> {
        //println!("fn_name: {:?}", fn_name);
        //println!("input fn_body: {:?}", input);
        // Find the function name in the input and move past it
        let (input, _) = take_until(fn_name)(input)?;
        let (input, _) = tag(fn_name)(input)?;

        // Skip whitespace and find the opening brace of the function body
        let (input, _) = multispace0(input)?;
        let (input, _) = char('{')(input)?;

        // Now capture everything inside the top-level curly braces
        let (input, content) = take_until("}")(input)?;

        Ok((input, content))
    }

    pub fn ensure(input: &str) -> IResult<&str, &str> {
        // Ignore leading whitespace, match "ensure!", and capture everything up to the ending ");"
        let (input, _) = preceded(multispace0, tag("ensure!"))(input)?;
        let (input, content) = delimited(
            char('('),
            // Capture everything inside the parentheses
            take_until(");"),
            // Expect the closing ");"
            tag(");"),
        )(input)?;

        Ok((input, content))
    }
}

pub struct BlockWriter;

impl BlockWriter {
    pub fn dispatch_mod(input: &str) -> String {
        // Check for benchmark-related keywords
        if input.trim_start().starts_with("benchmarks!") {
            Self::mod_item()
        } else if input
            .trim_start()
            .starts_with("benchmarks_instance_pallet!")
        {
            Self::mod_instance_item()
        } else {
            "Error: Invalid benchmark module type".to_string()
        }
    }

    pub fn mod_item() -> String {
        format!("#[benchmarks]\nmod benchmarks{{\n\n}}")
    }

    pub fn mod_instance_item() -> String {
        format!("#[instance_benchmarks]\nmod benchmarks{{\n\n}}")
    }

    pub fn fn_item(function_name: &str) -> String {
        format!(
            "#[benchmark]\nfn {}() -> Result<(), BenchmarkError> {{\n\n}}",
            function_name
        )
    }

    pub fn fn_into_mod(ast: Vec<Item>) -> Result<ItemMod> {
        let mut module: Option<ItemMod> = None;
        let mut functions: Vec<ItemFn> = Vec::new();

        for item in ast {
            match item {
                Item::Mod(item_mod) => {
                    module = Some(item_mod);
                }
                Item::Fn(item_fn) => {
                    functions.push(item_fn);
                }
                _ => {}
            }
        }
        let mut module = module.ok_or_else(|| anyhow!("fn_into_mod error"))?;

        // Insert functions into the module's content
        if let Some((_brace, content)) = &mut module.content {
            for function in functions {
                content.push(Item::Fn(function));
            }
        } else {
            module.content = Some((
                syn::token::Brace::default(),
                functions.into_iter().map(Item::Fn).collect(),
            ));
        }

        Ok(module)
    }

    pub fn content_into_fn(mut mod_block: ItemMod, body: Block) -> Result<String> {
        // Flag to indicate if the function body has been inserted
        let mut inserted = false;

        // Iterate over the items in the module
        for item in &mut mod_block.content.as_mut().unwrap().1 {
            // Match only on functions
            if let Item::Fn(ItemFn { ref mut block, .. }) = item {
                // Replace the existing block with the new one
                *block = Box::new(body.clone());
                inserted = true;
                break; // Assuming you only want to insert into the first found function
            }
        }

        // Check if the insertion was successful
        if !inserted {
            return Err(anyhow!("No suitable function found for insertion"));
        }

        // Convert the modified module back into a string
        let result = quote!(#mod_block).to_string();
        println!("result: {:?}", result);

        Ok(result)
    }

    pub fn extrinsic_into_fn(ast: Vec<Item>, ext: &str) -> Result<String> {
        let mut modified_ast = ast.clone();
        let mut last_mod_function: Option<&mut ItemFn> = None;
        // Iterate in reverse to find the last mod block
        for item in modified_ast.iter_mut().rev() {
            if let Item::Mod(ItemMod { content: Some((_, items)), .. }) = item {
                // Iterate in reverse to find the last function within this mod block
                for item in items.iter_mut().rev() {
                    if let Item::Fn(function) = item {
                        last_mod_function = Some(function);
                        break;
                    }
                }
                if last_mod_function.is_some() {
                    break;
                }
            }
        }

        if let Some(function) = last_mod_function {
            // Parse the extrinsic string into a TokenStream
            let insert_tokens: TokenStream = ext.parse().expect("Failed to parse into TokenStream");
            let extrinsic = parse2::<ExtrinsicCall>(insert_tokens)?;

            // Convert the parsed ExtrinsicCall into a Stmt
            let stmt = Stmt::Expr(Expr::Verbatim(quote! { #extrinsic }));
            function.block.stmts.push(stmt);
       } else {
            return Err(anyhow!("No function found in AST"));
        }

        let cleaned_ast = Self::remove_duplicate_mods(modified_ast);

        // Convert the modified AST back to a string
        let result = quote! {
        #( #cleaned_ast )*
        }.to_string();

        println!("result extrinsic: {:?}", result);

        Ok(result)
    }

    pub fn remove_duplicate_mods(ast: Vec<Item>) -> Vec<Item> {
        let mut new_ast = Vec::new();
        let mut last_non_empty_benchmarks_mod: Option<Item> = None;

        for item in ast {
            if let Item::Mod(ItemMod { ident, content: Some((_, items)), .. }) = &item {
                if ident == "benchmarks" {
                    // Check if the mod contains any function with a body
                    let contains_non_empty_fn = items.iter().any(|item| {
                        matches!(item, Item::Fn(ItemFn { block, .. }) if !block.stmts.is_empty())
                    });

                    if contains_non_empty_fn {
                        // Replace the last non-empty benchmarks mod with this one
                        last_non_empty_benchmarks_mod = Some(item.clone());
                    }
                } else {
                    // Keep all other items
                    new_ast.push(item.clone());
                }
            }
        }

        // Add the last non-empty benchmarks mod to the new AST, if any
        if let Some(mod_item) = last_non_empty_benchmarks_mod {
            new_ast.push(mod_item);
        }

        new_ast
    }


    pub fn clean_code_block(code_block: &str) -> Result<Block> {
        let cleaned_code = code_block
            .split(';')
            .map(|line| {
                let trimmed_line = line.trim();
                if let Some(start_idx) = trimmed_line.find("=>") {
                    trimmed_line[start_idx + 2..].trim()
                } else {
                    trimmed_line
                }
            })
            .filter(|line| !line.is_empty())
            .map(|line| {
                if line.ends_with(';') {
                    line.to_string()
                } else {
                    format!("{};", line)
                }
            }) // Ensure each line ends with a semicolon
            .collect::<Vec<String>>()
            .join(" ");

        // Wrap the cleaned code in braces to form a valid block
        let block_str = format!("{{ {} }}", cleaned_code);
        println!("block_str: {:?}", block_str);

        // Parse the cleaned code into a syn::Block
        syn::parse_str::<Block>(&block_str)
            .map_err(|e| anyhow!("Error parsing cleaned code into a Block: {}", e))
    }

    pub fn extrinsic(input: &str) -> String {
        let re = regex::Regex::new(r"(\w*):\s*_<T::(\w+)>\((\w+), (\w+)\)").unwrap();
        let replacement = "#[extrinsic_call]\n_<T::${2}>(${3}, ${4})";
        let output = re.replace(input, replacement).to_string();

        // Regex to match '}' with any preceding whitespace (including tabs and newlines)
        let trim_re = regex::Regex::new(r"[\s\t]*\}\s*").unwrap();
        trim_re.replace_all(&output, "").to_string()
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_dispatch_should_call_benchmarks() {
        let lexer = Lexer::new("".to_string());
        let input = "benchmarks!";
        let line = BlockParser::dispatch(input, &lexer).unwrap();
        assert_eq!(line.head, Some("benchmarks".to_string()));
    }

    #[test]
    fn test_dispatch_should_call_function() {
        let lexer = Lexer::new("".to_string());
        let input = "propose_proposed {";
        let line = BlockParser::dispatch(input, &lexer).unwrap();
        assert_eq!(line.head, Some("propose_proposed".to_string()));
    }

    #[test]
    fn test_benchmarks_instance_pallet() {
        let input = "benchmarks_instance_pallet!";
        let (_, parsed) = BlockParser::benchmark(input).unwrap();
        assert_eq!(parsed, "benchmarks_instance_pallet");
    }

    #[test]
    fn test_benchmarks_with_whitespace() {
        let input = "    benchmarks!"; // Input with leading whitespace
        let (_, parsed) = BlockParser::benchmark(input).unwrap();
        assert_eq!(parsed, "benchmarks");
    }

    #[test]
    fn test_parse_valid_function_call() {
        let input = "propose_proposed {";
        let (_, parsed) = BlockParser::function(input).unwrap();
        assert_eq!(parsed, "propose_proposed");
    }

    #[test]
    fn test_parse_verify_function_call() {
        let input = "verify {";
        let (_, parsed) = BlockParser::function(input).unwrap();
        assert_eq!(parsed, "verify");
    }

    #[test]
    fn test_parse_function_call_with_whitespace() {
        let input = "   propose_proposed {"; // Leading whitespace
        let (_, parsed) = BlockParser::function(input).unwrap();
        assert_eq!(parsed, "propose_proposed");
    }

    #[test]
    fn test_parse_function_call_with_new_name() {
        let input = "propose_proposed_with_new_name {";
        let (_, parsed) = BlockParser::function(input).unwrap();
        assert_eq!(parsed, "propose_proposed_with_new_name");
    }

    #[test]
    fn test_mod_item_generation() {
        let input = "benchmarks!";
        let (_, parsed) = BlockParser::benchmark(input).unwrap();
        let expected = "#[instance_benchmarks]\nmod benchmarks {\n\n}";
        let actual = BlockWriter::dispatch_mod(parsed);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_fn_item_generation() {
        let input = "propose_proposed {";
        let (_, parsed) = BlockParser::function(input).unwrap();
        let expected = "#[benchmark]\nfn propose_proposed() -> Result<(), BenchmarkError> {\n\n}";
        let actual = BlockWriter::dispatch_mod(parsed);
        assert_eq!(actual, expected);
    }
}
