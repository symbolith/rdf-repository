use rdf_trilith_graph::{Triple, TripleBuf};
use rdf_trilith_iri::IriAbsolute;
use oxrdfio::{JsonLdProfileSet, RdfFormat, RdfParser};
use snafu::{OptionExt, ResultExt, ensure};

use crate::store::wikilink::note_link_target;
use crate::tree::{Graph, RdfNode};
use crate::{
    Error, InvalidContextSnafu, IriSnafu, JsonLdDocumentSnafu, NotJsonCompatibleSnafu, RdfSnafu,
    UnmappedKeySnafu, UnsupportedSnafu, YamlSnafu,
};

pub struct YamlBlock {
    triples: Vec<TripleBuf>,
}

pub struct Schema {
    pub context: serde_json::Value,
}

pub struct YamlConfig<'a> {
    pub text: &'a str,
    pub subject: &'a IriAbsolute,
    pub schema: &'a Schema,
}

impl Graph for YamlBlock {
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

impl RdfNode for YamlBlock {
    type Config<'a> = YamlConfig<'a>;

    fn construct(config: Self::Config<'_>) -> Result<Self, Error> {
        let subject: &str = config.subject;
        let yaml = serde_yaml::from_str(config.text).context(YamlSnafu)?;
        if matches!(yaml, serde_yaml::Value::Null) {
            return Ok(YamlBlock {
                triples: Vec::new(),
            });
        }
        let serde_json::Value::Object(properties) = yaml_to_json(yaml)? else {
            return NotJsonCompatibleSnafu.fail();
        };
        let terms = config
            .schema
            .context
            .as_object()
            .context(InvalidContextSnafu)?;
        for key in properties.keys() {
            ensure!(terms.contains_key(key), UnmappedKeySnafu { key });
        }
        let properties = properties.into_iter().map(|(key, value)| {
            let value = if iri_term(&terms[&key]) {
                rewrite_markdown_links(value)
            } else {
                value
            };
            (key, value)
        });
        let mut document = serde_json::Map::new();
        document.insert("@context".to_string(), config.schema.context.clone());
        document.insert("@id".to_string(), subject.into());
        document.extend(properties);
        let document = serde_json::to_string(&document).context(JsonLdDocumentSnafu)?;
        let format = RdfFormat::JsonLd {
            profile: JsonLdProfileSet::empty(),
        };
        let parser = RdfParser::from_format(format)
            .with_base_iri(subject)
            .context(IriSnafu)?
            .for_slice(&document);
        let triples = parser
            .map(|quad| {
                TripleBuf::try_from(oxrdf::Triple::from(quad.context(RdfSnafu)?))
                    .context(UnsupportedSnafu)
            })
            .collect::<Result<_, _>>()?;
        Ok(YamlBlock { triples })
    }
}

fn iri_term(term: &serde_json::Value) -> bool {
    term.as_object()
        .and_then(|term| term.get("@type"))
        .is_some_and(|kind| kind == "@id")
}

fn yaml_to_json(value: serde_yaml::Value) -> Result<serde_json::Value, Error> {
    match value {
        serde_yaml::Value::Null => Ok(serde_json::Value::Null),
        serde_yaml::Value::Bool(boolean) => Ok(serde_json::Value::Bool(boolean)),
        serde_yaml::Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(integer.into())
            } else if let Some(unsigned) = number.as_u64() {
                Ok(unsigned.into())
            } else {
                number
                    .as_f64()
                    .and_then(serde_json::Number::from_f64)
                    .map(serde_json::Value::Number)
                    .context(NotJsonCompatibleSnafu)
            }
        }
        serde_yaml::Value::String(string) => Ok(serde_json::Value::String(string)),
        serde_yaml::Value::Sequence(sequence) => Ok(serde_json::Value::Array(
            sequence
                .into_iter()
                .map(yaml_to_json)
                .collect::<Result<_, _>>()?,
        )),
        serde_yaml::Value::Mapping(mapping) => Ok(serde_json::Value::Object(
            mapping
                .into_iter()
                .map(yaml_entry_to_json)
                .collect::<Result<_, _>>()?,
        )),
        serde_yaml::Value::Tagged(_) => NotJsonCompatibleSnafu.fail(),
    }
}

fn yaml_entry_to_json(
    (key, value): (serde_yaml::Value, serde_yaml::Value),
) -> Result<(String, serde_json::Value), Error> {
    let serde_yaml::Value::String(key) = key else {
        return NotJsonCompatibleSnafu.fail();
    };
    Ok((key, yaml_to_json(value)?))
}

fn rewrite_markdown_links(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(string) => serde_json::Value::String(
            markdown_link_target(&string)
                .map(str::to_string)
                .unwrap_or(string),
        ),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(rewrite_markdown_links).collect())
        }
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .map(|(key, value)| (key, rewrite_markdown_links(value)))
                .collect(),
        ),
        other => other,
    }
}

fn markdown_link_target(value: &str) -> Option<&str> {
    let (_, destination) = value.strip_prefix('[')?.split_once("](")?;
    note_link_target(destination.strip_suffix(')')?)
}
