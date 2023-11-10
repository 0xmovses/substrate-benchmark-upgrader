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
    Err as NomErr,
};

use crate::lexer::{BenchmarkLine, LineKind};
use anyhow::{Result, anyhow};

pub struct ParamParser;
pub struct ParamWriter;
#[derive(Debug)]
pub struct BenchmarkParameter {
    pub name: String,
    pub range_start: u8,
    pub range_end: String,
}


impl Default for BenchmarkParameter {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            range_start: 0,
            range_end: "".to_string(),
        }
    }
}

impl ParamParser {
    pub fn dispatch(input: &str) -> Result<BenchmarkLine> {
        if input.trim_start().starts_with("let ") {
            if input.trim_start().contains("=") && !input.trim_start().contains("=>"){
               Ok(BenchmarkLine {
                     head: None,
                     kind: LineKind::Content,
                     content: Some(input.to_string()),
                     param_content: None,
                     fn_body: None,
               })
            } else {
                match Self::let_declaration(input) {
                    Ok((_str, param)) => {
                        let param = BenchmarkParameter {
                            name: param.name,
                            range_start: param.range_start,
                            range_end: param.range_end,
                        };
                        Ok(BenchmarkLine {
                            head: None,
                            kind: LineKind::FnParam,
                            content: None,
                            param_content: Some(param),
                            fn_body: None,
                        })
                    }
                    Err(e) => {
                        Err(anyhow!("Error parsing parameter: {:?}", e))
                    }
                }
            }
        } else {
            Err(anyhow!("Error parsing parameter"))
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
        println!("name: {:?}, range_start: {:?}, range_end: {:?}", name, range_start_val, range_end_val);

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

impl ParamWriter {
    pub fn fn_input(param: &BenchmarkParameter) -> String {
        let range_end_expression = param.range_end.split("=>").next().unwrap_or("").trim();

        // Determine if range_end is a numeric constant or an expression.
        let is_numeric_constant = range_end_expression.parse::<u64>().is_ok();

        // For numeric constants, we don't add curly braces. For expressions, we do.
        let range_end = if is_numeric_constant {
            range_end_expression.to_string()
        } else {
            format!("{{ {} }}", range_end_expression)
        };

        // Format the parameter string.
        format!("{}: Linear<{}, {}>,", param.name, param.range_start, range_end)
    }

    pub fn fn_gen(param_input: String, fn_signature: &String) -> Result<String> {
        if let Some(open_paren_pos) = fn_signature.find('(') {
            if let Some(close_paren_pos) = fn_signature[open_paren_pos..].find(')') {
                let close_paren_pos = open_paren_pos + close_paren_pos;

                let start = &fn_signature[..open_paren_pos + 1];
                let end = &fn_signature[close_paren_pos..];

                // Construct the new function signature with the parameter input inserted.
                return Ok(format!("{}{}{}", start, param_input, end));
            }
        }
        Err(anyhow!("Error: The function signature does not have the correct format."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_param_declaration_with_expression() {
        let input = "let r in 1 .. T::MaxRegistrars::get() =>";
        match ParamParser::let_declaration(input) {
            Ok((str, param)) => {
                assert_eq!(param.name, "r");
                assert_eq!(param.range_start, 1);
                assert_eq!(param.range_end, "T::MaxRegistrars::get()");
            }
            Err(e) => panic!("Parsing failed when it should have succeeded: {:?}", e),
        }
    }

    #[test]
    fn test_parse_param_declaration_with_constant() {
        let input = "let b in 1 .. MAX_BYTES;";
        match ParamParser::let_declaration(input) {
            Ok((str, param)) => {
                assert_eq!(param.name, "b");
                assert_eq!(param.range_start, 1);
                assert_eq!(param.range_end, "MAX_BYTES");
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

    //#[test]
    fn test_writer_fn_input() {
        let params = vec![
            BenchmarkParameter {
                name: "b".to_string(),
                range_start: 1,
                range_end: "MAX_BYTES".to_string(),
            },
            BenchmarkParameter {
                name: "m".to_string(),
                range_start: 2,
                range_end: "T::MaxFellows::get()".to_string(),
            },
            BenchmarkParameter {
                name: "p".to_string(),
                range_start: 1,
                range_end: "T::MaxProposals::get()".to_string(),
            },
        ];

        let expected_outputs = vec![
            "b: Linear<1, MAX_BYTES>,",
            "m: Linear<2, { T::MaxFellows::get() }>,",
            "p: Linear<1, { T::MaxProposals::get() }>,",
        ];

        for (param, expected) in params.iter().zip(expected_outputs.iter()) {
            assert_eq!(ParamWriter::fn_input(param), *expected);
        }
    }
}
