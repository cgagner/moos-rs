extern crate proc_macro;

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::quote;
use std::panic::{self, catch_unwind};
use syn::{parse_macro_input, spanned::Spanned, token::Token, DeriveInput};

#[proc_macro_attribute]
pub fn request_handler(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    if let syn::Data::Enum(e) = ast.data {
        let ident = ast.ident;
        let old_variants = e.variants.clone();

        let new_variants: Vec<_> = old_variants
            .iter()
            .map(|v| {
                let ident = v.ident.clone();
                quote!(
                    #ident(RequestId, <#ident as lsp_types::request::Request>::Params),
                )
            })
            .collect();

        let variant_conversions: Vec<_> = old_variants
            .iter()
            .map(|v| {
                let ident = v.ident.clone();
                quote!(
                    #ident::METHOD => {
                        match serde_json::from_value::<<#ident as lsp_types::request::Request>::Params>(
                            request.params,
                        ) {
                            Ok(params) => Self::#ident(request.id, params),
                            Err(e) => Self::Error {
                                method: #ident::METHOD,
                                error: e,
                            },
                        }
                    }
                )
            })
            .collect();

        return TokenStream::from(quote!(
            enum #ident {
                #(#new_variants)*
                Unhandled(lsp_server::Request),
                Error {
                    method: &'static str,
                    error: serde_json::Error,
                },
            }
            // Convert from an lsp_server::Request to a #ident variant.
            impl From<lsp_server::Request> for #ident {
                fn from(request: lsp_server::Request) -> Self {
                    use lsp_types::request::Request;
                    let method = request.method.as_str();
                    match method {
                        #(#variant_conversions)*
                        _ => Self::Unhandled(request),
                    }
                }
            }
        ));
    } else {
        panic!("request_handler can only be used for enums")
    }
}

#[proc_macro_attribute]

pub fn notification_handler(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    if let syn::Data::Enum(e) = ast.data {
        let ident = ast.ident;
        let old_variants = e.variants.clone();
        let new_variants: Vec<_> = old_variants
            .iter()
            .map(|v| {
                let ident = v.ident.clone();
                quote!(
                    #ident(<#ident as lsp_types::notification::Notification>::Params),
                )
            })
            .collect();

        let variant_conversions: Vec<_> = old_variants
            .iter()
            .map(|v| {
                let ident = v.ident.clone();
                quote!(
                    #ident::METHOD => {
                        match serde_json::from_value::<<#ident as lsp_types::notification::Notification>::Params>(
                            notification.params,
                        ) {
                            Ok(params) => Self::#ident(params),
                            Err(e) => Self::Error {
                                method: #ident::METHOD,
                                error: e,
                            },
                        }
                    }
                )
            })
            .collect();

        return TokenStream::from(quote!(
            enum #ident {
                #(#new_variants)*
                Unhandled(lsp_server::Notification),
                Error {
                    method: &'static str,
                    error: serde_json::Error,
                },
            }
            // Convert from an lsp_server::Notification to a #ident variant.
            impl From<lsp_server::Notification> for #ident {
                fn from(notification: lsp_server::Notification) -> Self {
                    use lsp_types::notification::Notification;
                    let method = notification.method.as_str();
                    match method {
                        #(#variant_conversions)*
                        _ => Self::Unhandled(notification),
                    }
                }
            }
        ));
    } else {
        panic!("notification_handler can only be used for enums")
    }
}

// Copied from Blog post
// https://internals.rust-lang.org/t/custom-error-diagnostics-with-procedural-macros-on-almost-stable-rust/8113

// fn error(s: &str, start: Span, end: Span) -> TokenStream {
//     let mut v = Vec::new();
//     v.push(respan(Literal::string(&s), Span::call_site()));
//     let group = v.into_iter().collect();

//     let mut r = Vec::<TokenTree>::new();
//     r.push(respan(Ident::new("compile_error", start), start));
//     r.push(respan(Punct::new('!', Spacing::Alone), Span::call_site()));
//     r.push(respan(Group::new(Delimiter::Brace, group), end));

//     r.into_iter().collect()
// }

// fn respan<T: Into<TokenTree>>(t: T, span: Span) -> TokenTree {
//     let mut t = t.into();
//     t.set_span(span);
//     t
// }
