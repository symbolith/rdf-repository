use std::fs;
use std::path::Path;

use rdf_trilith_graph::{Triple, TripleBuf};
use rdf_trilith_iri::{Iri, IriAbsolute, IriRelative};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use snafu::ResultExt;
use rdf_trilith_term::{LexicalFormBuf, Literal};

use crate::store::turtle::{TurtleBlock, TurtleConfig};
use crate::store::wikilink::{Wikilink, WikilinkConfig, note_link_target};
use crate::store::yaml::{Schema, YamlBlock, YamlConfig};
use crate::tree::{Graph, RdfNode};
use crate::{Error, ReadSnafu, ResolveSnafu, TermSnafu, UnclosedFrontmatterSnafu};

pub struct MarkdownFile {
    frontmatter: Option<FrontmatterBlock>,
    links: Vec<Wikilink>,
    body: Option<TripleBuf>,
}

enum FrontmatterBlock {
    Turtle(TurtleBlock),
    Yaml(YamlBlock),
}

pub struct MarkdownConfig<'a> {
    pub path: &'a Path,
    pub base: &'a IriAbsolute,
    pub link_predicate: &'a Iri,
    pub body_predicate: &'a Iri,
    pub schema: &'a Schema,
}

impl Graph for MarkdownFile {
    type Triple<'g> = Triple<'g>;

    fn triples(&self) -> impl Iterator<Item = Triple<'_>> {
        self.frontmatter
            .iter()
            .flat_map(|block| block.triples())
            .chain(self.links.iter().flat_map(|link| link.triples()))
            .chain(self.body.iter().map(Triple::from))
    }

    fn contains<'s>(&'s self, triple: Triple<'s>) -> bool {
        self.triples().any(|candidate| candidate == triple)
    }

    fn len(&self) -> usize {
        self.frontmatter.iter().map(Graph::len).sum::<usize>()
            + self.links.len()
            + self.body.iter().count()
    }
}

impl Graph for FrontmatterBlock {
    type Triple<'g> = Triple<'g>;

    fn triples(&self) -> impl Iterator<Item = Triple<'_>> {
        let triples: Box<dyn Iterator<Item = Triple<'_>> + '_> = match self {
            FrontmatterBlock::Turtle(block) => Box::new(block.triples()),
            FrontmatterBlock::Yaml(block) => Box::new(block.triples()),
        };
        triples
    }

    fn contains<'s>(&'s self, triple: Triple<'s>) -> bool {
        match self {
            FrontmatterBlock::Turtle(block) => block.contains(triple),
            FrontmatterBlock::Yaml(block) => block.contains(triple),
        }
    }

    fn len(&self) -> usize {
        match self {
            FrontmatterBlock::Turtle(block) => block.len(),
            FrontmatterBlock::Yaml(block) => block.len(),
        }
    }
}

impl RdfNode for MarkdownFile {
    type Config<'a> = MarkdownConfig<'a>;

    fn construct(config: Self::Config<'_>) -> Result<Self, Error> {
        let markdown = fs::read_to_string(config.path).context(ReadSnafu { path: config.path })?;
        let markdown = markdown.strip_prefix('\u{feff}').unwrap_or(&markdown);
        let (frontmatter, body) = split_frontmatter(markdown)?;
        let links = body_links(body, config.base, config.link_predicate)?;
        let body = body_triple(body, config.base, config.body_predicate)?;
        let frontmatter = match frontmatter {
            Some(RawFrontmatter::Turtle(text)) => Some(FrontmatterBlock::Turtle(
                TurtleBlock::construct(TurtleConfig {
                    text,
                    base: config.base,
                })?,
            )),
            Some(RawFrontmatter::Yaml(text)) => {
                Some(FrontmatterBlock::Yaml(YamlBlock::construct(YamlConfig {
                    text: &text,
                    subject: config.base,
                    schema: config.schema,
                })?))
            }
            None => None,
        };
        Ok(MarkdownFile {
            frontmatter,
            links,
            body,
        })
    }
}

enum RawFrontmatter<'a> {
    Turtle(&'a str),
    Yaml(String),
}

fn split_frontmatter(markdown: &str) -> Result<(Option<RawFrontmatter<'_>>, &str), Error> {
    let mut events =
        Parser::new_ext(markdown, Options::ENABLE_YAML_STYLE_METADATA_BLOCKS).into_offset_iter();
    if let Some((Event::Start(Tag::MetadataBlock(_)), range)) = events.next() {
        let block = events
            .take_while(|(event, _)| !matches!(event, Event::End(TagEnd::MetadataBlock(_))))
            .filter_map(|(event, _)| match event {
                Event::Text(text) => Some(text.into_string()),
                _ => None,
            })
            .collect();
        return Ok((Some(RawFrontmatter::Yaml(block)), &markdown[range.end..]));
    }
    let Some(rest) = markdown.strip_prefix("---turtle").and_then(|rest| {
        rest.strip_prefix("\r\n")
            .or_else(|| rest.strip_prefix('\n'))
    }) else {
        return Ok((None, markdown));
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            let block = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return Ok((Some(RawFrontmatter::Turtle(block)), body));
        }
        offset += line.len();
    }
    UnclosedFrontmatterSnafu.fail()
}

fn body_links(body: &str, base: &IriAbsolute, predicate: &Iri) -> Result<Vec<Wikilink>, Error> {
    Parser::new(body)
        .filter_map(|event| match event {
            Event::Start(Tag::Link {
                dest_url: destination,
                ..
            }) => note_link_target(&destination).map(str::to_string),
            _ => None,
        })
        .map(|stem| {
            let target = IriRelative::new(&stem)
                .context(ResolveSnafu { reference: &stem })?
                .resolve(base);
            Wikilink::construct(WikilinkConfig {
                subject: base,
                predicate,
                target: &target,
            })
        })
        .collect()
}

fn body_triple(
    body: &str,
    base: &IriAbsolute,
    predicate: &Iri,
) -> Result<Option<TripleBuf>, Error> {
    if body.trim().is_empty() {
        return Ok(None);
    }
    let literal = Literal::Simple {
        lexical_form: LexicalFormBuf::new(body).context(TermSnafu)?,
    };
    Ok(Some(TripleBuf::new(
        base.to_owned(),
        predicate.to_owned(),
        literal,
    )))
}
