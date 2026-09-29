use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Ident, Result, Token, Visibility, braced,
    parse::{Parse, ParseStream},
};

// #[derive(Inspect)]
// struct Properties {
//     pub vector: [f32; 3],
// }
//
// trait Inspect {
//     fn label(&self) -> &str;
//
//     fn draw(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()> {
//         todo!()
//     }
// }
//
// struct InspectField {
//     pub visibility: Visibility,
//     pub name: Ident,
//     pub ty: Ident,
// }
//
// impl Parse for InspectField {
//     fn parse(input: ParseStream) -> Result<Self> {
//         let visibility = input.parse()?;
//         let name = input.parse()?;
//
//         input.parse::<Token![:]>()?;
//
//         let ty = input.parse()?;
//
//         Ok(Self {
//             visibility,
//             name,
//             ty,
//         })
//     }
// }
//
// struct IStruct {
//     pub visibility: Visibility,
//     pub name: Ident,
//     pub fields: Vec<InspectField>,
// }
//
// impl Parse for IStruct {
//     fn parse(input: ParseStream) -> Result<Self> {
//         let visibility = input.parse()?;
//         input.parse::<Token![struct]>()?;
//         let name = input.parse()?;
//
//         let fields;
//         let braced = braced!(fields in input);
//
//         fields.parse_terminated(InspectField::parse, Token![,])?;
//
//         Ok(Self {
//             visibility,
//             name,
//             fields,
//         })
//     }
// }

struct IVariant {
    pub name: Ident,
}

impl Parse for IVariant {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;

        Ok(Self { name })
    }
}

struct IEnum {
    pub visibility: Visibility,
    pub name: Ident,
    pub variants: Vec<Ident>,
}

impl IEnum {
    pub fn expand(self) -> TokenStream {
        let Self {
            visibility,
            name,
            variants,
        } = self;

        TokenStream::from(quote! {
            #visibility enum #name {
                #(#variants),*
            }
        })
    }
}

impl Parse for IEnum {
    fn parse(input: ParseStream) -> Result<Self> {
        let visibility = input.parse()?;
        input.parse::<Token![enum]>()?;
        let name = input.parse()?;

        let variants;
        let braced = braced!(variants in input);

        let variants = variants
            .parse_terminated(IVariant::parse, Token![,])?
            .into_iter()
            .map(|var| var.name)
            .collect::<Vec<_>>();

        Ok(Self {
            visibility,
            name,
            variants,
        })
    }
}

enum InspectInput {
    // Struct(IStruct),
    Enum(IEnum),
}

impl InspectInput {
    pub fn expand(self) -> TokenStream {
        match self {
            Self::Enum(x) => x.expand(),
        }
    }
}

impl Parse for InspectInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.peek2(Token![struct]) {
            todo!();
            // Self::Struct(IStruct::parse(input)?)
        } else if input.peek2(Token![enum]) {
            Self::Enum(IEnum::parse(input)?)
        } else {
            todo!("Return error");
        })
    }
}

#[proc_macro_derive(Inspect)]
pub fn derive_inspect(tokens: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(tokens as InspectInput);
    input.expand()
}
