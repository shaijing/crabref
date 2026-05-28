//! Crabref CLI — paper reference management tool

use anyhow::Result;
use clap::{Parser, Subcommand};
use crabref::{Author, CrabRef, CrabRefBuilder, GraphFormat, OutputFormat, Paper, SortBy, SourceKind};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "crabref")]
#[command(about = "Paper reference management tool", long_about = None)]
struct Cli {
    /// PDF storage directory
    #[arg(long, global = true)]
    pdf_dir: Option<PathBuf>,

    /// Database URL (PostgreSQL)
    #[arg(long, global = true, default_value = "postgres://localhost/crabref")]
    database_url: String,

    /// Semantic Scholar API key
    #[arg(long, global = true)]
    ss_api_key: Option<String>,

    /// HTTP/HTTPS proxy (e.g. http://127.0.0.1:7890)
    #[arg(short, long, global = true)]
    proxy: Option<String>,

    /// Output format: terminal, json, bibtex, markdown
    #[arg(long, global = true, default_value = "terminal")]
    format: String,

    /// Output file path
    #[arg(short, long, global = true)]
    output: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search papers across all sources
    Search {
        #[arg(short, long)]
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Search arXiv
    ArxivSearch {
        #[arg(short, long)]
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Search Semantic Scholar
    SsSearch {
        #[arg(short, long)]
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Search OpenAlex
    OaSearch {
        #[arg(short, long)]
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Fetch paper by identifier (auto-detect source)
    Get {
        #[arg(short, long)]
        id: String,
    },

    /// Fetch paper by arXiv ID
    GetArxiv {
        #[arg(short, long)]
        id: String,
    },

    /// Fetch paper by Semantic Scholar ID
    GetSs {
        #[arg(short, long)]
        id: String,
    },

    /// Get citations for a paper
    Citations {
        #[arg(short = 'i', long)]
        paper_id: String,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// Get references for a paper
    References {
        #[arg(short = 'i', long)]
        paper_id: String,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// Download paper PDF
    Download {
        #[arg(short, long)]
        id: String,
    },

    /// Save a paper to database
    Save {
        #[arg(short, long)]
        title: String,
        #[arg(short = 'a', long)]
        arxiv_id: Option<String>,
        #[arg(short = 's', long)]
        ss_id: Option<String>,
    },

    /// Fetch and save paper by identifier
    Fetch {
        #[arg(short, long)]
        id: String,
    },

    /// List papers in database
    List {
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Sort by: created, citation, title
        #[arg(short, long, default_value = "created")]
        sort: String,
    },

    /// Search database
    DBSearch {
        #[arg(short, long)]
        query: String,
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Search by: title, author, all
        #[arg(short, long, default_value = "all")]
        field: String,
    },

    /// Show recent papers
    Recent {
        #[arg(short, long, default_value = "7")]
        days: i64,
    },

    /// Delete a paper
    Delete {
        #[arg(short, long)]
        id: i64,
    },

    /// Update paper metadata from source
    Update {
        #[arg(short, long)]
        id: i64,
    },

    /// Export papers
    Export {
        /// Format: bibtex, json, markdown
        #[arg(short, long, default_value = "bibtex")]
        format: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short, long)]
        query: Option<String>,
    },

    /// Generate citation graph
    CitationGraph {
        #[arg(short, long, default_value = "dot")]
        format: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show citation statistics
    CitationStats,

    /// Sync citation counts from Semantic Scholar
    SyncCitations {
        #[arg(short, long, default_value = "50")]
        batch: usize,
    },

    /// Show configuration
    Config,

    /// List available paper sources
    Sources,

    /// Push database schema (create tables)
    PushSchema,

    /// Count papers in database
    Count,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // Override DATABASE_URL from CLI argument if provided
    if cli.database_url != "postgres://localhost/crabref" || std::env::var("DATABASE_URL").is_err() {
        std::env::set_var("DATABASE_URL", &cli.database_url);
    }

    let mut builder = CrabRefBuilder::new();

    if let Some(path) = cli.pdf_dir {
        builder = builder.pdf_dir(path);
    }
    if let Some(proxy) = cli.proxy {
        builder = builder.proxy(proxy);
    }

    // Use CLI arg first, fall back to SS_API_KEY env var (from .env)
    let api_key = cli.ss_api_key.or_else(|| std::env::var("SS_API_KEY").ok());
    if let Some(key) = api_key {
        builder = builder.api_key(key);
    }

    let crabref = builder.build().await?;

    execute(&crabref, &cli.command, &cli.format, &cli.output).await
}

async fn execute(crabref: &CrabRef, cmd: &Commands, _format: &str, _output: &Option<PathBuf>) -> Result<()> {
    match cmd {
        Commands::Search { query, limit } => {
            let papers = crabref.search(query, *limit).await?;
            cache_papers(crabref, &papers).await;
            print_papers(&papers);
        }
        Commands::ArxivSearch { query, limit } => {
            let papers = crabref.search_from(SourceKind::Arxiv, query, *limit).await?;
            cache_papers(crabref, &papers).await;
            print_papers(&papers);
        }
        Commands::SsSearch { query, limit } => {
            let papers = crabref.search_from(SourceKind::SemanticScholar, query, *limit).await?;
            cache_papers(crabref, &papers).await;
            print_papers(&papers);
        }
        Commands::OaSearch { query, limit } => {
            let papers = crabref.search_from(SourceKind::OpenAlex, query, *limit).await?;
            cache_papers(crabref, &papers).await;
            print_papers(&papers);
        }
        Commands::Get { id } => {
            match crabref.fetch(id).await? {
                Some(paper) => {
                    let _ = crabref.save(&paper).await;
                    print_paper_detail(&paper);
                }
                None => println!("Paper not found"),
            }
        }
        Commands::GetArxiv { id } => {
            let full_id = if id.starts_with("arxiv:") { id.clone() } else { format!("arxiv:{}", id) };
            match crabref.fetch(&full_id).await? {
                Some(paper) => {
                    let _ = crabref.save(&paper).await;
                    print_paper_detail(&paper);
                }
                None => println!("Paper not found"),
            }
        }
        Commands::GetSs { id } => {
            let full_id = if id.starts_with("ss:") { id.clone() } else { format!("ss:{}", id) };
            match crabref.fetch(&full_id).await? {
                Some(paper) => {
                    let _ = crabref.save(&paper).await;
                    print_paper_detail(&paper);
                }
                None => println!("Paper not found"),
            }
        }
        Commands::Citations { paper_id, limit } => {
            let citations = crabref.citations(paper_id, *limit).await?;
            println!("Cited by {} papers:", citations.len());
            for (paper, authors) in &citations {
                println!("\n---");
                println!("Title: {}", paper.title);
                print_author_list(authors);
            }
        }
        Commands::References { paper_id, limit } => {
            let refs = crabref.references(paper_id, *limit).await?;
            println!("References {} papers:", refs.len());
            for (paper, authors) in &refs {
                println!("\n---");
                println!("Title: {}", paper.title);
                print_author_list(authors);
            }
        }
        Commands::Download { id } => {
            let dest = crabref.download_pdf(id).await?;
            println!("Downloaded: {}", dest.display());
        }
        Commands::Save { title, arxiv_id, ss_id } => {
            let mut paper = Paper::new(title.clone());
            if let Some(aid) = arxiv_id { paper = paper.with_arxiv_id(aid.clone()); }
            if let Some(sid) = ss_id { paper = paper.with_semantic_scholar_id(sid.clone()); }
            let id = crabref.save(&paper).await?;
            println!("Paper saved, ID: {}", id);
        }
        Commands::Fetch { id } => {
            match crabref.fetch_and_cache(id).await? {
                Some((db_id, paper)) => {
                    println!("Fetched and saved. DB ID: {}", db_id);
                    print_paper_detail(&paper);
                }
                None => println!("Paper not found"),
            }
        }
        Commands::List { limit, sort } => {
            let sort_by = match sort.as_str() {
                "citation" => SortBy::Citation,
                "title" => SortBy::Title,
                _ => SortBy::Created,
            };
            let papers = crabref.list(*limit, sort_by).await?;
            println!("{} papers in database:", papers.len());
            for paper in &papers {
                println!("\nID: {}", paper.id.unwrap_or(-1));
                println!("Title: {}", paper.title);
                println!("arXiv: {}", paper.arxiv_id.as_deref().unwrap_or("N/A"));
                println!("SS: {}", paper.semantic_scholar_id.as_deref().unwrap_or("N/A"));
                println!("Citations: {}", paper.citation_count);
            }
        }
        Commands::DBSearch { query, limit, field } => {
            let papers = match field.as_str() {
                "author" => crabref.search_by_author(query, *limit).await?,
                _ => crabref.search_local(query, *limit).await?,
            };
            println!("Found {} papers:", papers.len());
            for paper in &papers {
                println!("\nID: {}", paper.id.unwrap_or(-1));
                println!("Title: {}", paper.title);
                println!("Citations: {}", paper.citation_count);
            }
        }
        Commands::Recent { days } => {
            let papers = crabref.database().recent_papers(*days).await?;
            println!("Papers from last {} days: {}", days, papers.len());
            for paper in &papers {
                println!("\nID: {}", paper.id.unwrap_or(-1));
                println!("Title: {}", paper.title);
                println!("Added: {}", paper.created_at);
            }
        }
        Commands::Delete { id } => {
            let ok = crabref.delete(*id).await?;
            if ok { println!("Paper {} deleted", id); }
            else { println!("Paper {} not found", id); }
        }
        Commands::Update { id } => {
            let ok = crabref.update(*id).await?;
            if ok { println!("Paper {} updated", id); }
            else { println!("Paper {} could not be updated", id); }
        }
        Commands::Export { format, output, query } => {
            let papers = if let Some(q) = query {
                crabref.search_local(q, 1000).await?
            } else {
                crabref.list(1000, SortBy::Created).await?
            };

            let fmt: OutputFormat = format.parse()
                .map_err(|e: String| anyhow::anyhow!("{}", e))?;
            let content = crabref.export(&papers, fmt);

            if let Some(path) = output {
                std::fs::write(path, &content)?;
                println!("Exported {} papers to {}", papers.len(), path.display());
            } else {
                println!("{}", content);
            }
        }
        Commands::CitationGraph { format, output } => {
            let graph = crabref.build_citation_graph().await?;
            let fmt = match format.as_str() {
                "json" => GraphFormat::Json,
                _ => GraphFormat::Dot,
            };
            let content = crabref.export_citation_graph(&graph, fmt)?;

            if let Some(path) = output {
                std::fs::write(path, &content)?;
                println!("Citation graph exported to {}", path.display());
            } else {
                println!("{}", content);
            }
        }
        Commands::CitationStats => {
            let stats = crabref.citation_stats().await?;
            println!("=== Citation Statistics ===");
            println!("Total papers: {}", stats.total_papers);
            println!("Total citation edges: {}", stats.total_citation_edges);
            println!("Average citations: {:.2}", stats.average_citations);
            println!("H-index: {}", stats.h_index);
            println!("Max citations: {}", stats.max_citations);

            if !stats.most_cited_papers.is_empty() {
                println!("\nMost cited papers:");
                for (paper, count) in stats.most_cited_papers.iter().take(5) {
                    println!("  {} (citations: {})", paper.title, count);
                }
            }
        }
        Commands::SyncCitations { batch } => {
            println!("Syncing citation counts...");
            let (updated, failed) = crabref.sync_citations(*batch).await?;
            println!("Sync complete: {} updated, {} failed", updated, failed);
        }
        Commands::Config => {
            let config = crabref.config();
            println!("PDF storage: {}", config.pdf_storage_path.display());
            println!("Citation depth: {}", config.citation_depth);
            println!("API key: {}", config.semantic_scholar_api_key.as_deref().unwrap_or("not set"));
            println!("Proxy: {}", config.proxy.as_deref().unwrap_or("not set"));
        }
        Commands::Sources => {
            println!("Registered paper sources:");
            for kind in crabref.sources().list_sources() {
                if let Some(source) = crabref.sources().get(kind) {
                    let caps = source.capabilities();
                    println!("\n{} ({})", source.name(), kind);
                    println!("  Search: {}", checkmark(caps.search));
                    println!("  Get by ID: {}", checkmark(caps.get_by_id));
                    println!("  Citations: {}", checkmark(caps.citations));
                    println!("  References: {}", checkmark(caps.references));
                    println!("  PDF download: {}", checkmark(caps.pdf_download));
                }
            }
        }
        Commands::PushSchema => {
            crabref.database().push_schema().await?;
            println!("Database schema pushed successfully.");
        }
        Commands::Count => {
            let count = crabref.count().await?;
            println!("Papers in database: {}", count);
        }
    }

    Ok(())
}

async fn cache_papers(crabref: &CrabRef, papers: &[Paper]) {
    for paper in papers {
        match crabref.save(paper).await {
            Ok(id) => tracing::debug!("cached paper ID: {}", id),
            Err(_) => { /* ignore duplicates */ }
        }
    }
}

fn checkmark(b: bool) -> &'static str {
    if b { "✓" } else { "✗" }
}

fn print_papers(papers: &[Paper]) {
    println!("Found {} papers:", papers.len());
    for paper in papers {
        println!("\n---");
        println!("Title: {}", paper.title);
        if let Some(aid) = &paper.arxiv_id {
            if !aid.is_empty() { println!("arXiv ID: {}", aid); }
        }
        if let Some(sid) = &paper.semantic_scholar_id {
            if !sid.is_empty() { println!("SS ID: {}", sid); }
        }
        print_author_list(&paper.authors);
        println!("Citations: {}", paper.citation_count);
        println!("PDF: {}", paper.pdf_url.as_deref().unwrap_or("N/A"));
    }
}

fn print_paper_detail(paper: &Paper) {
    println!("Title: {}", paper.title);
    if let Some(aid) = &paper.arxiv_id { println!("arXiv ID: {}", aid); }
    if let Some(sid) = &paper.semantic_scholar_id { println!("SS ID: {}", sid); }
    if let Some(doi) = &paper.doi { println!("DOI: {}", doi); }
    print_author_list(&paper.authors);
    println!("Citations: {}", paper.citation_count);
    println!("PDF: {}", paper.pdf_url.as_deref().unwrap_or("N/A"));
    if let Some(abs) = &paper.abstract_text {
        println!("\nAbstract:\n{}", abs);
    }
}

fn print_author_list(authors: &[Author]) {
    if !authors.is_empty() {
        let names: Vec<&str> = authors.iter().map(|a| a.name.as_str()).collect();
        println!("Authors: {}", names.join(", "));
    }
}