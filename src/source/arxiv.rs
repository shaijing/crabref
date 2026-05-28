//! arXiv 论文源实现

use crate::models::{Author, Paper};
use crate::source::*;
use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;

const ARXIV_API_URL: &str = "http://export.arxiv.org/api/query";

/// arXiv 论文源
pub struct ArxivSource {
    client: reqwest::Client,
    config: SourceConfig,
}

impl ArxivSource {
    pub fn new(config: SourceConfig) -> Result<Self, SourceError> {
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

        Ok(Self { client, config })
    }

    fn build_search_url(&self, params: &SearchParams) -> String {
        let sort_by = params.sort_by.as_deref().unwrap_or("relevance");
        let sort_order = params.sort_order.as_deref().unwrap_or("descending");

        format!(
            "{}?search_query={}&start={}&max_results={}&sortBy={}&sortOrder={}",
            ARXIV_API_URL,
            Self::url_encode(&params.query),
            params.offset,
            params.limit,
            sort_by,
            sort_order
        )
    }

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

    async fn fetch_with_retry(&self, url: &str) -> Result<String, SourceError> {
        let mut last_error = None;

        for _ in 0..self.config.max_retries {
            let response = self.client.get(url).send().await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    return resp.text().await.map_err(|e| SourceError::Network(e.to_string()));
                }
                Ok(resp) => {
                    let status = resp.status();
                    if status.as_u16() == 429 {
                        sleep(Duration::from_millis(self.config.retry_delay)).await;
                        last_error = Some(SourceError::RateLimit { retry_after: None });
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

    fn parse_response(&self, xml: &str) -> Result<Vec<Paper>, SourceError> {
        let mut papers = Vec::new();

        for entry in xml.split("<entry>").skip(1) {
            if let Some(end) = entry.find("</entry>") {
                let entry_xml = &entry[..end];
                if let Some(paper) = self.parse_entry(entry_xml) {
                    papers.push(paper);
                }
            }
        }

        Ok(papers)
    }

    fn parse_entry(&self, xml: &str) -> Option<Paper> {
        let title = self.extract_tag(xml, "title")?;
        let summary = self.extract_tag(xml, "summary").unwrap_or_default();
        let id = self.extract_tag(xml, "id")?;
        let doi = self.extract_tag(xml, "arxiv:doi");

        let arxiv_id = id
            .strip_prefix("http://arxiv.org/abs/")
            .or_else(|| id.strip_prefix("https://arxiv.org/abs/"))
            .map(|s| s.to_string());

        let pdf_url = self.extract_pdf_link(xml);
        let authors = self.extract_authors(xml);

        Some(Paper::new(title.trim().to_string())
            .with_abstract(summary.trim().to_string())
            .with_arxiv_id(arxiv_id.unwrap_or_default())
            .with_doi(doi.unwrap_or_default())
            .with_pdf_url(pdf_url.unwrap_or_default())
            .with_authors(authors))
    }

    fn extract_authors(&self, xml: &str) -> Vec<crate::Author> {
        let mut authors = Vec::new();
        let mut pos = 0;

        while let Some(start) = xml[pos..].find("<author>") {
            let author_start = pos + start + 8; // "<author>".len()
            if let Some(end) = xml[author_start..].find("</author>") {
                let author_xml = &xml[author_start..author_start + end];
                if let Some(name) = self.extract_tag(author_xml, "name") {
                    authors.push(crate::Author::new(name.trim().to_string()));
                }
                pos = author_start + end + 9; // "</author>".len()
            } else {
                break;
            }
        }

        authors
    }

    fn extract_tag(&self, xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let start = xml.find(&start_tag)?;
        let content_start = start + start_tag.len();
        let end = xml[content_start..].find(&end_tag)?;

        Some(xml[content_start..content_start + end].to_string())
    }

    fn extract_pdf_link(&self, xml: &str) -> Option<String> {
        for link in xml.split("<link ") {
            if link.contains("title=\"pdf\"") || link.contains("type=\"application/pdf\"") {
                if let Some(href_start) = link.find("href=\"") {
                    let rest = &link[href_start + 6..];
                    if let Some(end) = rest.find("\"") {
                        return Some(rest[..end].to_string());
                    }
                }
            }
        }
        None
    }
}

#[async_trait]
impl PaperSource for ArxivSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Arxiv
    }

    fn name(&self) -> &str {
        "arXiv"
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            search: true,
            get_by_id: true,
            citations: false,
            references: false,
            authors: false,
            pdf_download: true,
        }
    }

    async fn search(&self, params: &SearchParams) -> Result<SearchResult, SourceError> {
        let url = self.build_search_url(params);
        let xml = self.fetch_with_retry(&url).await?;
        let papers = self.parse_response(&xml)?;

        Ok(SearchResult {
            total: Some(papers.len()),
            has_more: papers.len() == params.limit,
            papers,
        })
    }

    async fn get_by_identifier(&self, identifier: &str) -> Result<Option<Paper>, SourceError> {
        let id = match Identifier::parse(identifier) {
            Identifier::Arxiv(id) => id,
            Identifier::Url(url) => {
                // 从 URL 提取 arXiv ID
                url.strip_prefix("https://arxiv.org/abs/")
                    .or_else(|| url.strip_prefix("http://arxiv.org/abs/"))
                    .map(|s| s.to_string())
                    .unwrap_or(url)
            }
            _ => identifier.to_string(),
        };

        let params = SearchParams {
            query: format!("id:{}", id),
            limit: 1,
            ..Default::default()
        };

        let result = self.search(&params).await?;
        Ok(result.papers.into_iter().next())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<Paper>, SourceError> {
        self.get_by_identifier(id).await
    }

    async fn get_citations(&self, _paper_id: &str, _limit: usize) -> Result<Vec<(Paper, Vec<Author>)>, SourceError> {
        Err(SourceError::Other("arXiv 不支持获取引用关系".into()))
    }

    async fn get_references(&self, _paper_id: &str, _limit: usize) -> Result<Vec<(Paper, Vec<Author>)>, SourceError> {
        Err(SourceError::Other("arXiv 不支持获取参考文献".into()))
    }

    async fn health_check(&self) -> Result<bool, SourceError> {
        let url = format!("{}?search_query=test&max_results=1", ARXIV_API_URL);
        self.fetch_with_retry(&url).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_source() -> ArxivSource {
        ArxivSource::new(SourceConfig::default()).unwrap()
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(ArxivSource::url_encode("hello"), "hello");
        assert_eq!(ArxivSource::url_encode("hello world"), "hello%20world");
        assert_eq!(ArxivSource::url_encode("machine+learning"), "machine%2Blearning");
        assert_eq!(ArxivSource::url_encode("test-123"), "test-123");
        assert_eq!(ArxivSource::url_encode("test_abc"), "test_abc");
    }

    #[test]
    fn test_build_search_url() {
        let source = create_test_source();
        let params = SearchParams {
            query: "machine learning".to_string(),
            offset: 0,
            limit: 10,
            sort_by: Some("relevance".to_string()),
            sort_order: Some("descending".to_string()),
        };

        let url = source.build_search_url(&params);
        assert!(url.contains("search_query=machine%20learning"));
        assert!(url.contains("start=0"));
        assert!(url.contains("max_results=10"));
        assert!(url.contains("sortBy=relevance"));
        assert!(url.contains("sortOrder=descending"));
    }

    #[test]
    fn test_parse_empty_response() {
        let source = create_test_source();
        let xml = "";
        let papers = source.parse_response(xml).unwrap();
        assert_eq!(papers.len(), 0);
    }

    #[test]
    fn test_parse_single_entry() {
        let source = create_test_source();
        let xml = r#"
<entry>
    <title>Test Paper Title</title>
    <summary>This is a test abstract.</summary>
    <id>http://arxiv.org/abs/2301.00001</id>
    <link href="https://arxiv.org/pdf/2301.00001" title="pdf"/>
    <arxiv:doi>10.1234/test</arxiv:doi>
</entry>
"#;
        let papers = source.parse_response(xml).unwrap();
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0].title, "Test Paper Title");
        assert_eq!(papers[0].abstract_text, Some("This is a test abstract.".to_string()));
        assert_eq!(papers[0].arxiv_id, Some("2301.00001".to_string()));
        assert_eq!(papers[0].pdf_url, Some("https://arxiv.org/pdf/2301.00001".to_string()));
    }

    #[test]
    fn test_parse_multiple_entries() {
        let source = create_test_source();
        let xml = r#"
<entry>
    <title>Paper 1</title>
    <id>http://arxiv.org/abs/2301.00001</id>
</entry>
<entry>
    <title>Paper 2</title>
    <id>http://arxiv.org/abs/2301.00002</id>
</entry>
"#;
        let papers = source.parse_response(xml).unwrap();
        assert_eq!(papers.len(), 2);
        assert_eq!(papers[0].title, "Paper 1");
        assert_eq!(papers[1].title, "Paper 2");
    }

    #[test]
    fn test_extract_tag() {
        let source = create_test_source();
        let xml = "<title>Test Title</title>";
        assert_eq!(source.extract_tag(xml, "title"), Some("Test Title".to_string()));
        assert_eq!(source.extract_tag(xml, "summary"), None);
    }

    #[test]
    fn test_extract_pdf_link() {
        let source = create_test_source();
        let xml1 = r#"<link href="https://arxiv.org/pdf/2301.00001" title="pdf"/>"#;
        assert_eq!(source.extract_pdf_link(xml1), Some("https://arxiv.org/pdf/2301.00001".to_string()));

        let xml2 = r#"<link href="https://arxiv.org/pdf/2301.00002" type="application/pdf"/>"#;
        assert_eq!(source.extract_pdf_link(xml2), Some("https://arxiv.org/pdf/2301.00002".to_string()));

        let xml3 = r#"<link href="https://arxiv.org/abs/2301.00003"/>"#;
        assert_eq!(source.extract_pdf_link(xml3), None);
    }

    #[test]
    fn test_capabilities() {
        let source = create_test_source();
        let caps = source.capabilities();
        assert!(caps.search);
        assert!(caps.get_by_id);
        assert!(!caps.citations);
        assert!(!caps.references);
        assert!(caps.pdf_download);
    }

    #[test]
    fn test_source_kind() {
        let source = create_test_source();
        assert_eq!(source.kind(), SourceKind::Arxiv);
        assert_eq!(source.name(), "arXiv");
    }

    #[test]
    fn test_new_with_proxy() {
        let config = SourceConfig {
            proxy: Some("http://127.0.0.1:7890".to_string()),
            ..Default::default()
        };
        let source = ArxivSource::new(config);
        assert!(source.is_ok());
    }
}