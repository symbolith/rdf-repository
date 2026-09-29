use std::iter;

use rdf_trilith_graph::{Triple, TripleBuf};
use rdf_trilith_iri::{Iri, IriAbsolute};

use crate::Error;
use crate::tree::{Graph, RdfNode};

pub struct Wikilink {
    triple: TripleBuf,
}

pub struct WikilinkConfig<'a> {
    pub subject: &'a IriAbsolute,
    pub predicate: &'a Iri,
    pub target: &'a Iri,
}

impl Graph for Wikilink {
    type Triple<'g> = Triple<'g>;

    fn triples(&self) -> impl Iterator<Item = Triple<'_>> {
        iter::once(Triple::from(&self.triple))
    }

    fn contains<'s>(&'s self, triple: Triple<'s>) -> bool {
        triple == Triple::from(&self.triple)
    }

    fn len(&self) -> usize {
        1
    }
}

impl RdfNode for Wikilink {
    type Config<'a> = WikilinkConfig<'a>;

    fn construct(config: Self::Config<'_>) -> Result<Self, Error> {
        Ok(Wikilink {
            triple: TripleBuf::new(
                config.subject.to_owned(),
                config.predicate.to_owned(),
                config.target.to_owned(),
            ),
        })
    }
}

pub(crate) fn note_link_target(destination: &str) -> Option<&str> {
    let destination = destination
        .split_once('#')
        .map_or(destination, |(path, _)| path);
    let target = destination.strip_suffix(".md")?;
    (!target.is_empty() && !target.contains(':')).then_some(target)
}
