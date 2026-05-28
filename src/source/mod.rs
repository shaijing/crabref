//! 论文源通用接口定义

mod arxiv;
mod manager;
mod openalex;
mod semantic_scholar;

pub use arxiv::*;
pub use manager::*;
pub use openalex::*;
pub use semantic_scholar::*;

use crate::models::{Author, Paper};
use async_trait::async_trait;
use std::fmt;
use thiserror::Error;

/// 论文源错误类型
#[derive(Debug, Error)]
pub enum SourceError {
    /// 网络请求错误
    #[error("网络错误: {0}")]
    Network(String),
    /// 解析错误
    #[error("解析错误: {0}")]
    Parse(String),
    /// 速率限制
    #[error("速率限制，{}", .retry_after.map(|s| format!("请等待 {s} 秒后重试")).unwrap_or_else(|| "请稍后重试".to_string()))]
    RateLimit { retry_after: Option<u64> },
    /// 未找到
    #[error("未找到论文")]
    NotFound,
    /// 其他错误
    #[error("错误: {0}")]
    Other(String),
}

/// 搜索结果
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub papers: Vec<Paper>,
    pub total: Option<usize>,
    pub has_more: bool,
}

impl Default for SearchResult {
    fn default() -> Self {
        Self {
            papers: Vec::new(),
            total: None,
            has_more: false,
        }
    }
}

/// 搜索参数
#[derive(Debug, Clone)]
pub struct SearchParams {
    /// 搜索关键词
    pub query: String,
    /// 起始位置
    pub offset: usize,
    /// 最大结果数
    pub limit: usize,
    /// 排序字段
    pub sort_by: Option<String>,
    /// 排序顺序 (asc/desc)
    pub sort_order: Option<String>,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            query: String::new(),
            offset: 0,
            limit: 10,
            sort_by: None,
            sort_order: None,
        }
    }
}

/// 论文源标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Arxiv,
    SemanticScholar,
    Crossref,
    OpenAlex,
    Pubmed,
    GoogleScholar,
    Custom(&'static str),
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Arxiv => "arxiv",
            Self::SemanticScholar => "semantic_scholar",
            Self::Crossref => "crossref",
            Self::OpenAlex => "openalex",
            Self::Pubmed => "pubmed",
            Self::GoogleScholar => "google_scholar",
            Self::Custom(name) => name,
        }
    }
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 论文源能力标识
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCapabilities {
    /// 支持搜索
    pub search: bool,
    /// 支持通过 ID 获取
    pub get_by_id: bool,
    /// 支持获取引用
    pub citations: bool,
    /// 支持获取参考文献
    pub references: bool,
    /// 支持获取作者
    pub authors: bool,
    /// 支持 PDF 下载
    pub pdf_download: bool,
}

/// 论文源通用接口
#[async_trait]
pub trait PaperSource: Send + Sync {
    /// 论文源标识
    fn kind(&self) -> SourceKind;

    /// 论文源名称
    fn name(&self) -> &str;

    /// 论文源能力
    fn capabilities(&self) -> SourceCapabilities;

    /// 搜索论文
    async fn search(&self, params: &SearchParams) -> Result<SearchResult, SourceError>;

    /// 通过标识符获取论文（如 arxiv:2301.00001, doi:10.xxxx）
    async fn get_by_identifier(&self, identifier: &str) -> Result<Option<Paper>, SourceError>;

    /// 通过内部 ID 获取论文
    async fn get_by_id(&self, id: &str) -> Result<Option<Paper>, SourceError>;

    /// 获取论文的引用（被哪些论文引用）
    async fn get_citations(
        &self,
        paper_id: &str,
        limit: usize,
    ) -> Result<Vec<(Paper, Vec<Author>)>, SourceError>;

    /// 获取论文的参考文献
    async fn get_references(
        &self,
        paper_id: &str,
        limit: usize,
    ) -> Result<Vec<(Paper, Vec<Author>)>, SourceError>;

    /// 检查论文源是否可用
    async fn health_check(&self) -> Result<bool, SourceError>;
}

/// 论文源配置
#[derive(Debug, Clone)]
pub struct SourceConfig {
    /// API Key
    pub api_key: Option<String>,
    /// 代理地址
    pub proxy: Option<String>,
    /// 请求超时（秒）
    pub timeout: u64,
    /// 最大重试次数
    pub max_retries: usize,
    /// 重试延迟（毫秒）
    pub retry_delay: u64,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            proxy: None,
            timeout: 30,
            max_retries: 3,
            retry_delay: 1000,
        }
    }
}

/// 标识符解析
#[derive(Debug, Clone)]
pub enum Identifier {
    Arxiv(String),
    Doi(String),
    SemanticScholar(String),
    Pmid(String),
    OpenAlex(String),
    Url(String),
    Unknown(String),
}

impl Identifier {
    /// 从字符串解析标识符
    pub fn parse(s: &str) -> Self {
        let s = s.trim();

        // 带前缀的标识符
        if let Some(id) = s.strip_prefix("arxiv:") {
            return Self::Arxiv(id.to_string());
        }
        if let Some(id) = s.strip_prefix("doi:") {
            return Self::Doi(id.to_string());
        }
        if let Some(id) = s.strip_prefix("ss:") {
            return Self::SemanticScholar(id.to_string());
        }
        if let Some(id) = s.strip_prefix("pmid:") {
            return Self::Pmid(id.to_string());
        }
        if let Some(id) = s.strip_prefix("openalex:") {
            return Self::OpenAlex(id.to_string());
        }

        // URL
        if s.starts_with("http://") || s.starts_with("https://") {
            return Self::Url(s.to_string());
        }

        // 自动识别格式
        if Self::is_arxiv_id(s) {
            return Self::Arxiv(s.to_string());
        }

        if s.starts_with("10.") && s.contains('/') {
            return Self::Doi(s.to_string());
        }

        Self::Unknown(s.to_string())
    }

    fn is_arxiv_id(s: &str) -> bool {
        Self::is_new_arxiv_format(s) || Self::is_old_arxiv_format(s)
    }

    fn is_new_arxiv_format(s: &str) -> bool {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return false;
        }
        if parts[0].len() != 4 || !parts[0].chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        let second = parts[1];
        if let Some(v_pos) = second.find('v') {
            let num = &second[..v_pos];
            let ver = &second[v_pos + 1..];
            num.len() >= 4 && num.len() <= 5 && num.chars().all(|c| c.is_ascii_digit())
                && ver.chars().all(|c| c.is_ascii_digit())
        } else {
            second.len() >= 4 && second.len() <= 5 && second.chars().all(|c| c.is_ascii_digit())
        }
    }

    fn is_old_arxiv_format(s: &str) -> bool {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return false;
        }
        parts[0].chars().all(|c| c.is_ascii_lowercase() || c == '-')
            && parts[1].len() == 7 && parts[1].chars().all(|c| c.is_ascii_digit())
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Arxiv(s) => s,
            Self::Doi(s) => s,
            Self::SemanticScholar(s) => s,
            Self::Pmid(s) => s,
            Self::OpenAlex(s) => s,
            Self::Url(s) => s,
            Self::Unknown(s) => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_parse_arxiv_new_format() {
        let id = Identifier::parse("2301.00001");
        assert!(matches!(id, Identifier::Arxiv(_)));
        assert_eq!(id.as_str(), "2301.00001");
    }

    #[test]
    fn test_identifier_parse_arxiv_with_version() {
        let id = Identifier::parse("2301.00001v2");
        assert!(matches!(id, Identifier::Arxiv(_)));
    }

    #[test]
    fn test_identifier_parse_arxiv_old_format() {
        let id = Identifier::parse("math/0001001");
        assert!(matches!(id, Identifier::Arxiv(_)));
    }

    #[test]
    fn test_identifier_parse_arxiv_prefix() {
        let id = Identifier::parse("arxiv:2301.00001");
        assert!(matches!(id, Identifier::Arxiv(_)));
        assert_eq!(id.as_str(), "2301.00001");
    }

    #[test]
    fn test_identifier_parse_doi() {
        let id = Identifier::parse("10.1234/test");
        assert!(matches!(id, Identifier::Doi(_)));
        assert_eq!(id.as_str(), "10.1234/test");
    }

    #[test]
    fn test_identifier_parse_doi_prefix() {
        let id = Identifier::parse("doi:10.1234/test");
        assert!(matches!(id, Identifier::Doi(_)));
    }

    #[test]
    fn test_identifier_parse_semantic_scholar() {
        let id = Identifier::parse("ss:abc123def");
        assert!(matches!(id, Identifier::SemanticScholar(_)));
    }

    #[test]
    fn test_identifier_parse_url() {
        let id = Identifier::parse("https://arxiv.org/abs/2301.00001");
        assert!(matches!(id, Identifier::Url(_)));

        let id2 = Identifier::parse("http://example.com");
        assert!(matches!(id2, Identifier::Url(_)));
    }

    #[test]
    fn test_identifier_parse_unknown() {
        let id = Identifier::parse("unknown-format");
        assert!(matches!(id, Identifier::Unknown(_)));
    }

    #[test]
    fn test_is_arxiv_id_new_format() {
        assert!(Identifier::is_arxiv_id("2301.00001"));
        assert!(Identifier::is_arxiv_id("2301.00001v2"));
        assert!(Identifier::is_arxiv_id("2301.1234"));
        assert!(!Identifier::is_arxiv_id("230.00001")); // Too short year
        assert!(!Identifier::is_arxiv_id("2301.001")); // Too short number
    }

    #[test]
    fn test_is_arxiv_id_old_format() {
        assert!(Identifier::is_arxiv_id("math/0001001"));
        assert!(Identifier::is_arxiv_id("hep-th/9901001"));
        assert!(!Identifier::is_arxiv_id("math/000100")); // Too short number
    }

    #[test]
    fn test_source_kind_display() {
        assert_eq!(SourceKind::Arxiv.to_string(), "arxiv");
        assert_eq!(SourceKind::SemanticScholar.to_string(), "semantic_scholar");
        assert_eq!(SourceKind::Custom("test").to_string(), "test");
    }

    #[test]
    fn test_source_kind_as_str() {
        assert_eq!(SourceKind::Arxiv.as_str(), "arxiv");
        assert_eq!(SourceKind::SemanticScholar.as_str(), "semantic_scholar");
        assert_eq!(SourceKind::Crossref.as_str(), "crossref");
        assert_eq!(SourceKind::OpenAlex.as_str(), "openalex");
        assert_eq!(SourceKind::Pubmed.as_str(), "pubmed");
        assert_eq!(SourceKind::GoogleScholar.as_str(), "google_scholar");
    }

    #[test]
    fn test_source_error_display() {
        let err = SourceError::Network("timeout".to_string());
        assert_eq!(err.to_string(), "网络错误: timeout");

        let err = SourceError::Parse("invalid json".to_string());
        assert_eq!(err.to_string(), "解析错误: invalid json");

        let err = SourceError::RateLimit { retry_after: Some(60) };
        assert_eq!(err.to_string(), "速率限制，请等待 60 秒后重试");

        let err = SourceError::RateLimit { retry_after: None };
        assert_eq!(err.to_string(), "速率限制，请稍后重试");

        let err = SourceError::NotFound;
        assert_eq!(err.to_string(), "未找到论文");

        let err = SourceError::Other("custom error".to_string());
        assert_eq!(err.to_string(), "错误: custom error");
    }

    #[test]
    fn test_search_params_default() {
        let params = SearchParams::default();
        assert_eq!(params.query, "");
        assert_eq!(params.offset, 0);
        assert_eq!(params.limit, 10);
        assert_eq!(params.sort_by, None);
        assert_eq!(params.sort_order, None);
    }

    #[test]
    fn test_search_result_default() {
        let result = SearchResult::default();
        assert_eq!(result.papers.len(), 0);
        assert_eq!(result.total, None);
        assert!(!result.has_more);
    }

    #[test]
    fn test_source_config_default() {
        let config = SourceConfig::default();
        assert_eq!(config.api_key, None);
        assert_eq!(config.proxy, None);
        assert_eq!(config.timeout, 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay, 1000);
    }

    #[test]
    fn test_source_capabilities() {
        let caps = SourceCapabilities {
            search: true,
            get_by_id: true,
            citations: false,
            references: false,
            authors: false,
            pdf_download: true,
        };
        assert!(caps.search);
        assert!(caps.get_by_id);
        assert!(!caps.citations);
    }
}