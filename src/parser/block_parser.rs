use nom::{
    IResult,
    bytes::complete::tag,
    branch::alt,
    combinator::map,
    sequence::preceded,
    character::complete::{multispace0, alpha1, multispace1, char},
};
use nom::combinator::{map_parser, not, peek, recognize};
use nom::error::context;
use nom::multi::{many0, many1, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, separated_pair, terminated, tuple};

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
                map(tag("benchmarks_instance_pallet!"), |_| "benchmarks_instance_pallet"),
            )),
        )(input)
    }

    pub fn function(input: &str) -> IResult<&str, &str> {
        recognize(
            preceded(
                multispace0,
                terminated(
                    separated_list1(tag("_"), alpha1), // At least one alphabetic character, possibly with underscores
                    preceded(multispace0, char('{'))   // Optional whitespace followed by an opening brace
                )
            )
        )(input)
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
        assert_eq!(parsed, "propose_proposed {");
    }

    #[test]
    fn test_benchmarks() {
        let input = "benchmarks!";
        let (_, parsed) = BlockParser::benchmark(input).unwrap();
        assert_eq!(parsed, "benchmarks");
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
        assert_eq!(parsed, "propose_proposed {");
    }

    #[test]
    fn test_parse_verify_function_call() {
        let input = "verify {";
        let (_, parsed) = BlockParser::function(input).unwrap();
        assert_eq!(parsed, "verify {");
    }

    #[test]
    fn test_parse_function_call_with_whitespace() {
        let input = "   propose_proposed {"; // Leading whitespace
        let (_, parsed) = BlockParser::function(input).unwrap();
        assert_eq!(parsed, "   propose_proposed {");
    }

    #[test]
    fn test_parse_function_call_with_new_name() {
        let input = "propose_proposed_with_new_name {";
        let (_, parsed) = BlockParser::function(input).unwrap();
        assert_eq!(parsed, "propose_proposed_with_new_name {");
    }
}
