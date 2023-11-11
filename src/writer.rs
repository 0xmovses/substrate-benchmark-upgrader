use crate::lexer::{BenchmarkLine, FnBody, Lexer, LineKind};
use crate::parser::{
    block::{BlockParser, BlockWriter},
    param::{ParamParser, ParamWriter},
};
use anyhow::{anyhow, Result};
use quote::quote;
use syn::{parse_str, Item, ItemFn, Stmt};
use proc_macro2::TokenStream;
use regex::Regex;


pub struct Writer;

#[derive(Debug, Clone)]
pub struct ModGen {
    pub parent: String,
    pub fns: Vec<FnBlockGen>,
}

impl Default for ModGen {
    fn default() -> Self {
        Self {
            parent: "".to_string(),
            fns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FnBlockGen {
    pub signature: Option<String>,
    pub body: Option<String>,
    pub extrinsic: Option<String>,
    pub verify: Option<String>,
}

impl Writer {
    // Generates the entire module with benchmarks from a block of DSL code.
    pub fn generate_module(lines: Vec<BenchmarkLine>) -> Result<ModGen> {
        let mut fns: Vec<FnBlockGen> = Vec::new();
        let mut mod_gen =  ModGen::default();
        for i in 0..lines.len() {
            println!("mod_gen: {:?}", mod_gen);
            let line = &lines[i];
            println!("\n input line: {:?}", line);
            match line.kind {
                LineKind::Mod => {
                    if let Some(_head) = &line.head {
                        println!("\n -> is Mod");
                        let output = BlockWriter::mod_item();
                        mod_gen.parent = output;
                    }
                }
                LineKind::FnBody => {
                    match FnBody::Content => {

                    },
                    if let Some(head) = &line.head {
                        println!("\n -> is Fn");
                        let output = BlockWriter::fn_item(&head);
                        if let (body) = &line.fn_body {
                           println!("body: {:?}", body);
                        }
                        mod_gen.fns.push(FnBlockGen {
                            signature: None,
                            body: Some(output),
                            extrinsic: None,
                            verify: None,
                        });
                    }
                }
                LineKind::Verify => {
                    if let Some(body) = &line.fn_body {
                        mod_gen.fns.push(FnBlockGen {
                            signature: None,
                            body: None,
                            extrinsic: None,
                            verify: Some(body.to_string()),
                        });
                    }
                }
                LineKind::Extrinsic => {
                    println!("\n -> is Extrinsic");
                    if let Some(content) = &line.content {
                        let extrinsic = BlockWriter::extrinsic(&content);
                        mod_gen.fns.push(FnBlockGen {
                            signature: None,
                            body: None,
                            extrinsic: Some(extrinsic),
                            verify: None,
                        });
                    }
                }
                LineKind::FnParam => {
                    if let Some(ref param_content) = line.param_content {
                        println!("\n -> is FnParam");
                        let fn_input = ParamWriter::fn_input(&param_content);
                        if let Some(fn_sig) = mod_gen.fns.last() {
                            let complete_sig = ParamWriter::fn_gen(fn_input, fn_sig.body.clone().unwrap())?;

                            //gen.pop();
                            mod_gen.fns.push(FnBlockGen{
                                signature: Some(complete_sig),
                                body: None,
                                extrinsic: None,
                                verify: None,
                            });

                            let ast = Self::parse_to_ast(mod_gen.fns.clone())?;
                            let fn_mod = BlockWriter::fn_into_mod(ast)?;

                            // the fn_body is in the previous line
                            if i > 0 {
                                if let Some(fn_body) = &lines[i - 1].fn_body {
                                    let valid_block = BlockWriter::clean_code_block(fn_body)?;
                                    let complete_fn =
                                        BlockWriter::content_into_fn(fn_mod, valid_block).unwrap();
                                    mod_gen.fns.push(FnBlockGen {
                                        signature: None,
                                        body: Some(complete_fn),
                                        extrinsic: None,
                                        verify: None,
                                    })
                                }
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        Ok(mod_gen)
        //Ok(Self::adjust_module_closure(gen)?)
    }

    pub fn parse_to_ast(lines: Vec<FnBlockGen>) -> Result<Vec<Item>> {
        let mut ast_nodes: Vec<Item> = Vec::new();
        for line in lines {
            let ast_node = parse_str::<Item>(&line.body.unwrap())?;
            ast_nodes.push(ast_node);
        }
        Ok(ast_nodes)
    }

    pub fn adjust_module_closure(lines: Vec<String>) -> Result<Vec<String>> {
        let mut adjusted_lines = Vec::new();
        let misplaced_bracket_pattern = Regex::new(r"\}\s*\}").unwrap();
        let mut found_misplaced_bracket = false;

        for (index, line) in lines.iter().enumerate() {
            // Check if this line contains the misplaced bracket
            if misplaced_bracket_pattern.is_match(line) {
                found_misplaced_bracket = true;
                // Check if this is the last line or if the next line is not a closing bracket
                if index == lines.len() - 1 || !lines[index + 1].trim().starts_with('}') {
                    adjusted_lines.push(line.clone());
                }
                continue;
            }
            adjusted_lines.push(line.clone());
        }

        if !found_misplaced_bracket {
            return Err(anyhow!("No misplaced bracket found"));
        }

        Ok(adjusted_lines)
    }
}



#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::writer::Writer;

    #[test]
    fn test_lexer() {
        let input = r#"benchmarks! {
	add_registrar {
		let r in 1 .. T::MaxRegistrars::get() - 1 => add_registrars::<T>(r)?;
		ensure!(Registrars::<T>::get().len() as u32 == r, "Registrars not set up correctly.");
		let origin =
			T::RegistrarOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let account = T::Lookup::unlookup(account("registrar", r + 1, SEED));
	}: _<T::RuntimeOrigin>(origin, account)
	verify {
		ensure!(Registrars::<T>::get().len() as u32 == r + 1, "Registrars not added.");
	}"#;

        let lexer = Lexer::new(input.to_string());
        let parsed_lines = lexer.parse().unwrap();
        let gen_lines = Writer::generate_module(parsed_lines).unwrap();
        println!("Generated Lines:");
        for line in gen_lines.fns {
            println!("{:?}", line);
        }
    }
}
