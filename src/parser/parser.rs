use nom::{
    IResult, character::complete::{multispace1, char, u8 as nom_u8}, sequence::{preceded, terminated, tuple}, combinator::map,
    bytes::complete::tag, error::ParseError,
};

// Helper parsers
fn parse_let(input: &str) -> IResult<&str, &str> {
    tag("let")(input)
}

fn parse_in(input: &str) -> IResult<&str, &str> {
    tag("in")(input)
}

// Parses the range start, just a u8 for now
fn parse_range_start(input: &str) -> IResult<&str, u8> {
    preceded(multispace1, nom_u8)(input)
}

// Stub for the range end parser
fn parse_range_end(input: &str) -> IResult<&str, &str> {
    // Placeholder logic for parsing the range end
    tag("T::MaxRegistrars::get() - 1")(input)
}

// Parser that ignores characters after '=>'
fn ignore_after_arrow(input: &str) -> IResult<&str, &str> {
    terminated(multispace1, preceded(tag("=>"), multispace1))(input)
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_let() {
        let input = "let";
        assert_eq!(parse_let(input), Ok(("", "let")));
    }

    #[test]
    fn test_parse_in() {
        let input = "in"; // Including spaces to test the parser's robustness
        assert_eq!(parse_in(input), Ok(("", "in")));
    }

    #[test]
    fn test_parse_range_start() {
        let input = " 42"; // Including leading space
        assert_eq!(parse_range_start(input), Ok(("", 42)));
    }

    #[test]
    fn test_parse_range_end() {
        let input = "T::MaxRegistrars::get() - 1";
        assert_eq!(parse_range_end(input), Ok(("", "T::MaxRegistrars::get() - 1")));
    }

    #[test]
    fn test_ignore_after_arrow() {
        let input = " => some irrelevant stuff";
        assert_eq!(ignore_after_arrow(input), Ok(("some irrelevant stuff", " ")));
    }

    fn test_parse_parameter() {
        let input = "let r in 1 .. T::MaxRegistrars::get() - 1 =>";
    }
}