use nom::branch::alt;
use nom::combinator::{map_res, recognize, value};
use nom::multi::{many0_count, many_till};
use nom::{
    bytes::complete::{tag, take_until, take_while_m_n, },
    character::complete::{alpha1, alphanumeric1, char, anychar, multispace0, digit1, multispace1, u8 as nom_u8},
    combinator::{map, cut},
    error::ParseError,
    sequence::{preceded, terminated, tuple},
    IResult,
};
use proc_macro2::Ident;

pub struct ParamParser;

#[derive(Debug, PartialEq)]
pub enum Range {
    Start(u8),
    End(RangeEndKind),
}
#[derive(Debug, PartialEq)]
pub enum RangeEndKind {
    Constant(String),
    Expression(String), // check, the DSL might not be valid syn::Expr ?
}

struct BenchmarkParameter {
    name: Ident,
    range_start: u8,
    range_end: RangeEndKind,
}

impl ParamParser {
    pub fn dispatch(input: &str) -> IResult<&str, Range> {
        if input.trim_start().starts_with("let ") {
            println!("Dispatching to after_let");
            // Detect the 'let' keyword, which starts a parameter declaration.
            // The logic after 'let' would determine what specific parsing function to call.
            // For now, we're assuming the next relevant parse after 'let' would be range_start.
            Self::after_let(input)
        } else {
            Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
        }
    }

    fn after_let(input: &str) -> IResult<&str, Range> {
        let (input, _) = tag("let ")(input.trim())?; // Confirm 'let' keyword is present
        let (input, _) = multispace0(input)?;        // Consume optional whitespace
        let (input, _) = alpha1(input)?;             // Consume the variable name
        let (input, _) = multispace0(input)?;        // Consume optional whitespace after variable name
        let (input, _) = tag("in")(input)?;          // Confirm 'in' keyword is present
        let (input, _) = multispace0(input)?;        // Consume optional whitespace after 'in'
        let (input, range_start_val) = digit1(input)?; // Parse the range start value

        // Convert the digit string to a u8 and wrap it in Range::Start
        let range_start_val = range_start_val.parse::<u8>().unwrap();
        Ok((input, Range::Start(range_start_val)))
    }
    /// Parses the range start
    pub fn range_start(input: &str) -> IResult<&str, u8> {
        println!("range_start: input: {:?}", input);
        preceded(multispace0, nom::character::complete::u8)(input)
    }

    /// Main range end parser that tries both parsers
    pub fn range_end(input: &str) -> IResult<&str, RangeEndKind> {
        preceded(
            tuple((multispace0, tag(".."), multispace0)),
            alt((Self::constant, Self::expression)),
        )(input)
    }

    /// Parses a constant range end, which is a string of uppercase letters
    fn constant(input: &str) -> IResult<&str, RangeEndKind> {
        let parse_identifier = recognize(preceded(
            alpha1,
            many0_count(terminated(
                take_while_m_n(1, 1, |c: char| c == '_' || c.is_alphanumeric()),
                alphanumeric1,
            )),
        ));
        map(terminated(parse_identifier, char(';')), |constant: &str| {
            RangeEndKind::Constant(constant.to_string())
        })(input)
    }

    // to parse an expression which may end with a semicolon or arrow
    fn expression(input: &str) -> IResult<&str, RangeEndKind> {
        let (input, expr) = alt((
            Self::till_semi, // Parse till semicolon
            Self::till_arrow, // Parse till arrow
        ))(input)?;

        Ok((input, RangeEndKind::Expression(expr.trim().to_string())))
    }

    // Parse until a semicolon is encountered
    fn till_semi(input: &str) -> IResult<&str, &str> {
        terminated(take_until(";"), char(';'))(input)
    }

    // Parse until an arrow "=>" is encountered
    pub fn till_arrow(input: &str) -> IResult<&str, &str> {
        terminated(take_until("=>"), tag("=>"))(input)
    }

    pub fn item_let(input: &str) -> IResult<&str, &str> {
        tag("let")(input)
    }

    pub fn item_in(input: &str) -> IResult<&str, &str> {
        tag("in")(input)
    }

    // ParamParser that ignores characters after '=>'
    pub fn ignore_after_arrow(input: &str) -> IResult<&str, &str> {
        terminated(multispace1, preceded(tag("=>"), multispace1))(input)
    }
}

#[cfg(test)]
mod tests {
    use super::RangeEndKind::*;
    use super::*;

    #[test]
    fn test_parse_let() {
        let input = "let";
        let (_, result) = ParamParser::item_let(input).unwrap();
        assert_eq!(result, "let");
    }
    #[test]
    fn test_parse_in() {
        let input = "in";
        let (_, result) = ParamParser::item_in(input).unwrap();
        assert_eq!(result, "in");
    }
    #[test]
    fn test_parse_range_start() {
        let input = "42";
        let (_, result) = ParamParser::range_start(input).unwrap();
        assert_eq!(result, 42);
    }
    #[test]
    fn test_range_end_with_constant() {
        let input = ".. MAX_BYTES;";
        let (remaining, result) = ParamParser::range_end(input).unwrap();
        assert_eq!(result, Constant("MAX_BYTES".to_string()));
        assert_eq!(remaining, "");
    }
    #[test]
    fn test_range_end_with_expression_semicolon() {
        let input = ".. T::MaxRegistrars::get() - 1;";
        let (remaining, result) = ParamParser::range_end(input).unwrap();
        assert_eq!(result, Expression("T::MaxRegistrars::get() - 1".to_string()));
        assert_eq!(remaining, "");
    }
    #[test]
    fn test_range_end_with_expression_arrow() {
        let input = ".. T::MaxRegistrars::get() =>";
        let (remaining, result) = ParamParser::range_end(input).unwrap();
        assert_eq!(result, Expression("T::MaxRegistrars::get()".to_string()));
        assert_eq!(remaining, "");
    }

    #[test]
    fn test_dispatch_range_start_with_range_operator() {
        let input = "let r in 1 ..";
        match ParamParser::dispatch(input) {
            Ok((remaining, Range::Start(value))) => {
                assert_eq!(value, 1);
            },
            Err(e) => panic!("Failed to dispatch to range_start with range operator: {:?}", e),
            _ => {}
        }
    }

    #[test]
    fn test_dispatch_range_start_without_range_operator() {
        let input = "let r in 1";
        match ParamParser::dispatch(input) {
            Ok((remaining, Range::Start(value))) => {
                assert_eq!(value, 1);
            },
            Err(e) => panic!("Failed to dispatch to range_start without range operator: {:?}", e),
            _ => {}
        }
    }
}
