use nom::branch::alt;
use nom::combinator::{map_res, recognize, value};
use nom::multi::{many0_count, many_till};
use nom::{
    bytes::complete::{tag, take_until, take_while_m_n},
    character::complete::{
        alpha1, alphanumeric1, anychar, char, digit1, multispace0, multispace1, u8 as nom_u8,
    },
    combinator::{cut, map},
    error::ParseError,
    sequence::{preceded, terminated, tuple},
    IResult,
};
use proc_macro2::Ident;
use syn::parse_str;

pub struct ParamParser;

pub struct BenchmarkParameter {
    name: String,
    range_start: u8,
    range_end: String,
}

impl ParamParser {
    pub fn dispatch(input: &str) -> IResult<&str, BenchmarkParameter> {
        if input.trim_start().starts_with("let ") {
            // Detect the 'let' keyword, which starts a parameter declaration.
            // The logic after 'let' would determine what specific parsing function to call.
            // For now, we're assuming the next relevant parse after 'let' would be range_start.
            Self::let_declaration(input)
        } else {
            Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    }

    pub fn let_declaration(input: &str) -> IResult<&str, BenchmarkParameter> {
        let (input, _) = tag("let ")(input.trim())?;
        let (input, name) = recognize(alpha1)(input)?;
        let (input, _) = multispace0(input)?;
        let (input, _) = tag("in")(input)?;
        let (input, _) = multispace0(input)?;

        let (input, range_start_val) =
            map_res(digit1, |digit_str: &str| digit_str.parse::<u8>())(input)?;
        let (input, _) = multispace0(input)?;

        // Directly capture the range end after '..'
        let (input, range_end_val) = Self::range_end(input)?;

        Ok((
            input,
            BenchmarkParameter {
                name: name.to_string(),
                range_start: range_start_val,
                range_end: range_end_val.trim().to_string(),
            },
        ))
    }

    fn range_end(input: &str) -> IResult<&str, String> {
        let (input, _) = tag("..")(input)?;
        alt((
            map(
                terminated(recognize(take_until(";")), char(';')),
                |s: &str| s.trim().to_string(),
            ),
            map(
                terminated(recognize(take_until("=>")), tag("=>")),
                |s: &str| s.trim().to_string(),
            ),
        ))(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::Err;

    #[test]
    fn test_parse_param_declaration_with_expression() {
        let input = "let r in 1 .. T::MaxRegistrars::get() =>";
        match ParamParser::let_declaration(input) {
            Ok((remaining, param)) => {
                assert_eq!(param.name, "r");
                assert_eq!(param.range_start, 1);
                assert_eq!(param.range_end, "T::MaxRegistrars::get()");
                assert_eq!(remaining, "");
            }
            Err(e) => panic!("Parsing failed when it should have succeeded: {:?}", e),
        }
    }

    #[test]
    fn test_parse_param_declaration_with_constant() {
        let input = "let b in 1 .. MAX_BYTES;";
        match ParamParser::let_declaration(input) {
            Ok((remaining, param)) => {
                assert_eq!(param.name, "b");
                assert_eq!(param.range_start, 1);
                assert_eq!(param.range_end, "MAX_BYTES");
                assert_eq!(remaining, "");
            }
            Err(e) => panic!("Parsing failed when it should have succeeded: {:?}", e),
        }
    }

    #[test]
    fn test_invalid_param_declaration_no_range() {
        let input = "let foo =";
        let result = ParamParser::let_declaration(input);
        assert!(result.is_err(), "The input should not be parsed successfully.");
    }
}
