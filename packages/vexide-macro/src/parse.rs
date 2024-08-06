use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Result, Token,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(banner);
}

pub struct MacroOpts {
    pub banner: bool,
}

impl Default for MacroOpts {
    fn default() -> Self {
        Self {
            banner: true,
        }
    }
}

impl From<Attrs> for MacroOpts {
    fn from(value: Attrs) -> Self {
        let mut opts = Self::default();
        for attr in value.attr_list {
            match attr {
                Attribute::Banner(banner) => opts.banner = banner.as_bool(),
            }
        }
        opts
    }
}

pub struct Attrs {
    attr_list: Punctuated<Attribute, Token![,]>,
}

impl Parse for Attrs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            attr_list: Punctuated::parse_terminated(input)?,
        })
    }
}

pub enum Attribute {
    Banner(Banner),
}

impl Parse for Attribute {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::banner) {
            input.parse().map(Attribute::Banner)
        } else {
            Err(lookahead.error())
        }
    }
}

pub struct Banner {
    token: kw::banner,
    eq: Token![=],
    arg: syn::LitBool,
}

impl Banner {
    pub const fn as_bool(&self) -> bool {
        self.arg.value
    }
}

impl Parse for Banner {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            token: input.parse()?,
            eq: input.parse()?,
            arg: input.parse()?,
        })
    }
}

impl ToTokens for Banner {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.token.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.arg.to_tokens(tokens);
    }
}

#[cfg(test)]
mod test {
    use quote::quote;
    use syn::Ident;

    use super::*;

    #[test]
    fn parses_banner_attribute() {
        let source = quote! {
            banner = true
        };
        let input = syn::parse2::<Banner>(source).unwrap();
        assert_eq!(input.as_bool(), true);
    }

    #[test]
    fn parses_attrs_into_macro_opts() {
        let source = quote! {
            banner = true
        };
        let input = syn::parse2::<Attrs>(source).unwrap();
        assert_eq!(input.attr_list.len(), 1);
        let opts = MacroOpts::from(input);
        assert_eq!(opts.banner, true);
    }
}
