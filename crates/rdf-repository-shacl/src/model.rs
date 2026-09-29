use std::path::Path;

use oxrdf::{Graph, Literal, NamedNode, Triple, vocab};

use crate::parser::{self, ParseError, SourceSpan, SpannedTriple};
use crate::vocabulary::md;

pub enum DocumentSource<'a> {
    File(&'a Path),
    String { base_iri: Option<&'a str> },
}

pub struct Document {
    pub base_subject: NamedNode,
    pub body: String,
    pub triples: Vec<SpannedTriple>,
    pub graph: Graph,
}

impl Document {
    pub fn from_str(input: &str, base_iri: Option<&str>) -> Result<Self, ParseError> {
        Self::build(input, DocumentSource::String { base_iri })
    }

    pub fn from_file(path: &Path) -> Result<Self, ParseError> {
        let input =
            std::fs::read_to_string(path).map_err(|e| ParseError::InvalidIri(e.to_string()))?;
        Self::build(&input, DocumentSource::File(path))
    }

    fn build(input: &str, source: DocumentSource) -> Result<Self, ParseError> {
        let (frontmatter, body) = parser::split_document(input)?;

        let default_base = match &source {
            DocumentSource::File(path) => {
                let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
                Some(format!("file://{}", abs.display()))
            }
            DocumentSource::String { base_iri } => base_iri.map(|s| s.to_string()),
        };

        let result = parser::parse_turtle(&frontmatter.content, default_base.as_deref())?;

        let base_subject = result
            .base_iri
            .or_else(|| default_base.and_then(|s| NamedNode::new(s).ok()))
            .unwrap_or_else(|| NamedNode::new_unchecked("urn:rdf-md:anonymous"));

        let mut triples = result.triples;

        let has_type = triples.iter().any(|st| {
            st.triple.predicate == vocab::rdf::TYPE
                && matches!(&st.triple.subject, oxrdf::NamedOrBlankNode::NamedNode(n) if *n == base_subject)
        });

        if !has_type {
            triples.push(SpannedTriple {
                triple: Triple::new(
                    base_subject.clone(),
                    vocab::rdf::TYPE.into_owned(),
                    md::DOCUMENT.into_owned(),
                ),
                span: SourceSpan {
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 0,
                    start_column: 0,
                    end_line: 0,
                    end_column: 0,
                },
            });
        }

        let body_span = SourceSpan {
            start_byte: 0,
            end_byte: 0,
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        };
        triples.push(SpannedTriple {
            triple: Triple::new(
                base_subject.clone(),
                md::BODY.into_owned(),
                Literal::new_typed_literal(&body.content, vocab::xsd::STRING),
            ),
            span: body_span,
        });

        let mut graph = Graph::new();
        for st in &triples {
            graph.insert(st.triple.as_ref());
        }

        Ok(Document {
            base_subject,
            body: body.content,
            triples,
            graph,
        })
    }
}
