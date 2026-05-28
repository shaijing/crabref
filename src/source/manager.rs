//! 论文源管理器

use crate::models::{Author, Paper};
use crate::source::*;
use std::collections::HashMap;
use std::sync::Arc;

/// 论文源管理器
pub struct SourceManager {
    sources: HashMap<SourceKind, Arc<dyn PaperSource>>,
    default_source: Option<SourceKind>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            default_source: None,
        }
    }

    /// 注册论文源
    pub fn register<S: PaperSource + 'static>(&mut self, source: S) {
        let kind = source.kind();
        self.sources.insert(kind, Arc::new(source));
    }

    /// 设置默认论文源
    pub fn set_default(&mut self, kind: SourceKind) {
        self.default_source = Some(kind);
    }

    /// 获取论文源
    pub fn get(&self, kind: SourceKind) -> Option<Arc<dyn PaperSource>> {
        self.sources.get(&kind).cloned()
    }

    /// 获取默认论文源
    pub fn get_default(&self) -> Option<Arc<dyn PaperSource>> {
        self.default_source.as_ref().and_then(|k| self.get(*k))
    }

    /// 列出所有已注册的论文源
    pub fn list_sources(&self) -> Vec<SourceKind> {
        self.sources.keys().copied().collect()
    }

    /// 检查论文源是否支持某功能
    pub fn supports(&self, kind: SourceKind, capability: fn(&SourceCapabilities) -> bool) -> bool {
        self.sources.get(&kind)
            .map(|s| capability(&s.capabilities()))
            .unwrap_or(false)
    }

    /// 智能搜索：自动选择合适的论文源
    pub async fn smart_search(&self, query: &str, limit: usize) -> Result<Vec<(SourceKind, Paper)>, SourceError> {
        let mut results = Vec::new();

        for (kind, source) in &self.sources {
            if source.capabilities().search {
                let params = SearchParams {
                    query: query.to_string(),
                    limit,
                    ..Default::default()
                };

                match source.search(&params).await {
                    Ok(search_result) => {
                        for paper in search_result.papers {
                            results.push((*kind, paper));
                        }
                    }
                    Err(SourceError::RateLimit { .. }) => continue,
                    Err(e) => tracing::warn!("Source {} search failed: {}", kind, e),
                }
            }
        }

        Ok(results)
    }

    /// 通过标识符获取论文（自动识别来源）
    pub async fn fetch_by_identifier(&self, identifier: &str) -> Result<Option<(SourceKind, Paper)>, SourceError> {
        let ident = Identifier::parse(identifier);

        // 根据标识符类型选择合适的源
        match &ident {
            Identifier::Arxiv(id) => {
                if let Some(source) = self.get(SourceKind::Arxiv) {
                    if let Some(paper) = source.get_by_id(id).await? {
                        return Ok(Some((SourceKind::Arxiv, paper)));
                    }
                }
                // 也可以尝试 Semantic Scholar
                if let Some(source) = self.get(SourceKind::SemanticScholar) {
                    if let Some(paper) = source.get_by_identifier(identifier).await? {
                        return Ok(Some((SourceKind::SemanticScholar, paper)));
                    }
                }
            }
            Identifier::SemanticScholar(id) => {
                if let Some(source) = self.get(SourceKind::SemanticScholar) {
                    if let Some(paper) = source.get_by_id(id).await? {
                        return Ok(Some((SourceKind::SemanticScholar, paper)));
                    }
                }
            }
            Identifier::Doi(doi) => {
                if let Some(source) = self.get(SourceKind::SemanticScholar) {
                    if let Some(paper) = source.get_by_identifier(&format!("doi:{}", doi)).await? {
                        return Ok(Some((SourceKind::SemanticScholar, paper)));
                    }
                }
            }
            Identifier::Url(url) => {
                // 尝试从 URL 中识别来源
                if url.contains("arxiv.org") {
                    if let Some(source) = self.get(SourceKind::Arxiv) {
                        if let Some(paper) = source.get_by_identifier(url).await? {
                            return Ok(Some((SourceKind::Arxiv, paper)));
                        }
                    }
                }
            }
            _ => {}
        }

        // 尝试所有支持的源
        for (kind, source) in &self.sources {
            if source.capabilities().get_by_id {
                if let Some(paper) = source.get_by_identifier(identifier).await? {
                    return Ok(Some((*kind, paper)));
                }
            }
        }

        Ok(None)
    }

    /// 获取引用关系（选择支持该功能的源）
    pub async fn get_citations(&self, paper_id: &str, limit: usize) -> Result<Vec<(Paper, Vec<Author>)>, SourceError> {
        for (_, source) in &self.sources {
            if source.capabilities().citations {
                match source.get_citations(paper_id, limit).await {
                    Ok(result) => return Ok(result),
                    Err(SourceError::NotFound) => continue,
                    Err(e) => return Err(e),
                }
            }
        }
        Err(SourceError::Other("没有支持获取引用关系的论文源".into()))
    }

    /// 获取参考文献
    pub async fn get_references(&self, paper_id: &str, limit: usize) -> Result<Vec<(Paper, Vec<Author>)>, SourceError> {
        for (_, source) in &self.sources {
            if source.capabilities().references {
                match source.get_references(paper_id, limit).await {
                    Ok(result) => return Ok(result),
                    Err(SourceError::NotFound) => continue,
                    Err(e) => return Err(e),
                }
            }
        }
        Err(SourceError::Other("没有支持获取参考文献的论文源".into()))
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 论文源构建器
pub struct SourceBuilder {
    config: SourceConfig,
}

impl SourceBuilder {
    pub fn new() -> Self {
        Self {
            config: SourceConfig::default(),
        }
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.config.proxy = Some(proxy.into());
        self
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout = seconds;
        self
    }

    pub fn max_retries(mut self, retries: usize) -> Self {
        self.config.max_retries = retries;
        self
    }

    pub fn build_arxiv(self) -> Result<ArxivSource, SourceError> {
        ArxivSource::new(self.config)
    }

    pub fn build_semantic_scholar(self) -> Result<SemanticScholarSource, SourceError> {
        SemanticScholarSource::new(self.config)
    }

    pub fn build_openalex(self) -> Result<OpenAlexSource, SourceError> {
        OpenAlexSource::new(self.config)
    }
}

impl Default for SourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_manager_new() {
        let manager = SourceManager::new();
        assert_eq!(manager.list_sources().len(), 0);
    }

    #[test]
    fn test_source_manager_default() {
        let manager = SourceManager::default();
        assert_eq!(manager.list_sources().len(), 0);
    }

    #[test]
    fn test_source_manager_register() {
        let mut manager = SourceManager::new();
        let source = ArxivSource::new(SourceConfig::default()).unwrap();
        manager.register(source);
        assert_eq!(manager.list_sources().len(), 1);
        assert!(manager.get(SourceKind::Arxiv).is_some());
    }

    #[test]
    fn test_source_manager_get_missing() {
        let manager = SourceManager::new();
        assert!(manager.get(SourceKind::Arxiv).is_none());
    }

    #[test]
    fn test_source_manager_set_default() {
        let mut manager = SourceManager::new();
        let source = ArxivSource::new(SourceConfig::default()).unwrap();
        manager.register(source);
        manager.set_default(SourceKind::Arxiv);
        assert!(manager.get_default().is_some());
    }

    #[test]
    fn test_source_manager_list_sources() {
        let mut manager = SourceManager::new();
        let arxiv = ArxivSource::new(SourceConfig::default()).unwrap();
        let ss = SemanticScholarSource::new(SourceConfig::default()).unwrap();
        manager.register(arxiv);
        manager.register(ss);

        let sources = manager.list_sources();
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&SourceKind::Arxiv));
        assert!(sources.contains(&SourceKind::SemanticScholar));
    }

    #[test]
    fn test_source_manager_supports() {
        let mut manager = SourceManager::new();
        let source = ArxivSource::new(SourceConfig::default()).unwrap();
        manager.register(source);

        assert!(manager.supports(SourceKind::Arxiv, |c| c.search));
        assert!(!manager.supports(SourceKind::Arxiv, |c| c.citations));
        assert!(!manager.supports(SourceKind::SemanticScholar, |c| c.search));
    }

    #[test]
    fn test_source_builder_new() {
        let builder = SourceBuilder::new();
        assert_eq!(builder.config.api_key, None);
        assert_eq!(builder.config.proxy, None);
        assert_eq!(builder.config.timeout, 30);
    }

    #[test]
    fn test_source_builder_api_key() {
        let builder = SourceBuilder::new().api_key("test-key");
        assert_eq!(builder.config.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_source_builder_proxy() {
        let builder = SourceBuilder::new().proxy("http://127.0.0.1:7890");
        assert_eq!(builder.config.proxy, Some("http://127.0.0.1:7890".to_string()));
    }

    #[test]
    fn test_source_builder_timeout() {
        let builder = SourceBuilder::new().timeout(60);
        assert_eq!(builder.config.timeout, 60);
    }

    #[test]
    fn test_source_builder_max_retries() {
        let builder = SourceBuilder::new().max_retries(5);
        assert_eq!(builder.config.max_retries, 5);
    }

    #[test]
    fn test_source_builder_build_arxiv() {
        let source = SourceBuilder::new().build_arxiv();
        assert!(source.is_ok());
    }

    #[test]
    fn test_source_builder_build_semantic_scholar() {
        let source = SourceBuilder::new().build_semantic_scholar();
        assert!(source.is_ok());
    }

    #[test]
    fn test_source_builder_chained() {
        let source = SourceBuilder::new()
            .api_key("test-key")
            .proxy("http://127.0.0.1:7890")
            .timeout(60)
            .max_retries(5)
            .build_arxiv();

        assert!(source.is_ok());
    }
}