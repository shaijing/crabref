//! Example: Using the crabref database module with PostgreSQL
//!
//! Prerequisites:
//! 1. A running PostgreSQL server
//! 2. DATABASE_URL env var set, e.g. "postgres://localhost/crabref"
//! 3. Or pass the URL directly to `Database::new()`

use crabref::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Connect to PostgreSQL
    let db = Database::from_env().await?;

    // Push schema (create tables if they don't exist)
    db.push_schema().await?;
    println!("Schema pushed successfully");

    // Count existing papers
    let count = db.count_papers().await?;
    println!("Papers in database: {}", count);

    // List recent papers
    let papers = db.list_papers(10).await?;
    for paper in &papers {
        println!(
            "  [{}] {} (citations: {})",
            paper.id.unwrap_or(-1),
            paper.title,
            paper.citation_count
        );
    }

    // Search papers by title prefix
    let results = db.search_papers("Machine", 5).await?;
    println!("\nPapers matching 'Machine': {}", results.len());

    // Search by author
    let results = db.search_by_author("Smith", 5).await?;
    println!("Papers by author 'Smith': {}", results.len());

    // Recent papers from last 7 days
    let recent = db.recent_papers(7).await?;
    println!("Papers from last 7 days: {}", recent.len());

    // Top cited papers
    let top = db.top_cited_papers(5).await?;
    println!("\nTop cited papers:");
    for paper in &top {
        println!(
            "  {} (citations: {})",
            paper.title,
            paper.citation_count
        );
    }

    Ok(())
}