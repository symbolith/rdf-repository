# rdf-r Architecture

## Overview

- rdf-repository is the store. There is no separate database.
- Markdown and Turtle files are the persistence format. Humans edit them directly.
- Bidirectional: files are read into a graph; mutations of the graph are written back into the files.

## RDF Tree

- produces triples
- accepts mutations
- points at where a term or statement is represented

- Composites all the way down to span leaves:
  - repo -> file -> block (yaml, turtle, body) -> statement -> span
- Each level owns the syntax problems of its own granularity
- The repo is itself a RDF Tree, a markdown file is a RDF Tree etc.

### Mutation

- A mutation changes only what it must. Human formatting survives.

### IRI construction

- IRIs are formed by base layering per RFC 3986:
  1. @base in Turtle, or the JSON-LD context
  2. File path relative to the repo root
  3. Base declared in config.ttl
- When adding triples, SHACL shapes can help constructing the IRI.

### Lookup

- Any term, statement pattern? can be looked up: the tree yields every place it is represented.
- Locations mirror the tree: each store speaks its own location language,
- Mutations look up first, then edit.

## Modules

```
rdf-r/src
├── config.rs         config.ttl: base, shapes, prefixes
├── tree.rs          
├── store/
│   ├── repo.rs           Repo: placement + conformance, routed write
│   ├── markdown.rs   composite: yaml + turtle + body blocks
│   ├── turtle.rs     composite: statement groups of one turtle text
│   ├── turtle/
│   │   └── statement.rs  leaf: one span, its triples
│   ├── yaml.rs       composite: frontmatter keys; leaf values
│   ├── wikilink.rs   leaf: one body link
│   └── sparql.rs     leaf: endpoint (later)
```
