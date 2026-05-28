# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Crabref is a Rust library for managing academic paper references, integrating with multiple academic sources (ArXiv, Semantic Scholar, OpenAlex).

## Build Commands

```bash
cargo build        # Build the library
cargo check       # Type-check without full build
cargo test        # Run all tests
cargo test <name> # Run specific test
cargo run --example <name>  # Run an example
```

## Architecture

### Source Module (`src/source/`)

Implements paper source integrations via the `PaperSource` trait:

- **Trait**: `PaperSource` defines async methods for `search`, `get_by_id`, `get_by_identifier`, `get_citations`, `get_references`, and `health_check`
- **Sources**: `ArxivSource`, `SemanticScholarSource`, `OpenAlexSource`
- **Manager**: `SourceManager` coordinates multiple sources, supports smart search (query all sources), and identifier auto-detection
- **Identifier**: Parses strings like `arxiv:2301.00001`, `doi:10.xxxx/...`, `ss:...`, `openalex:...`

### Models Module (`src/models/`)

Core data structures:

- **Paper**: title, abstract, arxiv_id, semantic_scholar_id, doi, pdf_url, citation_count, authors, etc.
- **Author**: name, semantic_scholar_id, orcid
- **Citation**: citing_paper_id, cited_paper_id

### Database

Uses `toasty` ORM (PostgreSQL) for persistence. Models are registered with `toasty::models!()` macro. See `examples/test_db.rs` for usage pattern.

## Dependencies

Key crates: `tokio` (async runtime), `reqwest` (HTTP client), `serde` (serialization), `async_trait`, `chrono` (dates), `toasty` (PostgreSQL ORM).