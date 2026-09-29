# rdf-md Architecture

## Overview

rdf-md's internal architecture — modules, data flow, key types.

## Module Structure

- **parser** — splits an rdf-md document into a frontmatter block and body; parses frontmatter RDF using `tree-sitter-turtle` (RDF 1.2 Turtle grammar); walks the AST, capturing source spans for all nodes; extracts triples and resolves prefixes and base IRIs
- **vocabulary** — defines the rdf-md RDF vocabulary: document class, body property, namespace constants
- **provenance** — annotates extracted triples with source spans using RDF 1.2 reification (`rdf:reifies`) and W3C Pointer Methods (`ptr:StartEndPointer`, `ptr:LineCharPointer`)
- **validation** — runs SHACL validation against the document graph; maps validation errors back to source locations via provenance annotations
- **model** — the parsed document representation: base subject, body content, annotated triple graph

## Data Flow

1. Input: file path or string
2. **parser** splits frontmatter and body, records byte spans for each region
3. **parser** parses frontmatter via `tree-sitter-turtle` into a syntax tree; walks the tree, capturing source spans for all nodes; extracts triples and resolves prefixes and base IRIs
4. **model** constructs the document: base subject resolution, body triple, type defaulting
5. **provenance** wraps each triple with a reification annotation carrying its source span
6. **validation** runs SHACL against the annotated graph; on error, looks up provenance to report file and span

## External Dependencies

- `oxrdf` — RDF data model (named nodes, literals, triples)
- `tree-sitter` + `tree-sitter-turtle` — frontmatter parsing; provides RDF 1.2 Turtle syntax tree with per-node source spans
- Markdown parser — TBD (comrak or pulldown-cmark)

## External Vocabularies

- **RDF 1.2 reification** (`rdf:reifies`) — associates provenance metadata with triples
- **W3C Pointer Methods in RDF** (`ptr:`, `http://www.w3.org/2009/pointers#`) — represents source spans

## Open Decisions

- Markdown parser choice (comrak vs pulldown-cmark) — see requirements.md
- Vocabulary namespace IRI — see requirements.md
