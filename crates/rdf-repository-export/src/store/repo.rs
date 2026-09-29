use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use rdf_trilith_graph::Triple;
use rdf_trilith_iri::{Iri, IriAbsolute, IriAbsoluteBuf};
use snafu::{OptionExt, ResultExt};

use crate::store::markdown::{MarkdownConfig, MarkdownFile};
use crate::store::turtle::{TurtleBlock, TurtleConfig};
use crate::store::yaml::Schema;
use crate::tree::{Graph, RdfNode};
use crate::{Error, ReadSnafu, ResolveSnafu, Utf8PathSnafu};

pub struct Repo {
    files: Vec<File>,
}

enum File {
    Markdown(MarkdownFile),
    Turtle(TurtleBlock),
}

pub struct RepoConfig<'a> {
    pub root: &'a Path,
    pub base: &'a IriAbsolute,
    pub link_predicate: &'a Iri,
    pub body_predicate: &'a Iri,
    pub schema: &'a Schema,
}

impl Graph for Repo {
    type Triple<'g> = Triple<'g>;

    fn triples(&self) -> impl Iterator<Item = Triple<'_>> {
        self.files.iter().flat_map(|file| file.triples())
    }

    fn contains<'s>(&'s self, triple: Triple<'s>) -> bool {
        self.files.iter().any(|file| file.contains(triple))
    }

    fn len(&self) -> usize {
        self.files.iter().map(Graph::len).sum()
    }
}

impl Graph for File {
    type Triple<'g> = Triple<'g>;

    fn triples(&self) -> impl Iterator<Item = Triple<'_>> {
        let triples: Box<dyn Iterator<Item = Triple<'_>> + '_> = match self {
            File::Markdown(file) => Box::new(file.triples()),
            File::Turtle(file) => Box::new(file.triples()),
        };
        triples
    }

    fn contains<'s>(&'s self, triple: Triple<'s>) -> bool {
        match self {
            File::Markdown(file) => file.contains(triple),
            File::Turtle(file) => file.contains(triple),
        }
    }

    fn len(&self) -> usize {
        match self {
            File::Markdown(file) => file.len(),
            File::Turtle(file) => file.len(),
        }
    }
}

impl RdfNode for Repo {
    type Config<'a> = RepoConfig<'a>;

    fn construct(config: Self::Config<'_>) -> Result<Self, Error> {
        let files = collect_paths(config.root)?
            .iter()
            .filter_map(|path| File::read(&config, path).transpose())
            .collect::<Result<_, _>>()?;
        Ok(Repo { files })
    }
}

fn collect_paths(directory: &Path) -> Result<Vec<PathBuf>, Error> {
    let entries = fs::read_dir(directory)
        .context(ReadSnafu { path: directory })?
        .map(|entry| entry.context(ReadSnafu { path: directory }))
        .collect::<Result<Vec<_>, _>>()?;
    let nested = entries
        .into_iter()
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .map(|entry| descend(entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut paths: Vec<PathBuf> = nested.into_iter().flatten().collect();
    paths.sort();
    Ok(paths)
}

fn descend(path: PathBuf) -> Result<Vec<PathBuf>, Error> {
    if path.is_dir() {
        collect_paths(&path)
    } else {
        Ok(vec![path])
    }
}

impl File {
    fn read(repo: &RepoConfig<'_>, path: &Path) -> Result<Option<Self>, Error> {
        let file = match path.extension().and_then(OsStr::to_str) {
            Some("md") => File::Markdown(MarkdownFile::construct(MarkdownConfig {
                path,
                base: &document_iri(repo.root, repo.base, path)?,
                link_predicate: repo.link_predicate,
                body_predicate: repo.body_predicate,
                schema: repo.schema,
            })?),
            Some("ttl") if path != repo.root.join("config.ttl") => {
                let text = fs::read_to_string(path).context(ReadSnafu { path })?;
                File::Turtle(TurtleBlock::construct(TurtleConfig {
                    text: &text,
                    base: &document_iri(repo.root, repo.base, path)?,
                })?)
            }
            _ => return Ok(None),
        };
        Ok(Some(file))
    }
}

fn document_iri(root: &Path, base: &IriAbsolute, path: &Path) -> Result<IriAbsoluteBuf, Error> {
    let relative = path.strip_prefix(root).unwrap_or(path).with_extension("");
    let relative = relative.to_str().context(Utf8PathSnafu { path })?;
    let base: &str = base;
    let base = base.trim_end_matches('/');
    let iri = format!("{base}/{}", encode_path(relative));
    IriAbsoluteBuf::new(&iri).context(ResolveSnafu { reference: &iri })
}

fn encode_path(relative: &str) -> String {
    relative.chars().map(encoded_character).collect()
}

fn encoded_character(character: char) -> String {
    let allowed = character.is_ascii_alphanumeric()
        || !character.is_ascii()
        || "-._~!$&'()*+,;=:@/".contains(character);
    if allowed {
        character.to_string()
    } else {
        format!("%{:02X}", character as u32)
    }
}
