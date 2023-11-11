use quote::ToTokens;
use syn::parse::{Parse, ParseBuffer, ParseStream, Result};
use syn::{Attribute, Expr, Type, Generics, Path, PathArguments, PathSegment, Ident, Token, punctuated::Punctuated, TypePath};
use proc_macro2::{TokenStream};

pub struct ExtrinsicCall {
    attribute: Vec<Attribute>,
    underscore: Token![_],
    runtime_origin: Type,
    args: Punctuated<Expr, Token![,]>,
}

impl Parse for ExtrinsicCall {
    fn parse(input: ParseStream) -> Result<Self> {
        let attribute = Attribute::parse_outer(input)?;

        let underscore: Token![_] = input.parse()?;
        let runtime_origin: Type = {
            input.parse::<Token![<]>()?;
            let ty: Type = input.parse()?;
            input.parse::<Token![>]>()?;
            ty
        };
        let mut content;
        let _paren_token = syn::parenthesized!(content in input);
        let args = content.parse_terminated(Expr::parse)?;

        Ok(ExtrinsicCall {
            attribute,
            underscore,
            runtime_origin,
            args,
        })
    }
}

impl ToTokens for ExtrinsicCall {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for attr in &self.attribute {
            attr.to_tokens(tokens);
        }
        self.underscore.to_tokens(tokens);
        self.runtime_origin.to_tokens(tokens);
        syn::token::Paren::default().surround(tokens, |tokens| {
            self.args.to_tokens(tokens);
        });
    }
}

