use nom::{
    IResult,
    bytes::complete::tag,
    branch::alt,
    combinator::map,
    sequence::preceded,
    character::complete::multispace0,
};

pub struct BenchmarkParser;

impl BenchmarkParser {
    pub fn parse_benchmark(input: &str) -> IResult<&str, &str> {
        preceded(
            multispace0, // Optional whitespace
            alt((
                map(tag("benchmarks!"), |_| "benchmarks"),
                map(tag("benchmarks_instance_pallet!"), |_| "benchmarks_instance_pallet"),
            )),
        )(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_benchmarks() {
        let input = "benchmarks!";
        assert_eq!(BenchmarkParser::parse_benchmark(input), Ok(("", "benchmarks")));
    }

    #[test]
    fn test_parse_benchmarks_instance_pallet() {
        let input = "benchmarks_instance_pallet!";
        assert_eq!(BenchmarkParser::parse_benchmark(input), Ok(("", "benchmarks_instance_pallet")));
    }

    #[test]
    fn test_parse_benchmarks_with_whitespace() {
        let input = "    benchmarks!"; // Input with leading whitespace
        assert_eq!(BenchmarkParser::parse_benchmark(input), Ok(("", "benchmarks")));
    }
}
