use oxrdf::{BlankNode, Graph, Literal, NamedNode, Triple, vocab};

use crate::parser::SpannedTriple;
use crate::vocabulary::ptr;

pub fn annotate(triples: &[SpannedTriple], source_iri: &NamedNode) -> Graph {
    let mut graph = Graph::new();
    let _ = source_iri;

    for st in triples {
        let reif = BlankNode::default();
        let ptr_node = BlankNode::default();
        let start_ptr = BlankNode::default();
        let end_ptr = BlankNode::default();

        let triple_term = oxrdf::Term::Triple(Box::new(st.triple.clone()));
        graph.insert(
            Triple::new(reif.clone(), vocab::rdf::REIFIES.into_owned(), triple_term).as_ref(),
        );

        graph
            .insert(Triple::new(reif, ptr::SOURCE_POINTER.into_owned(), ptr_node.clone()).as_ref());

        graph.insert(
            Triple::new(
                ptr_node.clone(),
                vocab::rdf::TYPE.into_owned(),
                ptr::START_END_POINTER.into_owned(),
            )
            .as_ref(),
        );

        graph.insert(
            Triple::new(
                ptr_node.clone(),
                ptr::START_POINTER.into_owned(),
                start_ptr.clone(),
            )
            .as_ref(),
        );

        graph
            .insert(Triple::new(ptr_node, ptr::END_POINTER.into_owned(), end_ptr.clone()).as_ref());

        graph.insert(
            Triple::new(
                start_ptr.clone(),
                vocab::rdf::TYPE.into_owned(),
                ptr::LINE_CHAR_POINTER.into_owned(),
            )
            .as_ref(),
        );
        graph.insert(
            Triple::new(
                start_ptr.clone(),
                ptr::LINE_NUMBER.into_owned(),
                Literal::new_typed_literal(st.span.start_line.to_string(), vocab::xsd::INTEGER),
            )
            .as_ref(),
        );
        graph.insert(
            Triple::new(
                start_ptr,
                ptr::CHAR_NUMBER.into_owned(),
                Literal::new_typed_literal(st.span.start_column.to_string(), vocab::xsd::INTEGER),
            )
            .as_ref(),
        );

        graph.insert(
            Triple::new(
                end_ptr.clone(),
                vocab::rdf::TYPE.into_owned(),
                ptr::LINE_CHAR_POINTER.into_owned(),
            )
            .as_ref(),
        );
        graph.insert(
            Triple::new(
                end_ptr.clone(),
                ptr::LINE_NUMBER.into_owned(),
                Literal::new_typed_literal(st.span.end_line.to_string(), vocab::xsd::INTEGER),
            )
            .as_ref(),
        );
        graph.insert(
            Triple::new(
                end_ptr,
                ptr::CHAR_NUMBER.into_owned(),
                Literal::new_typed_literal(st.span.end_column.to_string(), vocab::xsd::INTEGER),
            )
            .as_ref(),
        );
    }

    graph
}
