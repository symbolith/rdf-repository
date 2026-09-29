use std::error::Error;
use std::fmt::Write;

use rdf_trilith_iri::IriAbsoluteBuf;
use rdf_repository_export::store::turtle::{TurtleBlock, TurtleConfig};
use rdf_repository_export::tree::{Graph, RdfNode};

const EXPECTED: &str = r#"0..44 "@prefix prov: <http://www.w3.org/ns/prov#> ."
46..103 "<> a prov:Activity ;\n    prov:wasAssociatedWith <alice> ."
    <https://rdf-r.org/ex-note> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/prov#Activity>
    <https://rdf-r.org/ex-note> <http://www.w3.org/ns/prov#wasAssociatedWith> <https://rdf-r.org/alice>
105..127 "<alice> a prov:Agent ."
    <https://rdf-r.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/prov#Agent>
"#;

fn main() -> Result<(), Box<dyn Error>> {
    let text = "\
@prefix prov: <http://www.w3.org/ns/prov#> .

<> a prov:Activity ;
    prov:wasAssociatedWith <alice> .

<alice> a prov:Agent .
";
    let base = IriAbsoluteBuf::new("https://rdf-r.org/ex-note").unwrap();
    let block = TurtleBlock::construct(TurtleConfig { text, base: &base })?;
    let mut output = String::new();
    for statement in block.statements() {
        writeln!(
            output,
            "{:?} {:?}",
            statement.span,
            &text[statement.span.clone()]
        )?;
        for triple in statement.triples() {
            writeln!(output, "    {}", oxrdf::Triple::from(triple))?;
        }
    }
    assert_eq!(output, EXPECTED);
    print!("{output}");
    Ok(())
}
