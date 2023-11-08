mod parser;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use regex;
use std::fmt::{Display, Formatter};
use std::{fmt, fs, iter::Peekable, result::Result};
use syn::parse::Parser;
use syn::{
    braced,
    parse::{Parse, ParseStream, Result as ParseResult},
    parse2, parse_quote,
    punctuated::Punctuated,
    token::Semi,
    visit_mut::VisitMut,
    Attribute, Block, Expr, File, FnArg, Generics, Ident, Item, ItemFn, ItemMacro, ItemMod,
    ReturnType, Signature, Stmt, Token, Visibility,
};

struct Items(Vec<Item>);
fn main() {}
