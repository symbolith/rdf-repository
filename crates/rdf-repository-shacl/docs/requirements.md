# rdf-md Requirements

## Overview

rdf-md is a CLI and library for parsing markdown files with RDF metadata.

## Definitions

- **rdf-md document** — a markdown file with an RDF frontmatter block; the frontmatter contains RDF triples, the rest is markdown content
- **Base subject** — the RDF resource that the document describes; identified by an IRI
- **Body property** — the RDF property used to attach the markdown content to the base subject
- **rdf-md vocabulary** — an RDF vocabulary defined by this project for describing rdf-md documents

## Document Model

- **Structure**
  - An rdf-md document consists of an RDF frontmatter block and markdown content
  - rdf-md processes the entire file, not just the frontmatter
- **Base subject**
  - The frontmatter defines the base subject of the document
  - When the frontmatter does not specify a base subject, it defaults to the document's IRI (e.g. its file path)
  - The base subject carries an `rdf:type`, set in the frontmatter or defaulted from the rdf-md vocabulary
- **Body property**
  - The markdown content is attached to the base subject via the body property
  - The body property defaults to a property defined in the rdf-md vocabulary
  - The frontmatter can override the body property (e.g. `md:body_property`)

## Vocabulary

rdf-md defines its own RDF vocabulary containing:

- **Document class** — the default `rdf:type` for rdf-md documents
- **Body property** — attaches markdown content to the base subject
  - Declares an `rdfs:range`
- **W3C Pointer Methods in RDF** (`ptr:`, `http://www.w3.org/2009/pointers#`) — source span representation
- **RDF 1.2 reification** (`rdf:reifies`) — provenance annotation mechanism

## Validation

- **SHACL shape** — a shape for the rdf-md document type that validates document structure
- **KG ↔ document mapping** — traces SHACL validation errors back to source spans in rdf-md documents
  - Each triple extracted from a document has a provenance annotation recording its source span
  - Uses RDF 1.2 reification (`rdf:reifies`) to associate provenance with triples
  - Uses W3C Pointer Methods in RDF (`ptr:`) for source spans (`ptr:StartEndPointer`, `ptr:LineCharPointer`)
  - Frontmatter triples: one annotation per triple, spanning the triple's source text
  - Body triple: one annotation spanning the body section
  - SHACL validation errors that reference a triple with provenance report the source file and span
  - Errors referencing triples without provenance (e.g. inferred) are reported without a source location

## Serialization

Supported frontmatter RDF format: Turtle

## Open Decisions

- Markdown parser (comrak vs pulldown-cmark)
- Vocabulary namespace IRI
- Default body property IRI
- Default resource type IRI

## Out of Scope

- Inline RDF
