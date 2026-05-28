//! Database operations using toasty ORM with PostgreSQL

use anyhow::Result;
use toasty::Db;

use crate::models::{Author, Paper};

// toasty model for Paper
#[derive(Debug, toasty::Model)]
struct PaperModel {
    #[key]
    #[auto]
    id: i64,
    title: String,
    abstract_text: Option<String>,
    #[index]
    arxiv_id: Option<String>,
    #[index]
    semantic_scholar_id: Option<String>,
    doi: Option<String>,
    pdf_url: Option<String>,
    local_pdf_path: Option<String>,
    publication_date: Option<String>,
    venue: Option<String>,
    citation_count: i64,
    created_at: String,
    updated_at: String,
}

// toasty model for Author
#[derive(Debug, toasty::Model)]
struct AuthorModel {
    #[key]
    #[auto]
    id: i64,
    #[index]
    name: String,
    #[index]
    semantic_scholar_id: Option<String>,
}

// toasty model for Citation
#[derive(Debug, toasty::Model)]
struct CitationModel {
    #[key]
    citing_paper_id: i64,
    #[key]
    cited_paper_id: i64,
}

// toasty model for PaperAuthor junction
#[derive(Debug, toasty::Model)]
struct PaperAuthorModel {
    #[key]
    paper_id: i64,
    #[key]
    author_id: i64,
    author_order: i32,
}

fn model_to_paper(model: PaperModel, authors: Vec<Author>) -> Paper {
    Paper {
        id: Some(model.id),
        title: model.title,
        abstract_text: model.abstract_text,
        arxiv_id: model.arxiv_id,
        semantic_scholar_id: model.semantic_scholar_id,
        doi: model.doi,
        pdf_url: model.pdf_url,
        local_pdf_path: model.local_pdf_path,
        publication_date: model.publication_date,
        venue: model.venue,
        citation_count: model.citation_count,
        authors,
        created_at: chrono::DateTime::parse_from_rfc3339(&model.created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&model.updated_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    }
}

pub struct Database {
    db: Db,
}

impl Database {
    /// Create a new database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        let db = Db::builder()
            .models(toasty::models!(PaperModel, AuthorModel, CitationModel, PaperAuthorModel))
            .connect(database_url)
            .await?;

        Ok(Self { db })
    }

    /// Create from environment DATABASE_URL
    pub async fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/crabref".to_string());
        Self::new(&database_url).await
    }

    /// Ensure schema is pushed to the database
    pub async fn push_schema(&self) -> Result<()> {
        self.db.push_schema().await?;
        Ok(())
    }

    // Paper CRUD

    pub async fn insert_paper(&self, paper: &Paper) -> Result<i64> {
        let mut db = self.db.clone();

        let model = toasty::create!(PaperModel {
            title: paper.title.clone(),
            abstract_text: paper.abstract_text.clone(),
            arxiv_id: paper.arxiv_id.clone(),
            semantic_scholar_id: paper.semantic_scholar_id.clone(),
            doi: paper.doi.clone(),
            pdf_url: paper.pdf_url.clone(),
            local_pdf_path: paper.local_pdf_path.clone(),
            publication_date: paper.publication_date.clone(),
            venue: paper.venue.clone(),
            citation_count: paper.citation_count,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
        .exec(&mut db)
        .await?;

        let paper_id = model.id;

        // Insert authors and link them
        for (order, author) in paper.authors.iter().enumerate() {
            let author_id = self.insert_author(author).await?;
            self.link_paper_author(paper_id, author_id, order as i32).await?;
        }

        Ok(paper_id)
    }

    pub async fn get_paper_by_id(&self, id: i64) -> Result<Option<Paper>> {
        let mut db = self.db.clone();
        let model = PaperModel::get_by_id(&mut db, &id).await?;

        let authors = self.get_paper_authors(model.id).await?;
        Ok(Some(model_to_paper(model, authors)))
    }

    pub async fn get_paper_by_arxiv_id(&self, arxiv_id: &str) -> Result<Option<Paper>> {
        let mut db = self.db.clone();
        let model = PaperModel::filter_by_arxiv_id(arxiv_id.to_string())
            .first()
            .exec(&mut db)
            .await?;

        match model {
            Some(m) => {
                let authors = self.get_paper_authors(m.id).await?;
                Ok(Some(model_to_paper(m, authors)))
            }
            None => Ok(None),
        }
    }

    pub async fn get_paper_by_semantic_scholar_id(&self, ss_id: &str) -> Result<Option<Paper>> {
        let mut db = self.db.clone();
        let model = PaperModel::filter_by_semantic_scholar_id(ss_id.to_string())
            .first()
            .exec(&mut db)
            .await?;

        match model {
            Some(m) => {
                let authors = self.get_paper_authors(m.id).await?;
                Ok(Some(model_to_paper(m, authors)))
            }
            None => Ok(None),
        }
    }

    pub async fn update_paper(&self, paper: &Paper) -> Result<()> {
        let id = paper.id.ok_or_else(|| anyhow::anyhow!("Paper id required"))?;
        let mut db = self.db.clone();
        let now = chrono::Utc::now().to_rfc3339();

        PaperModel::filter_by_id(id)
            .update()
            .title(paper.title.clone())
            .abstract_text(paper.abstract_text.clone())
            .arxiv_id(paper.arxiv_id.clone())
            .semantic_scholar_id(paper.semantic_scholar_id.clone())
            .doi(paper.doi.clone())
            .pdf_url(paper.pdf_url.clone())
            .local_pdf_path(paper.local_pdf_path.clone())
            .publication_date(paper.publication_date.clone())
            .venue(paper.venue.clone())
            .citation_count(paper.citation_count)
            .updated_at(now)
            .exec(&mut db)
            .await?;

        Ok(())
    }

    pub async fn list_papers(&self, limit: i64) -> Result<Vec<Paper>> {
        let mut db = self.db.clone();
        let models = PaperModel::all()
            .latest_by(PaperModel::fields().created_at())
            .limit(limit as usize)
            .exec(&mut db)
            .await?;

        let mut papers = Vec::new();
        for model in models {
            let authors = self.get_paper_authors(model.id).await?;
            papers.push(model_to_paper(model, authors));
        }

        Ok(papers)
    }

    // Author CRUD

    pub async fn insert_author(&self, author: &Author) -> Result<i64> {
        // Check if author exists by semantic_scholar_id
        if let Some(ss_id) = &author.semantic_scholar_id {
            if !ss_id.is_empty() {
                let mut db = self.db.clone();
                let existing = AuthorModel::filter_by_semantic_scholar_id(ss_id.clone())
                    .first()
                    .exec(&mut db)
                    .await?;

                if let Some(m) = existing {
                    return Ok(m.id);
                }
            }
        }

        let mut db = self.db.clone();
        let model = toasty::create!(AuthorModel {
            name: author.name.clone(),
            semantic_scholar_id: author.semantic_scholar_id.clone(),
        })
        .exec(&mut db)
        .await?;

        Ok(model.id)
    }

    pub async fn link_paper_author(&self, paper_id: i64, author_id: i64, order: i32) -> Result<()> {
        let mut db = self.db.clone();
        toasty::create!(PaperAuthorModel {
            paper_id,
            author_id,
            author_order: order,
        })
        .exec(&mut db)
        .await?;

        Ok(())
    }

    pub async fn get_paper_authors(&self, paper_id: i64) -> Result<Vec<Author>> {
        let mut db = self.db.clone();
        // PaperAuthorModel has composite key (paper_id, author_id), use filter expression
        let links = PaperAuthorModel::filter(
            PaperAuthorModel::fields().paper_id().eq(paper_id),
        )
        .exec(&mut db)
        .await?;

        let mut authors = Vec::new();
        for link in links {
            let author_model = AuthorModel::get_by_id(&mut db, &link.author_id).await?;
            authors.push(Author {
                id: Some(author_model.id),
                name: author_model.name,
                semantic_scholar_id: author_model.semantic_scholar_id,
                orcid: None,
            });
        }

        Ok(authors)
    }

    // Citation operations

    pub async fn insert_citation(&self, citation: &crate::models::Citation) -> Result<()> {
        let mut db = self.db.clone();
        toasty::create!(CitationModel {
            citing_paper_id: citation.citing_paper_id,
            cited_paper_id: citation.cited_paper_id,
        })
        .exec(&mut db)
        .await?;

        Ok(())
    }

    pub async fn get_citations(&self, paper_id: i64) -> Result<Vec<i64>> {
        let mut db = self.db.clone();
        // CitationModel has composite key, use filter expression
        let citations = CitationModel::filter(
            CitationModel::fields().citing_paper_id().eq(paper_id),
        )
        .exec(&mut db)
        .await?;

        Ok(citations.into_iter().map(|c| c.cited_paper_id).collect())
    }

    pub async fn get_cited_by(&self, paper_id: i64) -> Result<Vec<i64>> {
        let mut db = self.db.clone();
        // CitationModel has composite key, use filter expression
        let citations = CitationModel::filter(
            CitationModel::fields().cited_paper_id().eq(paper_id),
        )
        .exec(&mut db)
        .await?;

        Ok(citations.into_iter().map(|c| c.citing_paper_id).collect())
    }

    /// Search papers by title (prefix match)
    pub async fn search_papers(&self, query: &str, limit: i64) -> Result<Vec<Paper>> {
        let mut db = self.db.clone();
        let models = PaperModel::filter(PaperModel::fields().title().starts_with(query.to_string()))
            .latest_by(PaperModel::fields().citation_count())
            .limit(limit as usize)
            .exec(&mut db)
            .await?;

        let mut papers = Vec::new();
        for model in models {
            let authors = self.get_paper_authors(model.id).await?;
            papers.push(model_to_paper(model, authors));
        }

        Ok(papers)
    }

    /// Delete a paper
    pub async fn delete_paper(&self, id: i64) -> Result<bool> {
        let mut db = self.db.clone();

        // Delete paper_author links
        let links = PaperAuthorModel::filter(
            PaperAuthorModel::fields().paper_id().eq(id),
        )
        .exec(&mut db)
        .await?;
        for link in links {
            link.delete().exec(&mut db).await?;
        }

        // Delete citation links where this paper cites others
        let citing = CitationModel::filter(
            CitationModel::fields().citing_paper_id().eq(id),
        )
        .exec(&mut db)
        .await?;
        for c in citing {
            c.delete().exec(&mut db).await?;
        }

        // Delete citation links where others cite this paper
        let cited = CitationModel::filter(
            CitationModel::fields().cited_paper_id().eq(id),
        )
        .exec(&mut db)
        .await?;
        for c in cited {
            c.delete().exec(&mut db).await?;
        }

        // Delete the paper itself
        let model = PaperModel::get_by_id(&mut db, &id).await?;
        model.delete().exec(&mut db).await?;
        Ok(true)
    }

    /// Get total paper count
    pub async fn count_papers(&self) -> Result<i64> {
        let mut db = self.db.clone();
        let count = PaperModel::all().count().exec(&mut db).await?;
        Ok(count as i64)
    }

    /// Get top cited papers
    pub async fn top_cited_papers(&self, limit: i64) -> Result<Vec<Paper>> {
        let mut db = self.db.clone();
        let models = PaperModel::all()
            .latest_by(PaperModel::fields().citation_count())
            .limit(limit as usize)
            .exec(&mut db)
            .await?;

        let mut papers = Vec::new();
        for model in models {
            let authors = self.get_paper_authors(model.id).await?;
            papers.push(model_to_paper(model, authors));
        }

        Ok(papers)
    }

    /// Get papers added in the last N days
    pub async fn recent_papers(&self, days: i64) -> Result<Vec<Paper>> {
        let mut db = self.db.clone();
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();

        let models = PaperModel::filter(
            PaperModel::fields().created_at().ge(cutoff_str),
        )
        .latest_by(PaperModel::fields().created_at())
        .exec(&mut db)
        .await?;

        let mut papers = Vec::new();
        for model in models {
            let authors = self.get_paper_authors(model.id).await?;
            papers.push(model_to_paper(model, authors));
        }

        Ok(papers)
    }

    /// Search papers by author name (prefix match)
    pub async fn search_by_author(&self, name: &str, limit: i64) -> Result<Vec<Paper>> {
        let mut db = self.db.clone();

        // Find authors matching the name using prefix match
        let matching_authors = AuthorModel::filter(
            AuthorModel::fields().name().starts_with(name.to_string()),
        )
        .exec(&mut db)
        .await?;

        let author_ids: Vec<i64> = matching_authors.iter().map(|a| a.id).collect();

        if author_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Find paper-author links for these authors
        let mut all_paper_ids = std::collections::HashSet::new();
        for author_id in &author_ids {
            let links = PaperAuthorModel::filter(
                PaperAuthorModel::fields().author_id().eq(*author_id),
            )
            .exec(&mut db)
            .await?;

            for link in links {
                all_paper_ids.insert(link.paper_id);
            }
        }

        if all_paper_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch each paper by ID
        let mut papers = Vec::new();
        for paper_id in all_paper_ids.into_iter().take(limit as usize) {
            if let Ok(model) = PaperModel::get_by_id(&mut db, &paper_id).await {
                let authors = self.get_paper_authors(model.id).await?;
                papers.push(model_to_paper(model, authors));
            }
        }

        Ok(papers)
    }

    /// Get database for advanced operations
    pub fn db(&self) -> &Db {
        &self.db
    }
}