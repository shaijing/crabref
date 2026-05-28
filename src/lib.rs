//! Crabref - A paper reference management library

pub mod citation;
pub mod config;
pub mod core;
pub mod db;
pub mod models;
pub mod output;
pub mod pdf;
pub mod source;

pub use citation::{CitationCrawler, CitationGraph, CitationStats, CrawlDirection};
pub use config::Config;
pub use core::{CrabRef, CrabRefBuilder, GraphFormat, SortBy};
pub use db::Database;
pub use models::{Author, Citation, Paper, PaperAuthor};
pub use output::{BibtexFormatter, JsonFormatter, MarkdownFormatter, OutputFormat, OutputFormatter, TerminalFormatter, CitationDirection};
pub use pdf::PdfDownloader;
pub use source::{
    Identifier, PaperSource, SearchParams, SearchResult, SourceCapabilities,
    SourceConfig, SourceError, SourceKind, SourceManager, SourceBuilder,
    ArxivSource, OpenAlexSource, SemanticScholarSource,
};