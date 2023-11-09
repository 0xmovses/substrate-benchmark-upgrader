use crate::lexer::{BenchmarkLine, LineKind};
use crate::parser::{block::{BlockParser, BlockWriter}, param::{ParamParser, ParamWriter}};
use anyhow::{Result, anyhow};
pub struct Writer;

impl Writer {
    // Generates the entire module with benchmarks from a block of DSL code.
    pub fn generate_module(lines: Vec<BenchmarkLine>) -> Result<Vec<String>> {
        let mut gen: Vec<String> = Vec::new();
        for line in lines {
            println!("\n input line: {:?}", line);
            match line.kind {
                LineKind::Mod => {
                   if let Some(_head)  = line.head {
                       println!("\n -> is Mod");
                       let output = BlockWriter::mod_item();
                       gen.push(output);
                   }
                }
                LineKind::Fn => {
                    if let Some(head) = line.head {
                        println!("\n -> is Fn");
                        let output = BlockWriter::fn_item(&head);
                        gen.push(output);
                    }
                }
                LineKind::FnParam => {
                    if let Some(param_content) = line.param_content {
                        println!("\n -> is FnParam");
                        let fn_input = ParamWriter::fn_input(&param_content);
                        if let Some(fn_signature) = gen.last() {
                            match ParamWriter::fn_gen(fn_input, fn_signature) {
                                Ok(output) => {
                                    gen.pop(); // we need to remove as we're overwriting this
                                    gen.push(output);
                                }
                                Err(e) => {
                                    return Err(anyhow!("Error parsing parameter: {:?}", e))
                                }
                            }
                        }
                    }
                }
                LineKind::Ensure => {
                    //@TODO
                    //let output = BlockWriter::ensure(&line.content.unwrap());
                    println!("\n TODO output Ensure");
                }
                LineKind::Extrinsic => {
                    //@TODO
                    //let output = BlockWriter::extrinsic(&line.content.unwrap());
                    println!("\n TODO output Extrinsic");
                }
                _ => {
                    println!("\n Other Case No output");
                }
            }
            //println!("\n Gen: {}", gen);
        }
        Ok(gen)
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

        let gen_lines= Writer::generate_module(parsed_lines).unwrap();
        for line in gen_lines {
            println!("\n Gen: {}", line);
        }

    }
}