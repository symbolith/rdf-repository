use rdf_repository_shacl::Document;

const INPUT: &str = "\
---
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@base <http://example.org/alice> .

<> a foaf:Person ;
   foaf:name \"Alice\" ;
   foaf:age 30 .
---
# About Alice

Alice is a person described in RDF-MD.
";

fn main() {
    let doc = Document::from_str(INPUT, None).unwrap();

    println!("Base subject: {}", doc.base_subject);
    println!();

    println!("Triples ({}):", doc.triples.len());
    for st in &doc.triples {
        println!(
            "  {} {} {}",
            st.triple.subject, st.triple.predicate, st.triple.object
        );
    }
    println!();

    println!("Body:\n{}", doc.body);
}
