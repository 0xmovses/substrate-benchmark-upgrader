use nom::IResult;

pub struct BenchmarkParser;

pub enum ParsedElement {
    Block(String),
    FnParam(String),
}

impl BenchmarkParser {
}