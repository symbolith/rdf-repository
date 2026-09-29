pub use rdf_trilith_graph::Graph;
use rdf_trilith_term::{Object, Predicate, Subject};

use crate::Error;

pub struct TriplePattern<'a> {
    pub subject: Option<Subject<'a>>,
    pub predicate: Option<Predicate<'a>>,
    pub object: Option<Object<'a>>,
}

pub trait Lookup {
    type Location<'a>
    where
        Self: 'a;

    fn lookup<'a>(&'a self, pattern: TriplePattern<'_>)
    -> impl Iterator<Item = Self::Location<'a>>;
}

pub trait RdfNode: Graph {
    type Config<'a>;

    fn construct(config: Self::Config<'_>) -> Result<Self, Error>
    where
        Self: Sized;
}
