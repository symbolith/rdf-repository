use oxrdf::{NamedNode, vocab};
use rdf_repository_shacl::Document;
use rdf_repository_shacl::vocabulary::{md, ptr};

const INPUT: &str = "\
---
@prefix schema: <http://schema.org/> .
@prefix ex: <http://example.org/> .
@base <http://example.org/article> .

<> a schema:Article ;
   schema:name \"Rust and RDF\"@en ;
   schema:author [ schema:name \"Bob\" ; schema:email \"bob@example.org\" ] ;
   schema:wordCount \"1500\"^^<http://www.w3.org/2001/XMLSchema#integer> ;
   schema:keywords (\"rust\" \"rdf\" \"semantic-web\") .
---
# Rust and RDF

An article about combining Rust with RDF technologies.
";

fn main() {
    let doc = Document::from_str(INPUT, None).unwrap();

    println!("=== Graph queries ===");
    println!("Base subject: {}", doc.base_subject);
    println!("Graph size: {} triples", doc.graph.len());
    println!();

    println!("Triples for base subject:");
    for t in doc.graph.triples_for_subject(doc.base_subject.as_ref()) {
        println!("  {} {}", t.predicate, t.object);
    }
    println!();

    let name_pred = NamedNode::new("http://schema.org/name").unwrap();
    println!("Objects for schema:name:");
    for obj in doc
        .graph
        .objects_for_subject_predicate(doc.base_subject.as_ref(), name_pred.as_ref())
    {
        println!("  {obj}");
    }
    println!();

    println!("=== Source spans ===");
    for st in &doc.triples {
        if st.span.end_byte > 0 {
            println!(
                "  {}:{}-{}:{} | {} {} {}",
                st.span.start_line,
                st.span.start_column,
                st.span.end_line,
                st.span.end_column,
                st.triple.subject,
                st.triple.predicate,
                st.triple.object,
            );
        }
    }
    println!();

    println!("=== Provenance ===");
    let prov = rdf_repository_shacl::provenance::annotate(&doc.triples, &doc.base_subject);
    println!("Provenance graph: {} triples", prov.len());

    if let Some(reif) = prov.iter().find(|t| t.predicate == vocab::rdf::REIFIES) {
        println!(
            "Sample reification: {} reifies {}",
            reif.subject, reif.object
        );
    }

    let has_ptr = prov
        .iter()
        .any(|t| t.predicate == vocab::rdf::TYPE && t.object == ptr::START_END_POINTER.into());
    println!("Contains ptr:StartEndPointer types: {has_ptr}");
    println!();

    println!("=== Validation ===");
    let errors = rdf_repository_shacl::validation::validate(&doc.graph);
    println!("Validation errors: {}", errors.len());
    println!();

    println!("=== Vocabulary constants ===");
    println!("md namespace: {}", md::NAMESPACE);
    println!("md:Document:  {}", md::DOCUMENT);
    println!("md:body:      {}", md::BODY);
    println!("ptr namespace: {}", ptr::NAMESPACE);
    println!("ptr:StartEndPointer: {}", ptr::START_END_POINTER);
    println!("ptr:lineNumber:      {}", ptr::LINE_NUMBER);
}
