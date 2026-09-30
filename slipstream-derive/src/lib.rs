use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::Result;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, Token, Type, Visibility};

#[derive(Debug, darling::FromField)]
#[darling(attributes(inspect))]
struct FieldOpt {
    pub ident: Option<Ident>,
    pub vis: Visibility,
    pub ty: Type,

    #[darling(default)]
    pub category: Option<String>,
    #[darling(default)]
    pub hidden: Option<bool>,
    #[darling(default)]
    pub rename: Option<String>,
    #[darling(default)]
    pub with: Option<syn::Path>,
}

impl FieldOpt {
    fn construct_field_value(&self) -> proc_macro2::TokenStream {
        match &self.ty {
            Type::Array(ty_array) => {
                quote! {
                    todo!("field value array")
                }
            }
            Type::Path(ty_path) => {
                quote! {
                    todo!("field value ty path")
                }
            }
            _ => unimplemented!(),
        }
    }
}

impl ToTokens for FieldOpt {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            ident,
            ty,
            vis,
            category,
            hidden,
            rename,
            with,
        } = self;

        let is_hidden = hidden.unwrap_or(!matches!(vis, Visibility::Public(_)));
        if !is_hidden {
            // `rename` takes priority over the actual field name.
            let name = rename
                .as_ref()
                .map(|r| Ident::new(r, rename.span()))
                .unwrap_or_else(|| {
                    ident
                        .clone()
                        .unwrap_or_else(|| Ident::new("<unknown>", Span::call_site()))
                });

            let field_value = self.construct_field_value();

            tokens.append_all(quote! {
                let inspectable = crate::shared::inspect::InspectFieldHelper {
                    label: stringify!(#name),
                    category: #category,
                    value: #field_value
                };

                inspectable.draw(&mut self, ui);
            });
        }
    }
}

#[derive(Debug, darling::FromVariant)]
struct FieldVariant {
    pub ident: Ident,
    pub vis: Visibility,

    #[darling(default)]
    pub rename: Option<String>,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(inspect))]
struct Input {
    pub ident: Ident,
    pub data: darling::ast::Data<FieldVariant, FieldOpt>,

    #[darling(default)]
    pub label: Option<String>,
}

impl ToTokens for Input {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self { ident, data, label } = self;

        let label = label
            .as_ref()
            .map(|s| Ident::new(&s, Span::call_site()))
            .unwrap_or(ident.clone());

        let data = data.as_struct().unwrap();
        if data.style != darling::ast::Style::Struct {
            panic!(
                "{}",
                darling::error::Error::unsupported_shape("non-field struct")
            );
        }

        let fields = &data.fields;

        tokens.append_all(quote! {
            impl crate::shared::inspect::Inspect for #ident {
                const LABEL: &str = stringify!(#label);

                fn draw_properties(&self, ui: &mut egui::Ui) -> slipstream_shared::SlipstreamResult<()> {
                    #(#fields)*

                    Ok(())
                }
            }
        });
    }
}

/// Fields not marked with `pub` will not be shown in the property window by default.
/// This can be overriden with `hidden = false`, the attribute will always take priority over field
/// visibility.
#[proc_macro_derive(Inspect, attributes(inspect))]
pub fn derive_inspect(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let input = Input::from_derive_input(&input).unwrap();

    input.into_token_stream().into()
}

struct CategoryInput {
    pub vis: Visibility,
    pub ident: Ident,
    pub enum_token: Token![enum],
}

impl Parse for CategoryInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis = input.parse()?;
        let ident = input.parse()?;
        let enum_token = input.parse()?;

        Ok(Self {
            vis,
            ident,
            enum_token,
        })
    }
}

impl ToTokens for CategoryInput {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self { ident, .. } = self;

        tokens.append_all(quote! {
            impl #ident {

            }
        });
    }
}

#[proc_macro_derive(InspectCategory)]
pub fn derive_inspect_category(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as CategoryInput);
    input.to_token_stream().into()
}
