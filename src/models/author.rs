use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub id: Option<i64>,
    pub name: String,
    pub semantic_scholar_id: Option<String>,
    #[serde(default)]
    pub orcid: Option<String>,
}

impl Author {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            semantic_scholar_id: None,
            orcid: None,
        }
    }

    pub fn with_semantic_scholar_id(mut self, ss_id: String) -> Self {
        self.semantic_scholar_id = Some(ss_id);
        self
    }

    pub fn with_orcid(mut self, orcid: String) -> Self {
        self.orcid = Some(orcid);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperAuthor {
    pub paper_id: i64,
    pub author_id: i64,
    pub author_order: i32,
}

impl PaperAuthor {
    pub fn new(paper_id: i64, author_id: i64, order: i32) -> Self {
        Self {
            paper_id,
            author_id,
            author_order: order,
        }
    }
}