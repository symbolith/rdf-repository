use oxrdf::{Graph, Term};

use crate::parser::SourceSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Violation,
    Warning,
    Info,
}

#[derive(Debug)]
pub struct ValidationError {
    pub message: String,
    pub focus_node: Term,
    pub source_span: Option<SourceSpan>,
    pub severity: Severity,
}

pub fn validate(_graph: &Graph) -> Vec<ValidationError> {
    Vec::new()
}
