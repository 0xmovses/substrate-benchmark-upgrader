use crate::lexer::{BenchmarkLine, Lexer, LineKind};
use crate::parser::{
    block::{BlockParser, BlockWriter},
    param::{ParamParser, ParamWriter},
};
use anyhow::{anyhow, Result};
use syn::{parse_str, Item, ItemMod};

pub struct Writer;

impl Writer {
    // Generates the entire module with benchmarks from a block of DSL code.
    pub fn generate_module(lines: Vec<BenchmarkLine>) -> Result<Vec<String>> {
        let mut gen: Vec<String> = Vec::new();
        for i in 0..lines.len() {
            let line = &lines[i];
            println!("\n input line: {:?}", line);
            match line.kind {
                LineKind::Mod => {
                    if let Some(_head) = &line.head {
                        println!("\n -> is Mod");
                        let output = BlockWriter::mod_item();
                        gen.push(output);
                    }
                }
                LineKind::Fn => {
                    if let Some(head) = &line.head {
                        println!("\n -> is Fn");
                        let output = BlockWriter::fn_item(&head);
                        gen.push(output);
                    }
                }
                LineKind::FnParam => {
                    if let Some(ref param_content) = line.param_content {
                        println!("\n -> is FnParam");
                        let fn_input = ParamWriter::fn_input(&param_content);
                        if let Some(fn_signature) = gen.last() {
                            let complete_sig = ParamWriter::fn_gen(fn_input, fn_signature)?;

                            gen.pop();
                            gen.push(complete_sig);

                            let ast = Self::parse_to_ast(gen.clone())?;
                            let fn_mod = BlockWriter::fn_into_mod(ast)?;

                            // the fn_body is in the previous line
                            if i > 0 {
                                if let Some(fn_body) = &lines[i - 1].fn_body {
                                    let valid_block = BlockWriter::clean_code_block(fn_body)?;
                                    let complete_fn =
                                        BlockWriter::content_into_fn(fn_mod, valid_block).unwrap();
                                    gen.push(complete_fn);
                                }
                            }
                        }
                    }
                },
                LineKind::Verify => {
                    if let Some(body) = &line.fn_body {
                        gen.push(body.to_owned());
                    }
                }
                LineKind::Extrinsic => {
                    let output = BlockWriter::extrinsic(&line.content.clone().unwrap());
                    let ast = Self::parse_to_ast(gen.clone())?;
                    let _ = BlockWriter::extrinsic_into_fn(ast, &output)?;
                    //gen.push(output);
                }
                _ => {}
            }
            println!("i = {:?}", i);
        }
        Ok(gen)
    }

    pub fn parse_to_ast(lines: Vec<String>) -> Result<Vec<Item>> {
        let mut ast_nodes: Vec<Item> = Vec::new();
        for line in lines {
            let ast_node = parse_str::<Item>(&line)?;
            ast_nodes.push(ast_node);
        }
        Ok(ast_nodes)
    }

    fn validate(code: &str) -> bool {
        let token_stream: proc_macro2::TokenStream = code.parse().unwrap();
        syn::parse::<ItemMod>(token_stream.into()).is_ok()
    }
}



#[cfg(test)]
mod tests {
    use syn::ItemMod;
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
        let gen = Writer::generate_module(parsed_lines).unwrap();
        for line in gen {
            println!("line: {:?}", line);
        }
    }
}
