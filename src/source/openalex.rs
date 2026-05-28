//! OpenAlex 论文源实现

use crate::models::{Author, Paper};
use crate::source::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

const OPENALEX_API_URL: &str = "https://api.openalex.org";

// API 响应结构
#[derive(Debug, Deserialize)]
struct OaSearchResponse {
    meta: OaMeta,
    results: Vec<OaWork>,
}

#[derive(Debug, Deserialize)]
struct OaMeta {
    count: i64,
    #[serde(default)]
    per_page: i64,
}

#[derive(Debug, Deserialize)]
struct OaWork {
    id: String,
    #[serde(default)]
    doi: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    publication_year: Option<i64>,
    #[serde(default)]
    publication_date: Option<String>,
    #[serde(default)]
    cited_by_count: i64,
    #[serde(default)]
    authorships: Vec<OaAuthorship>,
    #[serde(default)]
    abstract_inverted_index: Option<HashMap<String, Vec<i32>>>,
    #[serde(default)]
    primary_location: Option<OaLocation>,
    #[serde(default)]
    open_access: Option<OaOpenAccess>,
    #[serde(default, rename = "type")]
    work_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OaAuthorship {
    author: OaAuthorInfo,
}

#[derive(Debug, Deserialize)]
struct OaAuthorInfo {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    orcid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OaLocation {
    #[serde(default)]
    landing_page_url: Option<String>,
    #[serde(default)]
    pdf_url: Option<String>,
    #[serde(default)]
    source: Option<OaSource>,
}

#[derive(Debug, Deserialize)]
struct OaSource {
    display_name: Option<String>,
    #[serde(default)]
    issn: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OaOpenAccess {
    #[serde(default)]
    is_oa: bool,
    #[serde(default)]
    oa_url: Option<String>,
}

/// OpenAlex 论文源
pub struct OpenAlexSource {
    client: reqwest::Client,
    config: SourceConfig,
    email: Option<String>,
}

impl OpenAlexSource {
    pub fn new(config: SourceConfig) -> Result<Self, SourceError> {
        Self::with_email(config, None)
    }

    pub fn with_email(config: SourceConfig, email: Option<String>) -> Result<Self, SourceError> {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout));

        if let Some(proxy_url) = &config.proxy {
            if !proxy_url.is_empty() {
                let proxy = reqwest::Proxy::all(proxy_url)
                    .map_err(|e| SourceError::Network(e.to_string()))?;
                builder = builder.proxy(proxy);
            }
        }

        let client = builder.build()
            .map_err(|e| SourceError::Network(e.to_string()))?;

        Ok(Self { client, config, email })
    }

    fn build_url(&self, endpoint: &str, params: &[(&str, &str)]) -> String {
        let mut url = format!("{}{}", OPENALEX_API_URL, endpoint);

        // Add email for faster rate limits
        let email_param = if let Some(ref email) = self.email {
            Some(("mailto", email.as_str()))
        } else {
            None
        };

        let all_params: Vec<(&str, &str)> = email_param
            .into_iter()
            .chain(params.iter().copied())
            .collect();

        if !all_params.is_empty() {
            url.push('?');
            let query: Vec<String> = all_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            url.push_str(&query.join("&"));
        }

        url
    }

    async fn fetch_with_retry<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, SourceError> {
        let mut last_error = None;

        for _ in 0..self.config.max_retries {
            let response = self.client
                .get(url)
                .header("User-Agent", "Crabref/1.0")
                .header("Accept", "application/json")
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    // Get raw text first for debugging
                    let text = resp.text().await.map_err(|e| SourceError::Parse(e.to_string()))?;
                    return serde_json::from_str::<T>(&text).map_err(|e| {
                        SourceError::Parse(format!("{} (response: {}...)", e, &text[..text.len().min(200)]))
                    });
                }
                Ok(resp) => {
                    let status = resp.status();
                    if status.as_u16() == 429 {
                        sleep(Duration::from_millis(self.config.retry_delay)).await;
                        last_error = Some(SourceError::RateLimit { retry_after: None });
                    } else if status.as_u16() == 404 {
                        return Err(SourceError::NotFound);
                    } else {
                        last_error = Some(SourceError::Network(format!("HTTP {}", status)));
                    }
                }
                Err(e) => {
                    last_error = Some(SourceError::Network(e.to_string()));
                }
            }

            sleep(Duration::from_millis(self.config.retry_delay)).await;
        }

        Err(last_error.unwrap_or(SourceError::Network("Max retries exceeded".into())))
    }

    fn extract_work_id(id: &str) -> String {
        // Extract the ID from URL like https://openalex.org/W2101234009
        id.rsplit('/').next().unwrap_or(id).to_string()
    }

    fn reconstruct_abstract(inverted_index: &HashMap<String, Vec<i32>>) -> Option<String> {
        if inverted_index.is_empty() {
            return None;
        }

        // Find the maximum position
        let max_pos = inverted_index.values()
            .flat_map(|positions| positions.iter())
            .max()
            .copied()
            .unwrap_or(0) as usize;

        // Create a vector of the right size
        let mut words = vec![String::new(); max_pos + 1];

        // Fill in the words
        for (word, positions) in inverted_index {
            for pos in positions {
                if (*pos as usize) < words.len() {
                    words[*pos as usize] = word.clone();
                }
            }
        }

        let abstract_text = words.join(" ");
        if abstract_text.is_empty() {
            None
        } else {
            Some(abstract_text)
        }
    }

    fn oa_to_paper(&self, work: OaWork) -> Paper {
        let title = work.title
            .or(work.display_name)
            .unwrap_or_default();

        let doi = work.doi
            .map(|d| d.replace("https://doi.org/", ""));

        let openalex_id = Self::extract_work_id(&work.id);

        // Get PDF URL
        let pdf_url = work.open_access
            .and_then(|oa| oa.oa_url)
            .or_else(|| work.primary_location.as_ref().and_then(|l| l.pdf_url.clone()));

        // Get venue
        let venue = work.primary_location
            .and_then(|l| l.source.and_then(|s| s.display_name));

        // Reconstruct abstract
        let abstract_text = work.abstract_inverted_index
            .as_ref()
            .and_then(Self::reconstruct_abstract);

        // Convert authors
        let authors: Vec<Author> = work.authorships.into_iter()
            .map(|a| {
                let name = a.author.display_name.unwrap_or_else(|| "Unknown".to_string());
                let mut author = Author::new(name);
                if let Some(author_id) = a.author.id {
                    let id = Self::extract_work_id(&author_id);
                    author = author.with_semantic_scholar_id(id);
                }
                if let Some(orcid) = a.author.orcid {
                    author.orcid = Some(orcid);
                }
                author
            })
            .collect();

        let mut paper = Paper::new(title)
            .with_semantic_scholar_id(openalex_id)
            .with_citation_count(work.cited_by_count)
            .with_authors(authors);

        if let Some(abs) = abstract_text {
            paper = paper.with_abstract(abs);
        }

        if let Some(d) = doi {
            paper = paper.with_doi(d);
        }

        if let Some(pdf) = pdf_url {
            paper = paper.with_pdf_url(pdf);
        }

        if let Some(year) = work.publication_year {
            paper = paper.with_publication_date(year.to_string());
        }

        if let Some(v) = venue {
            paper = paper.with_venue(v);
        }

        paper
    }
}

#[async_trait]
impl PaperSource for OpenAlexSource {
    fn kind(&self) -> SourceKind {
        SourceKind::OpenAlex
    }

    fn name(&self) -> &str {
        "OpenAlex"
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            search: true,
            get_by_id: true,
            citations: false,  // OpenAlex doesn't have direct citations API
            references: false,
            authors: true,
            pdf_download: false,
        }
    }

    async fn search(&self, params: &SearchParams) -> Result<SearchResult, SourceError> {
        let url = self.build_url(
            "/works",
            &[
                ("search", &Self::url_encode(&params.query)),
                ("per_page", &params.limit.to_string()),
                ("page", &(params.offset / params.limit + 1).to_string()),
            ],
        );

        let response: OaSearchResponse = self.fetch_with_retry(&url).await?;
        let papers: Vec<Paper> = response.results.into_iter().map(|w| self.oa_to_paper(w)).collect();

        Ok(SearchResult {
            total: Some(response.meta.count as usize),
            has_more: papers.len() == params.limit,
            papers,
        })
    }

    async fn get_by_identifier(&self, identifier: &str) -> Result<Option<Paper>, SourceError> {
        let id = match Identifier::parse(identifier) {
            Identifier::OpenAlex(id) => id,
            Identifier::Doi(doi) => {
                return self.get_by_doi(&doi).await;
            }
            _ => identifier.to_string(),
        };

        self.get_by_id(&id).await
    }

    async fn get_by_id(&self, work_id: &str) -> Result<Option<Paper>, SourceError> {
        // Ensure the ID has the W prefix
        let id = if work_id.starts_with('W') {
            work_id.to_string()
        } else {
            format!("W{}", work_id)
        };

        let url = self.build_url(&format!("/works/{}", id), &[]);

        match self.fetch_with_retry::<OaWork>(&url).await {
            Ok(work) => Ok(Some(self.oa_to_paper(work))),
            Err(SourceError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn get_citations(&self, _paper_id: &str, _limit: usize) -> Result<Vec<(Paper, Vec<Author>)>, SourceError> {
        // OpenAlex doesn't have a direct citations API
        Err(SourceError::Other("OpenAlex does not support citations API".to_string()))
    }

    async fn get_references(&self, _paper_id: &str, _limit: usize) -> Result<Vec<(Paper, Vec<Author>)>, SourceError> {
        // OpenAlex doesn't have a direct references API
        Err(SourceError::Other("OpenAlex does not support references API".to_string()))
    }

    async fn health_check(&self) -> Result<bool, SourceError> {
        let url = self.build_url("/works?per_page=1", &[]);
        self.fetch_with_retry::<OaSearchResponse>(&url).await?;
        Ok(true)
    }
}

impl OpenAlexSource {
    fn url_encode(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u32)
                }
            })
            .collect()
    }

    async fn get_by_doi(&self, doi: &str) -> Result<Option<Paper>, SourceError> {
        let url = self.build_url(
            "/works",
            &[("filter", &format!("doi:{}", doi))],
        );

        let response: OaSearchResponse = self.fetch_with_retry(&url).await?;

        Ok(response.results.into_iter().next().map(|w| self.oa_to_paper(w)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_source() -> OpenAlexSource {
        OpenAlexSource::new(SourceConfig::default()).unwrap()
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(OpenAlexSource::url_encode("hello"), "hello");
        assert_eq!(OpenAlexSource::url_encode("hello world"), "hello%20world");
        assert_eq!(OpenAlexSource::url_encode("machine learning"), "machine%20learning");
    }

    #[test]
    fn test_extract_work_id() {
        assert_eq!(OpenAlexSource::extract_work_id("https://openalex.org/W2101234009"), "W2101234009");
        assert_eq!(OpenAlexSource::extract_work_id("W2101234009"), "W2101234009");
    }

    #[test]
    fn test_reconstruct_abstract() {
        let mut inverted = HashMap::new();
        inverted.insert("Hello".to_string(), vec![0]);
        inverted.insert("world".to_string(), vec![1]);

        let result = OpenAlexSource::reconstruct_abstract(&inverted);
        assert_eq!(result, Some("Hello world".to_string()));
    }

    #[test]
    fn test_reconstruct_abstract_empty() {
        let inverted = HashMap::new();
        let result = OpenAlexSource::reconstruct_abstract(&inverted);
        assert_eq!(result, None);
    }

    #[test]
    fn test_capabilities() {
        let source = create_test_source();
        let caps = source.capabilities();
        assert!(caps.search);
        assert!(caps.get_by_id);
        assert!(!caps.citations);
        assert!(!caps.references);
        assert!(caps.authors);
        assert!(!caps.pdf_download);
    }

    #[test]
    fn test_source_kind() {
        let source = create_test_source();
        assert_eq!(source.kind(), SourceKind::OpenAlex);
        assert_eq!(source.name(), "OpenAlex");
    }

    #[test]
    fn test_oa_work_deserialize() {
        let json = r#"{
            "id": "https://openalex.org/W2101234009",
            "doi": "https://doi.org/10.1234/test",
            "title": "Test Paper",
            "display_name": "Test Paper Display",
            "publication_year": 2023,
            "cited_by_count": 100,
            "authorships": [
                {
                    "author": {
                        "id": "https://openalex.org/A123",
                        "display_name": "John Doe",
                        "orcid": "https://orcid.org/0000-0000-0000-0000"
                    }
                }
            ]
        }"#;

        let work: OaWork = serde_json::from_str(json).unwrap();
        assert_eq!(work.id, "https://openalex.org/W2101234009");
        assert_eq!(work.title, Some("Test Paper".to_string()));
        assert_eq!(work.cited_by_count, 100);
        assert_eq!(work.authorships.len(), 1);
    }
}