use std::ops::Range;

use rdf_trilith_graph::{Triple, TripleBuf};

use crate::Error;
use crate::tree::{Graph, RdfNode};

pub struct Statement {
    pub span: Range<usize>,
    triples: Vec<TripleBuf>,
}

pub struct StatementConfig {
    pub span: Range<usize>,
    pub triples: Vec<TripleBuf>,
}

impl Graph for Statement {
    type Triple<'g> = Triple<'g>;

    fn triples(&self) -> impl Iterator<Item = Triple<'_>> {
        self.triples.iter().map(Triple::from)
    }

    fn contains<'s>(&'s self, triple: Triple<'s>) -> bool {
        self.triples().any(|candidate| candidate == triple)
    }

    fn len(&self) -> usize {
        self.triples.len()
    }
}

impl RdfNode for Statement {
    type Config<'a> = StatementConfig;

    fn construct(config: Self::Config<'_>) -> Result<Self, Error> {
        Ok(Statement {
            span: config.span,
            triples: config.triples,
        })
    }
}
