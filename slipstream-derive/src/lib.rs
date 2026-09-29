use proc_macro::TokenStream;
use darling::FromField;
use proc_macro2::Span;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{braced, parse::{Parse, ParseStream}, punctuated::Punctuated, token::Comma, Ident, Result, Token, Type, Visibility, TypeTuple, Field};

#[derive(darling::FromField)]
#[darling(attributes(inspect))]
struct FieldOpts {
    pub ident: Option<Ident>,
    pub ty: Type,
    pub category: Option<String>,
    pub hidden: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub with: Option<syn::Path>
}

struct InputStruct {
    pub vis: Visibility,
    pub struct_token: Token![struct],
    pub name: Ident,
    pub fields: Vec<FieldOpts>
}

impl Parse for InputStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis = input.parse()?;
        let struct_token = input.parse()?;
        let name = input.parse()?;

        let content;
        let braced = braced!(content in input);

        let mut errors = darling::Error::accumulator();
        let fields = content
            .parse_terminated(Field::parse_named, Token![,])?
            .into_iter()
            .filter_map(|field| {
                errors.handle_in(|| FieldOpts::from_field(&field))
            })
            .collect::<Vec<_>>();

        errors.finish()?;
        Ok(Self {
            vis, struct_token, name, fields
        })
    }
}

impl ToTokens for InputStruct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            vis, name, struct_token, fields
        } = self;

        tokens.append_all(quote! {
            impl crate::shared::inspect::Inspect for #name {
                fn draw_properties(&mut self, ui: &mut egui:Ui) -> slipstream_shared::SlipstreamResult<()> {
                    ui.label(concat!("This is a ", stringify!(#name)));

                    Ok(())
                }
            }
        });
    }
}

struct InputEnum {
    pub vis: Visibility,
    pub name: Ident
}

impl Parse for InputEnum {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis = input.parse()?;
        input.parse::<Token![enum]>()?;
        let name = input.parse()?;

        Ok(Self {
            vis, name
        })
    }
}

impl ToTokens for InputEnum {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {}
}

enum Input {
    Struct(InputStruct),
    Enum(InputEnum)
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.fork();
        lookahead.parse::<Visibility>()?;

        Ok(if lookahead.peek(Token![enum]) {
            Input::Enum(InputEnum::parse(input)?)
        } else if lookahead.peek(Token![struct]) {
            Input::Struct(InputStruct::parse(input)?)
        } else {
            return Err(input.error("expected enum or struct"))
        })
    }
}

impl ToTokens for Input {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Struct(x) => x.to_tokens(tokens),
            Self::Enum(x) => x.to_tokens(tokens)
        }
    }
}

#[proc_macro_derive(Inspect, attributes(inspect))]
pub fn derive_inspect(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as Input);
    input.into_token_stream().into()
}