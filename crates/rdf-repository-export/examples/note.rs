use std::error::Error;
use std::fmt::Write;
use std::{env, fs};

use rdf_trilith_iri::{IriAbsoluteBuf, IriBuf};
use rdf_repository_export::store::markdown::{MarkdownConfig, MarkdownFile};
use rdf_repository_export::store::yaml::Schema;
use rdf_repository_export::tree::{Graph, RdfNode};
use serde_json::json;

const INPUT: &str = r#"---
aliases:
  - AP1
type:
  - "[Plan](20251204173152.md)"
summary: Build the knowledge graph exporter
startedAtTime: 2025-02-01
personMonth: 3
wasAssociatedWith:
  - "[InfAI](60178bab-87e1-4425-943d-873d35f6c928.md)"
---

## Tasks

- [ ] Draft the ontology together with [PMD](bf9186dc-894b-4a7a-8efb-701657478b02.md)

## Description

The exporter converts every note into triples, based on the knowledge model
([AP2](3c46d666-b5a6-499d-a4ab-ef659982b882.md)).
"#;

const EXPECTED: &str = r#"<https://rdf-r.org/ex-note> <http://www.w3.org/2000/01/rdf-schema#label> "AP1" .
<https://rdf-r.org/ex-note> <http://dziw.is/wiki#personMonth> "3"^^<http://www.w3.org/2001/XMLSchema#integer> .
<https://rdf-r.org/ex-note> <http://www.w3.org/ns/prov#startedAtTime> "2025-02-01"^^<http://www.w3.org/2001/XMLSchema#date> .
<https://rdf-r.org/ex-note> <http://purl.org/dc/terms/abstract> "Build the knowledge graph exporter" .
<https://rdf-r.org/ex-note> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://rdf-r.org/20251204173152> .
<https://rdf-r.org/ex-note> <http://www.w3.org/ns/prov#wasAssociatedWith> <https://rdf-r.org/60178bab-87e1-4425-943d-873d35f6c928> .
<https://rdf-r.org/ex-note> <http://purl.org/dc/terms/references> <https://rdf-r.org/bf9186dc-894b-4a7a-8efb-701657478b02> .
<https://rdf-r.org/ex-note> <http://purl.org/dc/terms/references> <https://rdf-r.org/3c46d666-b5a6-499d-a4ab-ef659982b882> .
<https://rdf-r.org/ex-note> <http://schema.org/text> "\n\n## Tasks\n\n- [ ] Draft the ontology together with [PMD](bf9186dc-894b-4a7a-8efb-701657478b02.md)\n\n## Description\n\nThe exporter converts every note into triples, based on the knowledge model\n([AP2](3c46d666-b5a6-499d-a4ab-ef659982b882.md)).\n" .
"#;

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::temp_dir().join("plan.md");
    fs::write(&path, INPUT)?;
    let base = IriAbsoluteBuf::new("https://rdf-r.org/ex-note").unwrap();
    let link_predicate = IriBuf::new("http://purl.org/dc/terms/references").unwrap();
    let body_predicate = IriBuf::new("http://schema.org/text").unwrap();
    let schema = Schema {
        context: json!({
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
        }),
    };
    let file = MarkdownFile::construct(MarkdownConfig {
        path: &path,
        base: &base,
        link_predicate: &link_predicate,
        body_predicate: &body_predicate,
        schema: &schema,
    })?;
    let mut output = String::new();
    for statement in file.triples() {
        writeln!(output, "{} .", oxrdf::Triple::from(statement))?;
    }
    print!("{output}");
    assert_eq!(output, EXPECTED);
    Ok(())
}
