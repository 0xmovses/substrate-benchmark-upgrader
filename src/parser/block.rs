use nom::combinator::{map_parser, not, peek, recognize};
use nom::error::context;
use nom::multi::{many0, many1, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, separated_pair, terminated, tuple};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, char, multispace0, multispace1},
    combinator::map,
    sequence::preceded,
    IResult,
};

pub struct BlockParser;

impl BlockParser {
    pub fn dispatch(input: &str) -> IResult<&str, &str> {
        // Check for benchmark-related keywords
        if input.trim_start().starts_with("benchmarks") {
            Self::benchmark(input)
        } else {
            // Otherwise, assume it's a block function call
            Self::function(input)
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
}

pub struct BlockWriter;

impl BlockWriter {
    pub fn mod_item(benchmark_type: &str) -> String {
        format!("#[instance_benchmarks]\nmod {} {{\n\n}}", benchmark_type)
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
        let (_, parsed) = BlockParser::dispatch(input).unwrap();
        assert_eq!(parsed, "benchmarks");
    }

    #[test]
    fn test_dispatch_should_call_function() {
        let input = "propose_proposed {";
        let (_, parsed) = BlockParser::dispatch(input).unwrap();
        assert_eq!(parsed, "propose_proposed");
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
        let actual = BlockWriter::mod_item(parsed);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_fn_item_generation() {
        let input = "propose_proposed {";
        let (_, parsed) = BlockParser::function(input).unwrap();
        let expected = "#[benchmark]\nfn propose_proposed() -> Result<(), BenchmarkError> {\n\n}";
        let actual = BlockWriter::fn_item(parsed);
        assert_eq!(actual, expected);
    }
}
