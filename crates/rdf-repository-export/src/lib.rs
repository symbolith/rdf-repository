pub mod store;
pub mod tree;

use std::io;
use std::path::PathBuf;

use oxrdf::IriParseError;
use oxrdfio::RdfSyntaxError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("cannot read {}", path.display()))]
    Read { path: PathBuf, source: io::Error },
    #[snafu(display("{} is not UTF-8", path.display()))]
    Utf8Path { path: PathBuf },
    #[snafu(display("invalid IRI"))]
    Iri { source: IriParseError },
    #[snafu(display("cannot resolve {reference}"))]
    Resolve {
        reference: String,
        source: rdf_trilith_iri::ValidationError,
    },
    #[snafu(display("invalid RDF term"))]
    Term { source: rdf_trilith_term::ValidationError },
    #[snafu(display("unsupported RDF construct"))]
    Unsupported { source: rdf_trilith_term::ConversionError },
    #[snafu(display("frontmatter has no closing fence"))]
    UnclosedFrontmatter,
    #[snafu(display("invalid YAML frontmatter"))]
    Yaml { source: serde_yaml::Error },
    #[snafu(display("frontmatter is not JSON compatible"))]
    NotJsonCompatible,
    #[snafu(display("context is not a JSON object"))]
    InvalidContext,
    #[snafu(display("frontmatter key {key} is not defined in the context"))]
    UnmappedKey { key: String },
    #[snafu(display("cannot serialize JSON-LD document"))]
    JsonLdDocument { source: serde_json::Error },
    #[snafu(display("invalid RDF"))]
    Rdf { source: RdfSyntaxError },
    #[snafu(display("incompatible turtle grammar"))]
    Grammar { source: tree_sitter::LanguageError },
    #[snafu(display("cannot parse turtle"))]
    TurtleParse,
    #[snafu(display("turtle syntax error at byte {offset}"))]
    TurtleSyntax { offset: usize },
}
