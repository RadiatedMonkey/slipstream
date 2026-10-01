use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, Token, Type, Visibility};
use syn::{PathSegment, Result};

#[derive(Debug, darling::FromField)]
#[darling(attributes(inspect))]
struct FieldOpt {
    pub ident: Option<Ident>,
    pub vis: Visibility,
    pub ty: Type,

    #[darling(default)]
    pub min: Option<syn::Expr>,
    #[darling(default)]
    pub max: Option<syn::Expr>,
    #[darling(default)]
    pub read_only: bool,
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
        let Self {
            ident,
            ty,
            read_only,
            min,
            max,
            ..
        } = self;

        let read_only = *read_only;
        let range = match (min, max) {
            (None, None) => quote! {
                // explicit type annotations are required here due to both nones.
                None
            },
            (Some(min), Some(max)) => {
                quote! {
                    {
                        use slipstream_shared::inspect::IntoBounds;

                        // verify that this type support bounds.
                        const _: () = {
                            const fn assert_impl<T: ?Sized + IntoBounds<#ty>>() {}
                            let _ = assert_impl::<#ty>();
                        };
                        // #ty is specified twice because we need to both specify the generic and the impl we want to use.
                        Some(<#ty as IntoBounds::<#ty>>::into_bounds(Some(#min), Some(#max)))
                    }
                }
            }
            (Some(min), None) => quote! {
                {
                    use slipstream_shared::inspect::IntoBounds;

                    // verify that this type support bounds.
                    const _: () = {
                        const fn assert_impl<T: ?Sized + IntoBounds<#ty>>() {}
                        let _ = assert_impl::<#ty>();
                    };
                    // #ty is specified twice because we need to both specify the generic and the impl we want to use.
                    Some(<#ty as IntoBounds::<#ty>>::into_bounds(Some(#min), None))
                }
            },
            (None, Some(max)) => quote! {
                {
                    use slipstream_shared::inspect::IntoBounds;

                    // verify that this type support bounds.
                    const _: () = {
                        const fn assert_impl<T: ?Sized + IntoBounds<#ty>>() {}
                        let _ = assert_impl::<#ty>();
                    };
                    // #ty is specified twice because we need to both specify the generic and the impl we want to use.
                    Some(<#ty as IntoBounds::<#ty>>::into_bounds(None, Some(#max)))
                }
            },
        };

        quote! {
            {
                use slipstream_shared::inspect::{AsFieldValue, FieldConfig};
                <#ty as AsFieldValue>::as_field_value(&mut self.#ident, &FieldConfig {
                    range: #range,
                    read_only: #read_only
                })
            }
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
            ..
        } = self;

        let is_hidden = hidden.unwrap_or(!matches!(vis, Visibility::Public(_)));
        if !is_hidden {
            let name = match rename {
                Some(rename) => quote! { #rename },
                None => {
                    let ident = ident
                        .clone()
                        .unwrap_or_else(|| Ident::new("<unknown>", Span::call_site()));

                    quote! { stringify!(#ident) }
                }
            };

            let field_value = self.construct_field_value();
            let category = category
                .as_deref()
                .map(|c| quote! { Some(#c) })
                .unwrap_or_else(|| quote! { None });

            tokens.append_all(quote! {
                slipstream_shared::inspect::InspectFieldHelper {
                    label: #name,
                    category: #category,
                    value: #field_value
                }
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
            .map(|s| Ident::new(s, Span::call_site()))
            .unwrap_or(ident.clone());

        let data = data.as_struct().expect("input was not a struct");
        if data.style != darling::ast::Style::Struct {
            panic!(
                "{}",
                darling::error::Error::unsupported_shape("non-field struct")
            );
        }

        let fields = &data.fields;

        tokens.append_all(quote! {
            /// Automatically generated by the [`Inspect`] derive macro.
            ///
            /// This function generates an abstract representation of the current struct
            // #ty is specified twice because we need to both specify the generic and the impl we want to use.
            impl slipstream_shared::inspect::Inspect for #ident {
                #[inline]
                fn label(&self) -> &str {
                    stringify!(#ident)
                }

                #[inline]
                fn draw(&mut self, draw_fn: &mut dyn Fn(&mut [slipstream_shared::inspect::InspectFieldHelper<'_>])) {
                    draw_fn(&mut [#(#fields),*]);
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
    let input = Input::from_derive_input(&input).expect("failed to parse input");
    let tokens = input.into_token_stream();

    #[cfg(debug_assertions)]
    {
        if let Ok(parsed) = syn::parse2::<syn::File>(tokens.clone()) {
            eprintln!("{}", prettyplease::unparse(&parsed));
        } else {
            eprintln!("RAW OUTPUT: {}", tokens.to_string());
        }
    }

    TokenStream::from(tokens)
}
