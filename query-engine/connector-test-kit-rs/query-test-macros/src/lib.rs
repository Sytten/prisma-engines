extern crate proc_macro;

use darling::FromMeta;
use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use query_tests_setup::{ConnectorTag, ConnectorTagInterface, TestError};
use quote::quote;
use std::convert::TryFrom;
use syn::{parse_macro_input, spanned::Spanned, AttributeArgs, ItemFn, Meta, Path};

#[proc_macro_attribute]
pub fn connector_test(attr: TokenStream, input: TokenStream) -> TokenStream {
    connector_test_impl(attr, input)
}

fn connector_test_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = ConnectorTestArgs::from_list(&attributes_meta);
    let args = match args {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    if let Err(err) = args.validate() {
        return err.write_errors().into();
    };

    let connectors = args.connectors_to_test();
    let handler = args.schema.unwrap().handler_path;

    let connectors = connectors.into_iter().map(quote_connector).fold1(|aggr, next| {
        quote! {
            #aggr, #next
        }
    });

    // The shell function retains the name of the original test definition.
    let mut test_function = parse_macro_input!(input as ItemFn);
    let test_fn_ident = test_function.sig.ident.clone();

    // Rename original test function to run_<orig_name>.
    let runner_fn_ident = Ident::new(&format!("run_{}", test_fn_ident.to_string()), Span::call_site());
    test_function.sig.ident = runner_fn_ident.clone();

    let test = quote! {
        #[test]
        fn #test_fn_ident() {
            let config = &query_tests_setup::CONFIG;
            let schema = #handler();
            let runner = Runner::try_from(config.runner()).unwrap();

            let connectors = vec![
                #connectors
            ];

            println!("{:?}", connectors);

            #runner_fn_ident(&runner)
        }

        #test_function
    };

    test.into()
}

#[derive(Debug, FromMeta)]
struct ConnectorTestArgs {
    #[darling(default)]
    schema: Option<SchemaHandler>,

    #[darling(default)]
    only: OnlyConnectorTags,

    #[darling(default)]
    exclude: ExcludeConnectorTags,
}

impl ConnectorTestArgs {
    pub fn validate(&self) -> Result<(), darling::Error> {
        if !self.only.is_empty() && !self.exclude.is_empty() {
            return Err(darling::Error::custom(
                "Only one of `only` and `exclude` can be speficified for a connector test.",
            ));
        }

        Ok(())
    }

    /// Returns all the connectors that the test is valid for.
    pub fn connectors_to_test(&self) -> Vec<ConnectorTag> {
        if !self.only.is_empty() {
            self.only.tags.clone()
        } else if !self.exclude.is_empty() {
            todo!()
        } else {
            todo!()
        }
    }
}

#[derive(Debug)]
struct SchemaHandler {
    handler_path: Path,
}

impl darling::FromMeta for SchemaHandler {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        if items.len() != 1 {
            return Err(darling::Error::unsupported_shape(
                "Expected `schema` to contain exactly one function pointer to a schema handler.",
            )
            .with_span(&Span::call_site()));
        }

        let item = items.first().unwrap();
        match item {
            syn::NestedMeta::Meta(Meta::Path(p)) => Ok(Self {
                // Todo validate signature somehow
                handler_path: p.clone(),
            }),
            x => Err(darling::Error::unsupported_shape(
                "Expected `schema` to be a function pointer to a schema handler function.",
            )
            .with_span(&x.span())),
        }
    }
}

#[derive(Debug, Default)]
struct OnlyConnectorTags {
    tags: Vec<ConnectorTag>,
}

impl OnlyConnectorTags {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

#[derive(Debug, Default)]
struct ExcludeConnectorTags {
    tags: Vec<ConnectorTag>,
}

impl ExcludeConnectorTags {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

impl darling::FromMeta for OnlyConnectorTags {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let tags = tags_from_list(items)?;
        Ok(OnlyConnectorTags { tags })
    }
}

impl darling::FromMeta for ExcludeConnectorTags {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let tags = tags_from_list(items)?;
        Ok(ExcludeConnectorTags { tags })
    }
}

fn tags_from_list(items: &[syn::NestedMeta]) -> Result<Vec<ConnectorTag>, darling::Error> {
    if items.is_empty() {
        return Err(darling::Error::custom("At least one connector tag is required."));
    }

    let mut tags: Vec<ConnectorTag> = vec![];

    for item in items {
        match item {
            syn::NestedMeta::Meta(meta) => {
                match meta {
                    // A single variant without version, like `Postgres`.
                    Meta::Path(p) => {
                        let tag = tag_string_from_path(p)?;
                        tags.push(ConnectorTag::try_from(tag.as_str()).into_darling_error(&p.span())?);
                    }
                    Meta::List(l) => {
                        let tag = tag_string_from_path(&l.path)?;
                        for meta in l.nested.iter() {
                            match meta {
                                syn::NestedMeta::Lit(literal) => {
                                    let version_str = match literal {
                                        syn::Lit::Str(s) => s.value(),
                                        syn::Lit::Char(c) => c.value().to_string(),
                                        syn::Lit::Int(i) => i.to_string(),
                                        syn::Lit::Float(f) => f.to_string(),
                                        x => {
                                            return Err(darling::Error::unexpected_type(
                                                "Versions can be string, char, int and float.",
                                            )
                                            .with_span(&x.span()))
                                        }
                                    };

                                    tags.push(
                                        ConnectorTag::try_from((tag.as_str(), Some(version_str.as_str())))
                                            .into_darling_error(&l.span())?,
                                    );
                                }
                                syn::NestedMeta::Meta(meta) => {
                                    return Err(darling::Error::unexpected_type(
                                        "Versions can only be literals (string, char, int and float).",
                                    )
                                    .with_span(&meta.span()));
                                }
                            }
                        }
                    }
                    _ => unimplemented!(),
                }
            }
            x => {
                return Err(
                    darling::Error::custom("Expected `only` to be a list of `ConnectorTag`.").with_span(&x.span()),
                )
            }
        }
    }

    Ok(tags)
}

fn tag_string_from_path(path: &Path) -> Result<String, darling::Error> {
    if let Some(ident) = path.get_ident() {
        let name = ident.to_string();

        Ok(name)
    } else {
        Err(darling::Error::custom(
            "Expected `only` to be a list of idents (ConnectorTag variants), not paths.",
        ))
    }
}

fn quote_connector(tag: ConnectorTag) -> proc_macro2::TokenStream {
    let (connector, version) = tag.as_parse_pair();

    match version {
        Some(version) => quote! {
            ConnectorTag::try_from((#connector, Some(#version))).unwrap()
        },
        None => quote! {
            ConnectorTag::try_from(#connector).unwrap()
        },
    }
}

trait IntoDarlingError<T> {
    fn into_darling_error(self, span: &Span) -> std::result::Result<T, darling::Error>;
}

impl<T> IntoDarlingError<T> for std::result::Result<T, TestError> {
    fn into_darling_error(self, span: &Span) -> std::result::Result<T, darling::Error> {
        self.map_err(|err| match err {
            TestError::ParseError(msg) => darling::Error::custom(&format!("Parsing error: {}.", msg)).with_span(span),
            TestError::ConfigError(msg) => {
                darling::Error::custom(&format!("Configuration error: {}.", msg)).with_span(span)
            }
        })
    }
}