use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::Author;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: Option<i64>,
    pub title: String,
    pub abstract_text: Option<String>,
    pub arxiv_id: Option<String>,
    pub semantic_scholar_id: Option<String>,
    pub doi: Option<String>,
    pub pdf_url: Option<String>,
    pub local_pdf_path: Option<String>,
    pub publication_date: Option<String>,
    pub venue: Option<String>,
    pub citation_count: i64,
    pub authors: Vec<Author>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Paper {
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            title,
            abstract_text: None,
            arxiv_id: None,
            semantic_scholar_id: None,
            doi: None,
            pdf_url: None,
            local_pdf_path: None,
            publication_date: None,
            venue: None,
            citation_count: 0,
            authors: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_arxiv_id(mut self, arxiv_id: String) -> Self {
        self.arxiv_id = Some(arxiv_id);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_semantic_scholar_id(mut self, ss_id: String) -> Self {
        self.semantic_scholar_id = Some(ss_id);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_doi(mut self, doi: String) -> Self {
        self.doi = Some(doi);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_abstract(mut self, abstract_text: String) -> Self {
        self.abstract_text = Some(abstract_text);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_pdf_url(mut self, pdf_url: String) -> Self {
        self.pdf_url = Some(pdf_url);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_local_pdf(mut self, path: String) -> Self {
        self.local_pdf_path = Some(path);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_citation_count(mut self, count: i64) -> Self {
        self.citation_count = count;
        self.updated_at = Utc::now();
        self
    }

    pub fn with_venue(mut self, venue: String) -> Self {
        self.venue = Some(venue);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_publication_date(mut self, date: String) -> Self {
        self.publication_date = Some(date);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_authors(mut self, authors: Vec<Author>) -> Self {
        self.authors = authors;
        self.updated_at = Utc::now();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub citing_paper_id: i64,
    pub cited_paper_id: i64,
}

impl Citation {
    pub fn new(citing: i64, cited: i64) -> Self {
        Self {
            citing_paper_id: citing,
            cited_paper_id: cited,
        }
    }
}