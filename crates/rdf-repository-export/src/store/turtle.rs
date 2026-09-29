pub mod statement;

use rdf_trilith_graph::{Triple, TripleBuf};
use rdf_trilith_iri::IriAbsolute;
use oxrdfio::{RdfFormat, RdfParser};
use snafu::{OptionExt, ResultExt};

use crate::store::turtle::statement::{Statement, StatementConfig};
use crate::tree::{Graph, RdfNode};
use crate::{
    Error, GrammarSnafu, IriSnafu, RdfSnafu, TurtleParseSnafu, TurtleSyntaxSnafu, UnsupportedSnafu,
};

pub struct TurtleBlock {
    statements: Vec<Statement>,
}

pub struct TurtleConfig<'a> {
    pub text: &'a str,
    pub base: &'a IriAbsolute,
}

impl TurtleBlock {
    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }
}

impl Graph for TurtleBlock {
    type Triple<'g> = Triple<'g>;

    fn triples(&self) -> impl Iterator<Item = Triple<'_>> {
        self.statements
            .iter()
            .flat_map(|statement| statement.triples())
    }

    fn contains<'s>(&'s self, triple: Triple<'s>) -> bool {
        self.statements
            .iter()
            .any(|statement| statement.contains(triple))
    }

    fn len(&self) -> usize {
        self.statements.iter().map(Graph::len).sum()
    }
}

impl RdfNode for TurtleBlock {
    type Config<'a> = TurtleConfig<'a>;

    fn construct(config: Self::Config<'_>) -> Result<Self, Error> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_turtle::LANGUAGE.into())
            .context(GrammarSnafu)?;
        let tree = parser.parse(config.text, None).context(TurtleParseSnafu)?;
        if let Some(offset) = first_error_offset(tree.root_node()) {
            return TurtleSyntaxSnafu { offset }.fail();
        }
        let mut prologue = String::new();
        let mut statements = Vec::new();
        let mut cursor = tree.root_node().walk();
        for node in tree.root_node().named_children(&mut cursor) {
            if node.kind() != "statement" {
                continue;
            }
            let span = node.byte_range();
            let text = &config.text[span.clone()];
            let triples = if is_directive(&node) {
                prologue.push_str(text);
                prologue.push('\n');
                Vec::new()
            } else {
                parse_statement(&format!("{prologue}{text}"), config.base)?
            };
            statements.push(Statement::construct(StatementConfig { span, triples })?);
        }
        Ok(TurtleBlock { statements })
    }
}

fn is_directive(statement: &tree_sitter::Node) -> bool {
    let mut cursor = statement.walk();
    statement
        .named_children(&mut cursor)
        .any(|child| matches!(child.kind(), "base" | "prefix"))
}

fn parse_statement(text: &str, base: &IriAbsolute) -> Result<Vec<TripleBuf>, Error> {
    let base: &str = base;
    RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(base)
        .context(IriSnafu)?
        .for_slice(text)
        .map(|quad| {
            TripleBuf::try_from(oxrdf::Triple::from(quad.context(RdfSnafu)?))
                .context(UnsupportedSnafu)
        })
        .collect()
}

fn first_error_offset(node: tree_sitter::Node) -> Option<usize> {
    if !node.has_error() {
        return None;
    }
    if node.is_error() || node.is_missing() {
        return Some(node.start_byte());
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find_map(|child| first_error_offset(child))
}
