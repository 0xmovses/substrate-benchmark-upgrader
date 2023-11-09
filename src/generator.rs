use crate::parser::{block::{BlockParser, BlockWriter}, param::{ParamParser, ParamWriter}};
pub struct CodeGenerator;

impl CodeGenerator {
    // Generates the entire module with benchmarks from a block of DSL code.
    pub fn generate_module(input: &str) -> Result<String, String> {
        println!("input on gen: {}", input);
        let new_mod = BlockWriter::dispatch_mod(input);
        println!("generated mod: {}", new_mod);
        let (parsed, removed) = BlockParser::function(input).unwrap();
        println!("parsed: {}", parsed);
        println!("removed: {}", removed);
        let new_nf = BlockWriter::fn_item(input);

        // Generate the module header.
        let output = BlockWriter::dispatch_mod(input);

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_module() {
        let input = "benchmarks_instance_pallet! {
            propose_proposed {
                let b in 1 .. MAX_BYTES;
                let m in 2 .. T::MaxFellows::get();
                let p in 1 .. T::MaxProposals::get();
            }
        }";
        let expected = "#[instance_benchmarks]
mod benchmarks_instance_pallet {
    use super::*;

    #[benchmark]
    fn propose_proposed(
        b: Linear<1, MAX_BYTES>,
        m: Linear<2, { T::MaxFellows::get() }>,
        p: Linear<1, { T::MaxProposals::get() }>,
    ) -> Result<(), BenchmarkError> {
        // TODO: Implement function body
    }
}";

        match CodeGenerator::generate_module(input) {
            Ok(output) => assert_eq!(output, expected),
            Err(e) => panic!("Module generation failed: {}", e),
        }
    }
}
