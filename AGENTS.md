# AGENTS.md

## Project

rdf-repository — Rust tooling for markdown files with RDF metadata. Bidirectional: parse RDF from markdown frontmatter, generate/embed RDF into markdown, query/index collections.

## Architecture

Virtual Cargo workspace, all crates under `crates/`.

- **rdf-repository-shacl** (`crates/rdf-repository-shacl/`): core library, parsing/generating markdown with RDF metadata
- **rdf-repository-export** (`crates/rdf-repository-export/`): Obsidian wiki to RDF/Turtle exporter
- **rdf-repository-task** (`crates/rdf-repository-task/`): markdown task summary tool

## Key Decisions

- **RDF data model**: rdf-trilith crates (`rdf-trilith-term` terms, `rdf-trilith-graph` triples and `Graph` traits) in rdf-repository-export; `oxrdf`/`oxrdfio` only at the parse/serialize boundary via the rdf-trilith `oxrdf` compat features
- **Frontmatter RDF formats**: Turtle, JSON-LD, N-Triples, RDF/XML, N-Quads, TriG
- **Markdown parser**: TBD (comrak or pulldown-cmark)
- **Edition**: Rust 2024

## Conventions

- No inline RDF yet (deferred)
- Additional deps (markdown parser, SPARQL, CLI framework) added as features land
