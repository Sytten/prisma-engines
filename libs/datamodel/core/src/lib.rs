#![allow(clippy::upper_case_acronyms)]

//! This crate is responsible for parsing, rendering and formatting a Prisma Schema.
//! A Prisma Schema consists out of two parts:
//! 1. The `Datamodel` part refers to the model and enum definitions.
//! 2. The `Configuration` part refers to the generator and datasource definitions.
//!
//! The data structures are organized into 3 layers:
//! * The AST layer contains the data data structures representing the schema input.
//! * The model layer contains the data structures that are semantically rich and therefore engines can build upon.
//! * The JSON layer contains the data structures that represent the contract for the DMMF which is the API for client generators.
//!
//! The responsibilities of each top level module is as following:
//! * `common`: contains constants and generic helpers
//! * `error`: contains the error and result types
//! * `ast`: contains the data structures for the AST of a Prisma schema. And the parsing functions to turn an input string into an AST.
//! * `dml`: contains the models representing the Datamodel part of a Prisma schema
//! * `configuration`: contains the models representing the Datasources and Generators of a Prisma schema
//! * `transform`: contains the logic to turn an AST into models and vice versa
//! * `json`: contains the logic to turn models into their JSON/DMMF representation
//!
//! The flow between the layers is depicted in the following diagram.
//!<pre>
//!                ┌──────────────────┐
//!                │       json       │
//!                └────────▲─────────┘
//!                         │
//!                         │
//!      ┌──────────────────┐┌──────────────────┐
//!      │       dml        ││  configuration   │
//!      └──────────────────┘└──────────────────┘
//!                        │  ▲
//!┌─────────────────────┐ │  │ ┌─────────────────────┐
//!│transform::dml_to_ast│ │  │ │transform::ast_to_dml│
//!└─────────────────────┘ │  │ └─────────────────────┘
//!                        │  │
//!                        ▼  │
//!                ┌──────────────────┐
//!                │       ast        │
//!                └──────────────────┘
//!                        │  ▲
//!                        │  │
//!                        │  │
//!                        ▼  │
//!                 ┌──────────────────┐
//!                 │  schema string   │
//!                 └──────────────────┘
//!</pre>
//!
//! The usage dependencies between the main modules is depicted in the following diagram.
//! The modules `error` and `common` are not shown as any module may depend on them.
//!<pre>
//!                       ┌──────────────────┐
//!                       │    transform     │
//!                       └──────────────────┘
//!                                 │
//!                                 │ use
//!          ┌──────────────────────┼──────────────────────────┐
//!          │                      │                          │
//!          │                      │                          │
//!          ▼                      ▼                          ▼
//!┌──────────────────┐   ┌──────────────────┐       ┌──────────────────┐
//!│       ast        │   │       dml        │       │  configuration   │
//!└──────────────────┘   └──────────────────┘       └──────────────────┘
//!                                 ▲                          ▲
//!                                 │                          │
//!                                 ├──────────────────────────┘
//!                                 │ use
//!                                 │
//!                       ┌──────────────────┐
//!                       │       json       │
//!                       └──────────────────┘
//!</pre>
//!

#![allow(clippy::module_inception, clippy::suspicious_operation_groupings)]

pub mod ast;
pub mod common;
pub mod configuration;
pub mod diagnostics;
pub mod dml;
pub mod json;
pub mod transform;
pub mod walkers;

use std::path::Path;

pub use crate::dml::*;
pub use configuration::*;

use crate::ast::SchemaAst;
use crate::diagnostics::{ValidatedConfiguration, ValidatedDatamodel, ValidatedDatasources};
use transform::{
    ast_to_dml::{DatasourceLoader, GeneratorLoader, ValidationPipeline},
    dml_to_ast::{DatasourceSerializer, GeneratorSerializer, LowerDmlToAst},
};

/// Parses and validates a datamodel string, using core attributes only.
pub fn parse_datamodel(datamodel_string: &str) -> Result<ValidatedDatamodel, diagnostics::Diagnostics> {
    parse_datamodel_internal(datamodel_string, false, false)
}

pub fn parse_datamodel_and_ignore_datasource_urls(
    datamodel_string: &str,
) -> Result<ValidatedDatamodel, diagnostics::Diagnostics> {
    parse_datamodel_internal(datamodel_string, true, false)
}

pub fn parse_datamodel_and_ignore_datasource_urls_for_formatter(
    datamodel_string: &str,
) -> Result<ValidatedDatamodel, diagnostics::Diagnostics> {
    parse_datamodel_internal(datamodel_string, true, true)
}

/// Parses and validates a datamodel string, using core attributes only.
/// In case of an error, a pretty, colorful string is returned.
pub fn parse_datamodel_or_pretty_error(datamodel_string: &str, file_name: &str) -> Result<ValidatedDatamodel, String> {
    parse_datamodel_internal(datamodel_string, false, false)
        .map_err(|err| err.to_pretty_string(file_name, datamodel_string))
}

fn parse_datamodel_internal(
    datamodel_string: &str,
    ignore_datasource_urls: bool,
    transform: bool,
) -> Result<ValidatedDatamodel, diagnostics::Diagnostics> {
    let mut diagnostics = diagnostics::Diagnostics::new();
    let ast = ast::parse_schema(datamodel_string)?;

    let sources = load_sources(&ast, ignore_datasource_urls, vec![])?;
    let generators = GeneratorLoader::load_generators_from_ast(&ast)?;
    let validator = ValidationPipeline::new(&sources.subject);

    diagnostics.append_warning_vec(sources.warnings);
    diagnostics.append_warning_vec(generators.warnings);

    match validator.validate(&ast, transform) {
        Ok(mut src) => {
            src.warnings.append(&mut diagnostics.warnings);
            Ok(src)
        }
        Err(mut err) => {
            diagnostics.append(&mut err);
            Err(diagnostics)
        }
    }
}

pub fn parse_schema_ast(datamodel_string: &str) -> Result<SchemaAst, diagnostics::Diagnostics> {
    ast::parse_schema(datamodel_string)
}

/// Loads all configuration blocks from a datamodel using the built-in source definitions.
pub fn parse_configuration(datamodel_string: &str) -> Result<ValidatedConfiguration, diagnostics::Diagnostics> {
    let mut warnings = Vec::new();
    let ast = ast::parse_schema(datamodel_string)?;
    let mut validated_sources = load_sources(&ast, false, vec![])?;
    let mut validated_generators = GeneratorLoader::load_generators_from_ast(&ast)?;

    warnings.append(&mut validated_generators.warnings);
    warnings.append(&mut validated_sources.warnings);

    Ok(ValidatedConfiguration {
        subject: Configuration {
            datasources: validated_sources.subject,
            generators: validated_generators.subject,
        },
        warnings,
    })
}

/// - `datasource_url_overrides`: the tuples consist of datasource name and url
pub fn parse_configuration_with_url_overrides(
    schema: &str,
    datasource_url_overrides: Vec<(String, String)>,
) -> Result<ValidatedConfiguration, diagnostics::Diagnostics> {
    let mut warnings = Vec::new();
    let ast = ast::parse_schema(schema)?;
    let mut validated_sources = load_sources(&ast, false, datasource_url_overrides)?;
    let mut validated_generators = GeneratorLoader::load_generators_from_ast(&ast)?;

    warnings.append(&mut validated_generators.warnings);
    warnings.append(&mut validated_sources.warnings);

    Ok(ValidatedConfiguration {
        subject: Configuration {
            datasources: validated_sources.subject,
            generators: validated_generators.subject,
        },
        warnings,
    })
}

pub fn parse_configuration_with_config_dir(
    schema: &str,
    datasource_url_overrides: Vec<(String, String)>,
    config_dir: &Path,
) -> Result<ValidatedConfiguration, diagnostics::Diagnostics> {
    let mut warnings = Vec::new();
    let ast = ast::parse_schema(schema)?;
    let mut validated_sources = load_sources(&ast, false, datasource_url_overrides)?;
    let mut validated_generators = GeneratorLoader::load_generators_from_ast(&ast)?;

    warnings.append(&mut validated_generators.warnings);
    warnings.append(&mut validated_sources.warnings);

    for source in validated_sources.subject.iter_mut() {
        source.set_config_dir(config_dir);
    }

    Ok(ValidatedConfiguration {
        subject: Configuration {
            datasources: validated_sources.subject,
            generators: validated_generators.subject,
        },
        warnings,
    })
}

pub fn parse_configuration_and_ignore_datasource_urls(
    datamodel_string: &str,
) -> Result<ValidatedConfiguration, diagnostics::Diagnostics> {
    let mut warnings = Vec::new();
    let ast = ast::parse_schema(datamodel_string)?;
    let mut validated_sources = load_sources(&ast, true, vec![])?;
    let mut validated_generators = GeneratorLoader::load_generators_from_ast(&ast)?;

    warnings.append(&mut validated_generators.warnings);
    warnings.append(&mut validated_sources.warnings);

    Ok(ValidatedConfiguration {
        subject: Configuration {
            datasources: validated_sources.subject,
            generators: validated_generators.subject,
        },
        warnings,
    })
}

fn load_sources(
    schema_ast: &SchemaAst,
    ignore_datasource_urls: bool,
    datasource_url_overrides: Vec<(String, String)>,
) -> Result<ValidatedDatasources, diagnostics::Diagnostics> {
    let source_loader = DatasourceLoader::new();
    source_loader.load_datasources_from_ast(&schema_ast, ignore_datasource_urls, datasource_url_overrides)
}

//
//  ************** RENDERING FUNCTIONS **************
//

/// Renders to a return string.
pub fn render_datamodel_to_string(datamodel: &dml::Datamodel) -> String {
    let mut writable_string = common::WritableString::new();
    render_datamodel_to(&mut writable_string, datamodel, None);
    writable_string.into()
}

/// Renders an AST to a string.
pub fn render_schema_ast_to_string(schema: &SchemaAst) -> String {
    let mut writable_string = common::WritableString::new();
    render_schema_ast_to(&mut writable_string, &schema, 2);
    writable_string.into()
}

/// Renders as a string into the stream.
pub fn render_datamodel_to(
    stream: &mut dyn std::io::Write,
    datamodel: &dml::Datamodel,
    datasource: Option<&Datasource>,
) {
    let lowered = LowerDmlToAst::new(datasource).lower(datamodel);
    render_schema_ast_to(stream, &lowered, 2);
}

/// Renders a datamodel, sources and generators to a string.
pub fn render_datamodel_and_config_to_string(
    datamodel: &dml::Datamodel,
    config: &configuration::Configuration,
) -> String {
    let mut writable_string = common::WritableString::new();
    render_datamodel_and_config_to(&mut writable_string, datamodel, config);
    writable_string.into()
}

/// Renders a datamodel, generators and sources to a stream as a string.
fn render_datamodel_and_config_to(
    stream: &mut dyn std::io::Write,
    datamodel: &dml::Datamodel,
    config: &configuration::Configuration,
) {
    let mut lowered = LowerDmlToAst::new(config.datasources.first()).lower(datamodel);

    DatasourceSerializer::add_sources_to_ast(config.datasources.as_slice(), &mut lowered);
    GeneratorSerializer::add_generators_to_ast(&config.generators, &mut lowered);

    render_schema_ast_to(stream, &lowered, 2);
}

/// Renders as a string into the stream.
fn render_schema_ast_to(stream: &mut dyn std::io::Write, schema: &ast::SchemaAst, ident_width: usize) {
    let mut renderer = ast::Renderer::new(stream, ident_width);
    renderer.render(schema);
}
