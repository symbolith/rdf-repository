use std::collections::HashMap;

use oxrdf::{BlankNode, Literal, NamedNode, NamedNodeRef, NamedOrBlankNode, Term, Triple, vocab};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl SourceSpan {
    fn from_node(node: &tree_sitter::Node) -> Self {
        let start = node.start_position();
        let end = node.end_position();
        Self {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_line: start.row,
            start_column: start.column,
            end_line: end.row,
            end_column: end.column,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpannedTriple {
    pub triple: Triple,
    pub span: SourceSpan,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("missing frontmatter delimiters")]
    MissingFrontmatter,
    #[error("tree-sitter parse error")]
    TreeSitter,
    #[error("syntax error in frontmatter at {line}:{column}: {message}")]
    Syntax {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("undefined prefix: {0}")]
    UndefinedPrefix(String),
    #[error("invalid IRI: {0}")]
    InvalidIri(String),
}

pub struct FrontmatterRegion {
    pub content: String,
    pub offset_lines: usize,
}

pub struct BodyRegion {
    pub content: String,
}

pub struct ParseResult {
    pub triples: Vec<SpannedTriple>,
    pub base_iri: Option<NamedNode>,
    pub prefixes: HashMap<String, String>,
}

pub fn split_document(input: &str) -> Result<(FrontmatterRegion, BodyRegion), ParseError> {
    let mut lines = input.lines().enumerate();

    let first = lines.next().ok_or(ParseError::MissingFrontmatter)?;
    if first.1.trim() != "---" {
        return Err(ParseError::MissingFrontmatter);
    }

    let mut frontmatter_lines = Vec::new();
    let mut found_end = false;
    let mut end_line_index = 0;

    for (index, line) in lines {
        if line.trim() == "---" {
            found_end = true;
            end_line_index = index;
            break;
        }
        frontmatter_lines.push(line);
    }

    if !found_end {
        return Err(ParseError::MissingFrontmatter);
    }

    let body_start = input
        .lines()
        .take(end_line_index + 1)
        .map(|line| line.len() + 1)
        .sum::<usize>();
    let body = if body_start <= input.len() {
        &input[body_start..]
    } else {
        ""
    };

    Ok((
        FrontmatterRegion {
            content: frontmatter_lines.join("\n"),
            offset_lines: 1,
        },
        BodyRegion {
            content: body.to_string(),
        },
    ))
}

pub fn parse_turtle(frontmatter: &str, base_iri: Option<&str>) -> Result<ParseResult, ParseError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_turtle::LANGUAGE.into())
        .map_err(|_| ParseError::TreeSitter)?;

    let tree = parser
        .parse(frontmatter, None)
        .ok_or(ParseError::TreeSitter)?;

    let root = tree.root_node();
    if root.has_error()
        && let Some(error_node) = find_error_node(root)
    {
        let text = error_node
            .utf8_text(frontmatter.as_bytes())
            .unwrap_or("???");
        return Err(syntax_error(&error_node, format!("unexpected '{text}'")));
    }

    let mut walker = TurtleWalker {
        source: frontmatter,
        prefixes: HashMap::new(),
        base_iri: base_iri.map(String::from),
        triples: Vec::new(),
        blank_counter: 0,
    };

    walker.walk_document(root)?;

    Ok(ParseResult {
        base_iri: walker
            .base_iri
            .as_deref()
            .and_then(|iri| NamedNode::new(iri).ok()),
        prefixes: walker.prefixes,
        triples: walker.triples,
    })
}

fn find_error_node(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).find_map(find_error_node)
}

fn syntax_error(node: &tree_sitter::Node, message: String) -> ParseError {
    let position = node.start_position();
    ParseError::Syntax {
        line: position.row,
        column: position.column,
        message,
    }
}

struct TurtleWalker<'a> {
    source: &'a str,
    prefixes: HashMap<String, String>,
    base_iri: Option<String>,
    triples: Vec<SpannedTriple>,
    blank_counter: u64,
}

impl TurtleWalker<'_> {
    fn next_blank(&mut self) -> BlankNode {
        self.blank_counter += 1;
        BlankNode::new_unchecked(format!("b{}", self.blank_counter))
    }

    fn node_text(&self, node: &tree_sitter::Node) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    fn walk_document(&mut self, root: tree_sitter::Node) -> Result<(), ParseError> {
        let mut cursor = root.walk();
        root.named_children(&mut cursor)
            .filter(|child| child.kind() == "statement")
            .try_for_each(|child| self.walk_statement(child))
    }

    fn walk_statement(&mut self, node: tree_sitter::Node) -> Result<(), ParseError> {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "base" => self.walk_base(child)?,
                "prefix" => self.walk_prefix(child)?,
                "triples" => self.walk_triples(child)?,
                "version" => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn walk_base(&mut self, node: tree_sitter::Node) -> Result<(), ParseError> {
        if let Some(iri_node) = node.child_by_field_name("iri") {
            let raw = self.node_text(&iri_node);
            let iri = strip_angle_brackets(raw);
            self.base_iri = Some(iri.to_string());
        }
        Ok(())
    }

    fn walk_prefix(&mut self, node: tree_sitter::Node) -> Result<(), ParseError> {
        let label_node = node.child_by_field_name("prefix_label");
        let iri_node = node.child_by_field_name("iri");

        if let (Some(label), Some(iri)) = (label_node, iri_node) {
            let prefix = self.node_text(&label);
            let prefix = prefix.strip_suffix(':').unwrap_or(prefix);
            let namespace = strip_angle_brackets(self.node_text(&iri));
            self.prefixes
                .insert(prefix.to_string(), namespace.to_string());
        }
        Ok(())
    }

    fn walk_triples(&mut self, node: tree_sitter::Node) -> Result<(), ParseError> {
        let span = SourceSpan::from_node(&node);
        match node.child_by_field_name("subject") {
            Some(subject_node) => {
                let subject = self.resolve_subject(subject_node)?;
                self.walk_predicate_object_lists(node, &subject, &span)
            }
            None => self.walk_blank_subject_triples(node, &span),
        }
    }

    fn walk_blank_subject_triples(
        &mut self,
        node: tree_sitter::Node,
        span: &SourceSpan,
    ) -> Result<(), ParseError> {
        let mut cursor = node.walk();
        let Some(property_list) = node
            .named_children(&mut cursor)
            .find(|child| child.kind() == "blankNodePropertyList")
        else {
            return Ok(());
        };
        let subject = NamedOrBlankNode::from(self.next_blank());
        self.walk_predicate_object_lists(property_list, &subject, span)?;
        self.walk_predicate_object_lists(node, &subject, span)
    }

    fn walk_predicate_object_lists(
        &mut self,
        node: tree_sitter::Node,
        subject: &NamedOrBlankNode,
        span: &SourceSpan,
    ) -> Result<(), ParseError> {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .filter(|child| child.kind() == "predicateObjectList")
            .try_for_each(|child| self.walk_predicate_object_list(child, subject, span))
    }

    fn walk_predicate_object_list(
        &mut self,
        node: tree_sitter::Node,
        subject: &NamedOrBlankNode,
        span: &SourceSpan,
    ) -> Result<(), ParseError> {
        let mut current_predicate: Option<NamedNode> = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "objectList" => {
                    self.walk_object_list_for_predicate(
                        child,
                        subject,
                        current_predicate.as_ref(),
                        span,
                    )?;
                }
                ";" | "," => {}
                _ => current_predicate = Some(self.resolve_verb(child)?),
            }
        }
        Ok(())
    }

    fn walk_object_list_for_predicate(
        &mut self,
        node: tree_sitter::Node,
        subject: &NamedOrBlankNode,
        predicate: Option<&NamedNode>,
        span: &SourceSpan,
    ) -> Result<(), ParseError> {
        match predicate {
            Some(predicate) => self.walk_object_list(node, subject, predicate, span),
            None => Ok(()),
        }
    }

    fn resolve_verb(&self, node: tree_sitter::Node) -> Result<NamedNode, ParseError> {
        let text = self.node_text(&node);
        if text == "a" {
            return Ok(vocab::rdf::TYPE.into_owned());
        }

        match node.kind() {
            "IRIREF" => self.resolve_iriref(node),
            "PrefixedName" => self.resolve_prefixed_name(node),
            _ => self.resolve_iri_from_children(node, format!("unexpected verb: {text}")),
        }
    }

    fn walk_object_list(
        &mut self,
        node: tree_sitter::Node,
        subject: &NamedOrBlankNode,
        predicate: &NamedNode,
        span: &SourceSpan,
    ) -> Result<(), ParseError> {
        let mut cursor = node.walk();
        for child in node.children_by_field_name("object", &mut cursor) {
            let object = self.resolve_object(child)?;
            self.triples.push(SpannedTriple {
                triple: Triple::new(subject.clone(), predicate.clone(), object),
                span: span.clone(),
            });
        }
        Ok(())
    }

    fn resolve_subject(&mut self, node: tree_sitter::Node) -> Result<NamedOrBlankNode, ParseError> {
        match node.kind() {
            "IRIREF" => Ok(self.resolve_iriref(node)?.into()),
            "PrefixedName" => Ok(self.resolve_prefixed_name(node)?.into()),
            "BLANK_NODE_LABEL" => Ok(self.resolve_blank_node_label(node).into()),
            "ANON" => Ok(self.next_blank().into()),
            "collection" => self.collect_list(node),
            _ => self.resolve_subject_from_children(node),
        }
    }

    fn resolve_subject_from_children(
        &mut self,
        node: tree_sitter::Node,
    ) -> Result<NamedOrBlankNode, ParseError> {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .find_map(|child| self.resolve_subject(child).ok())
            .ok_or_else(|| syntax_error(&node, format!("unexpected subject kind: {}", node.kind())))
    }

    fn resolve_object(&mut self, node: tree_sitter::Node) -> Result<Term, ParseError> {
        match node.kind() {
            "IRIREF" => Ok(self.resolve_iriref(node)?.into()),
            "PrefixedName" => Ok(self.resolve_prefixed_name(node)?.into()),
            "BLANK_NODE_LABEL" => Ok(self.resolve_blank_node_label(node).into()),
            "ANON" => Ok(self.next_blank().into()),
            "INTEGER" => Ok(self.typed_literal(node, vocab::xsd::INTEGER)),
            "DECIMAL" => Ok(self.typed_literal(node, vocab::xsd::DECIMAL)),
            "DOUBLE" => Ok(self.typed_literal(node, vocab::xsd::DOUBLE)),
            "BooleanLiteral" => Ok(self.typed_literal(node, vocab::xsd::BOOLEAN)),
            "RDFLiteral" => self.resolve_rdf_literal(node),
            "blankNodePropertyList" => self.resolve_blank_node_property_list(node),
            "collection" => Ok(named_or_blank_to_term(self.collect_list(node)?)),
            _ => self.resolve_object_from_children(node),
        }
    }

    fn resolve_object_from_children(
        &mut self,
        node: tree_sitter::Node,
    ) -> Result<Term, ParseError> {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .find_map(|child| self.resolve_object(child).ok())
            .ok_or_else(|| syntax_error(&node, format!("unexpected object kind: {}", node.kind())))
    }

    fn resolve_blank_node_label(&self, node: tree_sitter::Node) -> BlankNode {
        let text = self.node_text(&node);
        let identifier = text.strip_prefix("_:").unwrap_or(text);
        BlankNode::new_unchecked(identifier)
    }

    fn resolve_blank_node_property_list(
        &mut self,
        node: tree_sitter::Node,
    ) -> Result<Term, ParseError> {
        let blank_node = self.next_blank();
        let subject = NamedOrBlankNode::from(blank_node.clone());
        let span = SourceSpan::from_node(&node);
        self.walk_predicate_object_lists(node, &subject, &span)?;
        Ok(blank_node.into())
    }

    fn typed_literal(&self, node: tree_sitter::Node, datatype: NamedNodeRef<'static>) -> Term {
        Literal::new_typed_literal(self.node_text(&node), datatype).into()
    }

    fn collect_list(&mut self, node: tree_sitter::Node) -> Result<NamedOrBlankNode, ParseError> {
        let span = SourceSpan::from_node(&node);
        let mut cursor = node.walk();
        let objects = node
            .named_children(&mut cursor)
            .map(|item| self.resolve_object(item))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(objects.into_iter().rev().fold(rdf_nil(), |rest, object| {
            self.prepend_list_node(object, rest, &span)
        }))
    }

    fn prepend_list_node(
        &mut self,
        object: Term,
        rest: NamedOrBlankNode,
        span: &SourceSpan,
    ) -> NamedOrBlankNode {
        let blank_node = self.next_blank();
        self.triples.push(SpannedTriple {
            triple: Triple::new(blank_node.clone(), vocab::rdf::FIRST.into_owned(), object),
            span: span.clone(),
        });
        self.triples.push(SpannedTriple {
            triple: Triple::new(
                blank_node.clone(),
                vocab::rdf::REST.into_owned(),
                named_or_blank_to_term(rest),
            ),
            span: span.clone(),
        });
        blank_node.into()
    }

    fn resolve_rdf_literal(&self, node: tree_sitter::Node) -> Result<Term, ParseError> {
        let value = node
            .child_by_field_name("string")
            .map(|string_node| unescape_string(self.node_text(&string_node)))
            .unwrap_or_default();

        if let Some(language_node) = node.child_by_field_name("language_tag") {
            let tag = self.node_text(&language_node);
            let tag = tag.strip_prefix('@').unwrap_or(tag);
            return Ok(Literal::new_language_tagged_literal_unchecked(value, tag).into());
        }
        if let Some(datatype_node) = node.child_by_field_name("datatype_iri") {
            let datatype = self.resolve_iri_node(datatype_node)?;
            return Ok(Literal::new_typed_literal(value, datatype).into());
        }
        Ok(Literal::new_simple_literal(value).into())
    }

    fn resolve_iri_node(&self, node: tree_sitter::Node) -> Result<NamedNode, ParseError> {
        match node.kind() {
            "IRIREF" => self.resolve_iriref(node),
            "PrefixedName" => self.resolve_prefixed_name(node),
            _ => self.resolve_iri_from_children(node, format!("expected IRI, got {}", node.kind())),
        }
    }

    fn resolve_iri_from_children(
        &self,
        node: tree_sitter::Node,
        message: String,
    ) -> Result<NamedNode, ParseError> {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .find_map(|child| match child.kind() {
                "IRIREF" => Some(self.resolve_iriref(child)),
                "PrefixedName" => Some(self.resolve_prefixed_name(child)),
                _ => None,
            })
            .unwrap_or_else(|| Err(syntax_error(&node, message)))
    }

    fn resolve_iriref(&self, node: tree_sitter::Node) -> Result<NamedNode, ParseError> {
        let raw = self.node_text(&node);
        let iri = strip_angle_brackets(raw);
        let resolved = self.resolve_iri(iri)?;
        NamedNode::new(&resolved).map_err(|_| ParseError::InvalidIri(resolved))
    }

    fn resolve_prefixed_name(&self, node: tree_sitter::Node) -> Result<NamedNode, ParseError> {
        let text = self.node_text(&node);
        let (prefix, local) = text.split_once(':').unwrap_or((text, ""));

        let namespace = self
            .prefixes
            .get(prefix)
            .ok_or_else(|| ParseError::UndefinedPrefix(prefix.to_string()))?;

        let full = format!("{namespace}{local}");
        NamedNode::new(&full).map_err(|_| ParseError::InvalidIri(full))
    }

    fn resolve_iri(&self, iri: &str) -> Result<String, ParseError> {
        if iri.contains(':') {
            return Ok(iri.to_string());
        }
        let Some(base) = &self.base_iri else {
            return Err(ParseError::InvalidIri(iri.to_string()));
        };
        if iri.is_empty() {
            return Ok(base.clone());
        }
        let base_without_fragment = base.split('#').next().unwrap_or(base);
        if iri.starts_with('#') {
            return Ok(format!("{base_without_fragment}{iri}"));
        }
        match base_without_fragment.rfind('/') {
            Some(index) => Ok(format!("{}{iri}", &base_without_fragment[..=index])),
            None => Ok(format!("{base}/{iri}")),
        }
    }
}

fn rdf_nil() -> NamedOrBlankNode {
    NamedNode::new_unchecked(vocab::rdf::NIL.as_str()).into()
}

fn strip_angle_brackets(s: &str) -> &str {
    s.strip_prefix('<')
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(s)
}

fn strip_enclosing_quotes(s: &str) -> &str {
    if s.starts_with("\"\"\"") || s.starts_with("'''") {
        &s[3..s.len() - 3]
    } else if s.starts_with('"') || s.starts_with('\'') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn unescape_string(s: &str) -> String {
    let inner = strip_enclosing_quotes(s);
    let mut result = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        match character {
            '\\' => result.push_str(&decode_escape_sequence(&mut characters)),
            other => result.push(other),
        }
    }
    result
}

fn decode_escape_sequence(characters: &mut std::str::Chars) -> String {
    match characters.next() {
        Some('n') => "\n".to_string(),
        Some('r') => "\r".to_string(),
        Some('t') => "\t".to_string(),
        Some('\\') => "\\".to_string(),
        Some('"') => "\"".to_string(),
        Some('\'') => "'".to_string(),
        Some('u') => decode_unicode_escape(characters, 4),
        Some('U') => decode_unicode_escape(characters, 8),
        Some(other) => format!("\\{other}"),
        None => "\\".to_string(),
    }
}

fn decode_unicode_escape(characters: &mut std::str::Chars, digit_count: usize) -> String {
    let digits: String = characters.by_ref().take(digit_count).collect();
    u32::from_str_radix(&digits, 16)
        .ok()
        .and_then(char::from_u32)
        .map(String::from)
        .unwrap_or_default()
}

fn named_or_blank_to_term(named_or_blank: NamedOrBlankNode) -> Term {
    match named_or_blank {
        NamedOrBlankNode::NamedNode(named) => Term::NamedNode(named),
        NamedOrBlankNode::BlankNode(blank) => Term::BlankNode(blank),
    }
}
