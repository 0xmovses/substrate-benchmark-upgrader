use crate::parser::block::BlockParser;
use crate::parser::param::BenchmarkParameter;
use syn::{parse_str, Item};

#[derive(Debug, Clone)]
pub enum LineKind {
    Mod,
    Fn,
    FnParam,
    Verify,
    Ensure,
    Extrinsic,
    Content,
}

#[derive(Debug, Clone)]
pub struct BenchmarkLine {
    pub head: Option<String>,
    pub kind: LineKind,
    pub content: Option<String>,
    pub param_content: Option<BenchmarkParameter>,
    pub fn_body: Option<String>,
}

pub struct Lexer(pub(crate) String);

impl Lexer {
    pub fn new(input: String) -> Self {
        Self(input)
    }

    pub fn parse(&self) -> Result<Vec<BenchmarkLine>, String> {
        let lines: Vec<&str> = self.0.split("\n").collect();
        let mut blocks: Vec<BenchmarkLine> = Vec::new();
        for line in lines {
            match BlockParser::dispatch(line, self.clone()) {
                Ok(line) => {
                    let benchmark_line = BenchmarkLine {
                        head: line.head,
                        kind: line.kind,
                        content: line.content,
                        param_content: line.param_content,
                        fn_body: line.fn_body,
                    };
                    //println!("/n: {:?}", benchmark_line);
                    blocks.push(benchmark_line);
                }
                Err(e) => {
                    return Err(format!("Parsing failed: {:?}", e));
                }
            }
            //println!("blocks: {:?}", blocks);
        }
        if blocks.is_empty() {
            Err("No blocks parsed".to_string())
        } else {
            Ok(blocks)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_function_call() {
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

        let l = Lexer::new(input.to_string());
        let parsed = l.parse().unwrap();
        for block in parsed {
            println!("\n{:?}\n", block);
        }
    }
}
