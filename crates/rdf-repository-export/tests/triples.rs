use std::collections::HashSet;
use std::path::Path;

use rdf_trilith_graph::{GraphMut, MemoryGraph, Triple, TripleBuf};
use rdf_trilith_iri::{IriAbsoluteBuf, IriBuf};
use rdf_repository_export::store::markdown::{MarkdownConfig, MarkdownFile};
use rdf_repository_export::store::repo::{Repo, RepoConfig};
use rdf_repository_export::store::turtle::{TurtleBlock, TurtleConfig};
use rdf_repository_export::store::yaml::Schema;
use rdf_repository_export::tree::{Graph, RdfNode};
use serde_json::json;
use rdf_trilith_term::{LexicalFormBuf, Literal};

const BASE: &str = "https://wiki.example.org/note";

fn iri(text: &str) -> IriBuf {
    IriBuf::new(text).unwrap()
}

fn references() -> IriBuf {
    iri("http://purl.org/dc/terms/references")
}

fn rdf_type() -> IriBuf {
    iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
}

fn body_predicate() -> IriBuf {
    iri("http://schema.org/text")
}

fn simple(text: &str) -> Literal {
    Literal::Simple {
        lexical_form: LexicalFormBuf::new(text).unwrap(),
    }
}

fn typed(text: &str, datatype: &str) -> Literal {
    Literal::Typed {
        lexical_form: LexicalFormBuf::new(text).unwrap(),
        datatype_iri: iri(datatype),
    }
}

fn body_count(graph: &MemoryGraph) -> usize {
    graph
        .triples()
        .filter(|triple| *triple.predicate == body_predicate())
        .count()
}

fn schema() -> Schema {
    let context = json!({
        "rdf": "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
        "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
        "prov": "http://www.w3.org/ns/prov#",
        "wiki": "http://dziw.is/wiki#",
        "xsd": "http://www.w3.org/2001/XMLSchema#",
        "aliases": "rdfs:label",
        "type": {"@id": "rdf:type", "@type": "@id"},
        "dcterms": "http://purl.org/dc/terms/",
        "summary": "dcterms:abstract",
        "startedAtTime": {"@id": "prov:startedAtTime", "@type": "xsd:date"},
        "personMonth": "wiki:personMonth",
        "wasAssociatedWith": {"@id": "prov:wasAssociatedWith", "@type": "@id"},
    });
    Schema { context }
}

fn note(stem: &str) -> IriBuf {
    iri(&format!("{BASE}/{stem}"))
}

fn collect(store: &impl for<'g> Graph<Triple<'g> = Triple<'g>>) -> MemoryGraph {
    let mut graph = MemoryGraph::new();
    for triple in store.triples() {
        graph.insert(triple);
    }
    graph
}

fn export_from(directory: &str, fixture: &str) -> MemoryGraph {
    let stem = fixture.strip_suffix(".md").unwrap();
    let path = Path::new(directory).join(fixture);
    let base = IriAbsoluteBuf::new(&format!("{BASE}/{stem}")).unwrap();
    let schema = schema();
    let file = MarkdownFile::construct(MarkdownConfig {
        path: &path,
        base: &base,
        link_predicate: &references(),
        body_predicate: &body_predicate(),
        schema: &schema,
    })
    .unwrap();
    collect(&file)
}

fn export(fixture: &str) -> MemoryGraph {
    export_from("tests/fixtures", fixture)
}

fn edge(fixture: &str) -> MemoryGraph {
    export_from("tests/edge", fixture)
}

#[test]
fn note_without_frontmatter() {
    let graph = export("minimal.md");
    let expected = TripleBuf::new(
        note("minimal"),
        references(),
        note("0176aad4-8445-431d-93f0-eeff1a5b5f09"),
    );
    assert!(graph.contains(Triple::from(&expected)));
    assert_eq!(body_count(&graph), 1);
    assert_eq!(graph.len(), 2);
}

#[test]
fn note_with_yaml_frontmatter() {
    let graph = export("plan.md");
    let subject = note("plan");
    let expected = [
        TripleBuf::new(subject.clone(), rdf_type(), note("20251204173152")),
        TripleBuf::new(
            subject.clone(),
            iri("http://www.w3.org/2000/01/rdf-schema#label"),
            simple("AP1"),
        ),
        TripleBuf::new(
            subject.clone(),
            iri("http://www.w3.org/ns/prov#startedAtTime"),
            typed("2025-02-01", "http://www.w3.org/2001/XMLSchema#date"),
        ),
        TripleBuf::new(
            subject.clone(),
            iri("http://www.w3.org/ns/prov#wasAssociatedWith"),
            note("60178bab-87e1-4425-943d-873d35f6c928"),
        ),
        TripleBuf::new(
            subject.clone(),
            references(),
            note("bf9186dc-894b-4a7a-8efb-701657478b02"),
        ),
        TripleBuf::new(
            subject,
            references(),
            note("3c46d666-b5a6-499d-a4ab-ef659982b882"),
        ),
    ];
    for triple in expected {
        assert!(graph.contains(Triple::from(&triple)), "missing {triple:?}");
    }
}

#[test]
fn note_with_turtle_frontmatter() {
    let graph = export("turtle.md");
    let subject = note("turtle");
    let expected = [
        TripleBuf::new(
            subject.clone(),
            rdf_type(),
            iri("http://www.w3.org/ns/prov#Activity"),
        ),
        TripleBuf::new(
            subject.clone(),
            iri("http://www.w3.org/ns/prov#wasAssociatedWith"),
            note("60178bab-87e1-4425-943d-873d35f6c928"),
        ),
        TripleBuf::new(
            subject,
            references(),
            note("3c46d666-b5a6-499d-a4ab-ef659982b882"),
        ),
    ];
    for triple in expected {
        assert!(graph.contains(Triple::from(&triple)), "missing {triple:?}");
    }
}

#[test]
fn repo_produces_the_union() {
    let repo = Repo::construct(RepoConfig {
        root: Path::new("tests/fixtures"),
        base: &IriAbsoluteBuf::new(BASE).unwrap(),
        link_predicate: &references(),
        body_predicate: &body_predicate(),
        schema: &schema(),
    })
    .unwrap();
    let graph = collect(&repo);
    assert_eq!(graph.len(), 20);
    assert_eq!(body_count(&graph), 5);
    assert!(graph.contains(Triple::from(&TripleBuf::new(
        note("minimal"),
        references(),
        note("0176aad4-8445-431d-93f0-eeff1a5b5f09"),
    ))));
    assert!(graph.contains(Triple::from(&TripleBuf::new(
        note("turtle"),
        rdf_type(),
        iri("http://www.w3.org/ns/prov#Activity"),
    ))));
    assert!(graph.contains(Triple::from(&TripleBuf::new(
        note("sub/nested"),
        references(),
        note("sub/sibling"),
    ))));
    assert!(graph.contains(Triple::from(&TripleBuf::new(
        note("sub/My%20Note"),
        references(),
        note("sub/minimal"),
    ))));
    assert!(graph.contains(Triple::from(&TripleBuf::new(
        note("graph"),
        references(),
        note("minimal")
    ))));
}

fn turtle_block(text: &str) -> TurtleBlock {
    let base = IriAbsoluteBuf::new(&format!("{BASE}/doc")).unwrap();
    TurtleBlock::construct(TurtleConfig { text, base: &base }).unwrap()
}

#[test]
fn turtle_statements_carry_spans() {
    let text = "@prefix prov: <http://www.w3.org/ns/prov#> .\n\
                <> a prov:Activity .\n\
                <> prov:wasAssociatedWith <other> .\n";
    let block = turtle_block(text);
    let statements = block.statements();
    assert_eq!(statements.len(), 3);
    assert_eq!(statements[0].triples().count(), 0);
    assert_eq!(statements[1].triples().count(), 1);
    assert_eq!(statements[2].triples().count(), 1);
    assert!(text[statements[0].span.clone()].starts_with("@prefix"));
    assert!(text[statements[2].span.clone()].starts_with("<> prov:wasAssociatedWith"));
    assert_eq!(collect(&block).len(), 2);
}

#[test]
fn same_subject_in_two_statements() {
    let text = "<> <http://example.org/p> <http://example.org/a> .\n\
                <> <http://example.org/q> <http://example.org/b> .\n";
    let block = turtle_block(text);
    assert_eq!(block.statements().len(), 2);
    assert_eq!(collect(&block).len(), 2);
}

#[test]
fn shared_blank_node_label_across_statements() {
    let text = "_:shared <http://example.org/p> <http://example.org/a> .\n\
                _:shared <http://example.org/p> <http://example.org/b> .\n";
    let graph = collect(&turtle_block(text));
    assert_eq!(graph.len(), 2);
    let subjects: HashSet<_> = graph.triples().map(|triple| triple.subject).collect();
    assert_eq!(subjects.len(), 1);
}

#[test]
fn blank_node_property_list_statement() {
    let text = "<> <http://example.org/p> [ <http://example.org/q> <http://example.org/o> ] .\n";
    let block = turtle_block(text);
    assert_eq!(block.statements().len(), 1);
    assert_eq!(collect(&block).len(), 2);
}

#[test]
fn link_with_heading_anchor() {
    let graph = edge("anchor.md");
    assert!(graph.contains(Triple::from(&TripleBuf::new(
        note("anchor"),
        references(),
        note("other")
    ))));
    assert_eq!(graph.len(), 2);
}

#[test]
fn comment_only_frontmatter() {
    let graph = edge("comment.md");
    assert_eq!(body_count(&graph), 1);
    assert_eq!(graph.len(), 1);
}

#[test]
fn markdown_link_in_literal_survives() {
    let graph = edge("links.md");
    let expected = [
        TripleBuf::new(
            note("links"),
            iri("http://purl.org/dc/terms/abstract"),
            simple("[AP2](3c46d666.md)"),
        ),
        TripleBuf::new(
            note("links"),
            iri("http://www.w3.org/ns/prov#wasAssociatedWith"),
            note("60178bab"),
        ),
    ];
    for triple in expected {
        assert!(graph.contains(Triple::from(&triple)), "missing {triple:?}");
    }
}

#[test]
fn crlf_turtle_frontmatter() {
    let graph = edge("crlf.md");
    assert!(graph.contains(Triple::from(&TripleBuf::new(
        note("crlf"),
        iri("http://www.w3.org/ns/prov#wasAssociatedWith"),
        note("other"),
    ))));
}

#[test]
fn byte_order_mark_before_frontmatter() {
    let graph = edge("bom.md");
    assert!(graph.contains(Triple::from(&TripleBuf::new(
        note("bom"),
        iri("http://www.w3.org/2000/01/rdf-schema#label"),
        simple("x"),
    ))));
}
