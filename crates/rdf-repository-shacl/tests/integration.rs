use oxrdf::{NamedNode, vocab};
use rdf_repository_shacl::Document;
use rdf_repository_shacl::parser;
use rdf_repository_shacl::vocabulary::md;

const SAMPLE_DOC: &str = "\
---
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@base <http://example.org/alice> .

<> a foaf:Person ;
   foaf:name \"Alice\" .
---
# Hello

This is the body.
";

#[test]
fn split_document_extracts_frontmatter_and_body() {
    let (fm, body) = parser::split_document(SAMPLE_DOC).unwrap();
    assert!(fm.content.contains("@prefix"));
    assert!(fm.content.contains("foaf:Person"));
    assert!(body.content.contains("# Hello"));
    assert!(body.content.contains("This is the body."));
}

#[test]
fn parse_turtle_extracts_triples() {
    let (fm, _) = parser::split_document(SAMPLE_DOC).unwrap();
    let result = parser::parse_turtle(&fm.content, None).unwrap();

    assert_eq!(result.triples.len(), 2);

    let type_triple = result
        .triples
        .iter()
        .find(|st| st.triple.predicate == vocab::rdf::TYPE);
    assert!(type_triple.is_some());

    let name_triple = result
        .triples
        .iter()
        .find(|st| st.triple.predicate.as_str() == "http://xmlns.com/foaf/0.1/name");
    assert!(name_triple.is_some());
}

#[test]
fn parse_turtle_resolves_base_iri() {
    let (fm, _) = parser::split_document(SAMPLE_DOC).unwrap();
    let result = parser::parse_turtle(&fm.content, None).unwrap();

    assert_eq!(
        result.base_iri.as_ref().map(|n| n.as_str()),
        Some("http://example.org/alice")
    );

    let subj = &result.triples[0].triple.subject;
    match subj {
        oxrdf::NamedOrBlankNode::NamedNode(n) => {
            assert_eq!(n.as_str(), "http://example.org/alice");
        }
        _ => panic!("expected named node subject"),
    }
}

#[test]
fn source_spans_are_nonzero() {
    let (fm, _) = parser::split_document(SAMPLE_DOC).unwrap();
    let result = parser::parse_turtle(&fm.content, None).unwrap();

    for st in &result.triples {
        assert!(
            st.span.end_byte > st.span.start_byte,
            "span should have nonzero length"
        );
    }
}

#[test]
fn document_from_str_sets_base_subject() {
    let doc = Document::from_str(SAMPLE_DOC, None).unwrap();
    assert_eq!(doc.base_subject.as_str(), "http://example.org/alice");
}

#[test]
fn document_from_str_default_type_present() {
    let no_type_doc = "\
---
@base <http://example.org/bob> .
---
Body text.
";
    let doc = Document::from_str(no_type_doc, None).unwrap();

    let has_type = doc
        .graph
        .triples_for_subject(doc.base_subject.as_ref())
        .any(|t| t.predicate == vocab::rdf::TYPE && t.object == md::DOCUMENT.into());
    assert!(has_type, "default rdfmd:Document type should be added");
}

#[test]
fn document_from_str_explicit_type_not_duplicated() {
    let doc = Document::from_str(SAMPLE_DOC, None).unwrap();

    let type_count = doc
        .graph
        .triples_for_subject(doc.base_subject.as_ref())
        .filter(|t| t.predicate == vocab::rdf::TYPE)
        .count();
    assert_eq!(
        type_count, 1,
        "explicit type should not be duplicated with default"
    );
}

#[test]
fn document_from_str_body_triple_present() {
    let doc = Document::from_str(SAMPLE_DOC, None).unwrap();

    let body_triple = doc
        .graph
        .triples_for_subject(doc.base_subject.as_ref())
        .find(|t| t.predicate == md::BODY);
    assert!(body_triple.is_some(), "body triple should be present");

    let body_obj = body_triple.unwrap().object;
    match body_obj {
        oxrdf::TermRef::Literal(l) => {
            assert!(l.value().contains("# Hello"));
            assert!(l.value().contains("This is the body."));
        }
        _ => panic!("body should be a literal"),
    }
}

#[test]
fn document_fallback_base_iri() {
    let doc_str = "\
---
@prefix ex: <http://example.org/> .
ex:thing ex:name \"Thing\" .
---
Content.
";
    let doc = Document::from_str(doc_str, Some("http://example.org/fallback")).unwrap();
    assert_eq!(doc.base_subject.as_str(), "http://example.org/fallback");
}

#[test]
fn provenance_generates_annotations() {
    let (fm, _) = parser::split_document(SAMPLE_DOC).unwrap();
    let result = parser::parse_turtle(&fm.content, None).unwrap();

    let source_iri = result.base_iri.clone().unwrap();
    let prov_graph = rdf_repository_shacl::provenance::annotate(&result.triples, &source_iri);

    assert!(!prov_graph.is_empty());

    let has_reifies = prov_graph
        .iter()
        .any(|t| t.predicate == vocab::rdf::REIFIES);
    assert!(has_reifies, "provenance should contain rdf:reifies triples");

    let ptr_type = NamedNode::new("http://www.w3.org/2009/pointers#StartEndPointer").unwrap();
    let has_ptr = prov_graph
        .iter()
        .any(|t| t.predicate == vocab::rdf::TYPE && t.object == ptr_type.as_ref().into());
    assert!(
        has_ptr,
        "provenance should contain ptr:StartEndPointer types"
    );
}

#[test]
fn validation_stub_returns_empty() {
    let doc = Document::from_str(SAMPLE_DOC, None).unwrap();
    let errors = rdf_repository_shacl::validation::validate(&doc.graph);
    assert!(errors.is_empty());
}

const TYPED_LITERALS_DOC: &str = "\
---
@prefix schema: <http://schema.org/> .
@base <http://example.org/> .
<item> a schema:Thing ;
    schema:name \"Test Item\"@en ;
    schema:count 42 ;
    schema:active true .
---
Body.
";

fn typed_literals_document() -> Document {
    Document::from_str(TYPED_LITERALS_DOC, None).unwrap()
}

fn literal_for_predicate(document: &Document, predicate_iri: &str) -> oxrdf::Literal {
    let triple = document
        .graph
        .iter()
        .find(|triple| triple.predicate.as_str() == predicate_iri)
        .expect("triple for predicate should exist");
    let oxrdf::TermRef::Literal(literal) = triple.object else {
        panic!("expected literal object for {predicate_iri}");
    };
    literal.into_owned()
}

#[test]
fn parse_prefixed_names_resolves_base_subject() {
    let document = typed_literals_document();
    assert_eq!(document.base_subject.as_str(), "http://example.org/");
}

#[test]
fn parse_language_tagged_literal() {
    let literal = literal_for_predicate(&typed_literals_document(), "http://schema.org/name");
    assert_eq!(literal.value(), "Test Item");
    assert_eq!(literal.language(), Some("en"));
}

#[test]
fn parse_integer_literal() {
    let literal = literal_for_predicate(&typed_literals_document(), "http://schema.org/count");
    assert_eq!(literal.value(), "42");
    assert_eq!(literal.datatype(), vocab::xsd::INTEGER);
}

#[test]
fn parse_boolean_literal() {
    let literal = literal_for_predicate(&typed_literals_document(), "http://schema.org/active");
    assert_eq!(literal.value(), "true");
    assert_eq!(literal.datatype(), vocab::xsd::BOOLEAN);
}

#[test]
fn parse_blank_node_property_list() {
    let input = "\
---
@prefix ex: <http://example.org/> .
@base <http://example.org/doc> .
<> ex:author [ ex:name \"Bob\" ] .
---
Body.
";
    let doc = Document::from_str(input, None).unwrap();

    let name_triple = doc
        .graph
        .iter()
        .find(|t| t.predicate.as_str() == "http://example.org/name");
    assert!(name_triple.is_some());
    match name_triple.unwrap().object {
        oxrdf::TermRef::Literal(l) => assert_eq!(l.value(), "Bob"),
        _ => panic!("expected literal"),
    }
}

#[test]
fn parse_collection() {
    let input = "\
---
@prefix ex: <http://example.org/> .
@base <http://example.org/doc> .
<> ex:items (ex:a ex:b ex:c) .
---
Body.
";
    let doc = Document::from_str(input, None).unwrap();

    let first_count = doc
        .graph
        .iter()
        .filter(|t| t.predicate == vocab::rdf::FIRST)
        .count();
    let rest_count = doc
        .graph
        .iter()
        .filter(|t| t.predicate == vocab::rdf::REST)
        .count();
    assert_eq!(first_count, 3);
    assert_eq!(rest_count, 3);
}

#[test]
fn missing_frontmatter_returns_error() {
    let input = "No frontmatter here.";
    assert!(Document::from_str(input, None).is_err());
}
