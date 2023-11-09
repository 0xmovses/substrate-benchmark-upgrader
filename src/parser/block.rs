use nom::combinator::{cut, map_parser, not, peek, recognize};
use nom::error::context;
use nom::multi::{many0, many1, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, separated_pair, terminated, tuple};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{alpha1, char, multispace0, multispace1},
    combinator::map,
    sequence::preceded,
    IResult,
};
use crate::lexer::{BenchmarkLine, LineKind};
use crate::parser::param::ParamParser;
use anyhow::{Result, anyhow};
pub struct BlockParser;

impl BlockParser {
    pub fn dispatch(line: &str) -> Result<BenchmarkLine>{
        let trimmed_line = line.trim_start();

        match trimmed_line {
            _ if trimmed_line.starts_with("benchmarks") => {
               match Self::benchmark(line) {
                   Ok((_remaining, parsed)) => {
                       Ok(BenchmarkLine{
                           head: Some(parsed.to_string()),
                           kind: LineKind::Mod,
                           content: None,
                           param_content: None,
                       })
                   },
                   Err(e) => {
                       Err(anyhow!("Error parsing benchmark: {:?}", e))
                   }
               }
            },
            _ if trimmed_line.starts_with("let") => {
                match ParamParser::dispatch(line) {
                    Ok(parameter) => Ok(parameter),
                    Err(e) => {
                        Err(anyhow!("Error parsing parameter: {:?}", e))
                    }
                }
            },
            _ if trimmed_line.starts_with("ensure!") => {
                match Self::ensure(line) {
                    Ok((_remaining, parsed)) => {
                        Ok(BenchmarkLine{
                            head: None,
                            kind: LineKind::Ensure,
                            content: Some(parsed.to_string()),
                            param_content: None,
                        })
                    },
                    Err(e) => {
                        Err(anyhow!("Error parsing ensure: {:?}", e))
                    }
                }
            },
            _ if trimmed_line.starts_with("}:") => {
                Ok(BenchmarkLine{
                   head: None,
                    kind: LineKind::Extrinsic,
                    content: Some(line.to_string()),
                    param_content: None,
                })
            }
            _ if trimmed_line.starts_with("(")
                || trimmed_line.starts_with("T::")
                || trimmed_line.starts_with("}") => {
                Ok(BenchmarkLine{
                    head: None,
                    kind: LineKind::Content,
                    content: Some(line.to_string()),
                    param_content: None,
                })
            },
            _ => {
                match Self::function(line) {
                    Ok((_remaining, parsed)) => {
                        Ok(BenchmarkLine {
                            head: Some(parsed.to_string()),
                            kind: LineKind::Fn,
                            content: None,
                            param_content: None,
                        })
                    },
                    Err(e) => {
                        Err(anyhow!("Error parsing function: {:?}", e))
                    }
                }
            }
        }
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
            preceded(
                multispace0,
                recognize(separated_list1(tag("_"), alpha1))
            ),
            preceded(multispace0, char('{'))
        )(input)
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
        } else if input.trim_start().starts_with("benchmarks_instance_pallet!") {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_dispatch_should_call_benchmarks() {
        let input = "benchmarks!";
        let line = BlockParser::dispatch(input).unwrap();
        assert_eq!(line.head, Some("benchmarks".to_string()));
    }

    #[test]
    fn test_dispatch_should_call_function() {
        let input = "propose_proposed {";
        let line = BlockParser::dispatch(input).unwrap();
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
