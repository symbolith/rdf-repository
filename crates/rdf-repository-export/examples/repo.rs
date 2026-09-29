use std::error::Error;
use std::{env, fs};

use rdf_trilith_iri::{IriAbsoluteBuf, IriBuf};
use oxrdfio::{RdfFormat, RdfSerializer};
use rdf_repository_export::store::repo::{Repo, RepoConfig};
use rdf_repository_export::store::yaml::Schema;
use rdf_repository_export::tree::{Graph, RdfNode};
use serde_json::json;

const PLAN: &str = r#"---
aliases:
  - AP1
startedAtTime: 2025-02-01
wasAssociatedWith:
  - "[InfAI](agent.md)"
---

Task with [AP2](ap2.md).
"#;

const MINIMAL: &str = "References [another note](ap2.md).\n";

const GRAPH: &str = "<> <http://purl.org/dc/terms/references> <plan> .\n";

const EXPECTED: &str = r#"<https://rdf-r.org/graph> <http://purl.org/dc/terms/references> <https://rdf-r.org/plan> .
<https://rdf-r.org/minimal> <http://purl.org/dc/terms/references> <https://rdf-r.org/ap2> ;
	<http://schema.org/text> "References [another note](ap2.md).\n" .
<https://rdf-r.org/plan> <http://www.w3.org/2000/01/rdf-schema#label> "AP1" ;
	<http://www.w3.org/ns/prov#startedAtTime> "2025-02-01"^^<http://www.w3.org/2001/XMLSchema#date> ;
	<http://www.w3.org/ns/prov#wasAssociatedWith> <https://rdf-r.org/agent> ;
	<http://purl.org/dc/terms/references> <https://rdf-r.org/ap2> ;
	<http://schema.org/text> "\n\nTask with [AP2](ap2.md).\n" .
"#;

fn main() -> Result<(), Box<dyn Error>> {
    let root = env::temp_dir().join("rdf-repository-example-repo");
    fs::create_dir_all(&root)?;
    fs::write(root.join("plan.md"), PLAN)?;
    fs::write(root.join("minimal.md"), MINIMAL)?;
    fs::write(root.join("graph.ttl"), GRAPH)?;
    let base = IriAbsoluteBuf::new("https://rdf-r.org").unwrap();
    let link_predicate = IriBuf::new("http://purl.org/dc/terms/references").unwrap();
    let body_predicate = IriBuf::new("http://schema.org/text").unwrap();
    let schema = Schema {
        context: json!({
            "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
            "prov": "http://www.w3.org/ns/prov#",
            "xsd": "http://www.w3.org/2001/XMLSchema#",
            "aliases": "rdfs:label",
            "startedAtTime": {"@id": "prov:startedAtTime", "@type": "xsd:date"},
            "wasAssociatedWith": {"@id": "prov:wasAssociatedWith", "@type": "@id"},
        }),
    };
    let repo = Repo::construct(RepoConfig {
        root: &root,
        base: &base,
        link_predicate: &link_predicate,
        body_predicate: &body_predicate,
        schema: &schema,
    })?;
    let mut serializer = RdfSerializer::from_format(RdfFormat::Turtle).for_writer(Vec::new());
    for triple in repo.triples() {
        serializer.serialize_triple(&oxrdf::Triple::from(triple))?;
    }
    let output = String::from_utf8(serializer.finish()?)?;
    print!("{output}");
    assert_eq!(output, EXPECTED);
    Ok(())
}
