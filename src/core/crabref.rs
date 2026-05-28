//! Crabref 核心入口

use crate::citation::{CitationGraph, CitationStats};
use crate::config::Config;
use crate::db::Database;
use crate::models::{Author, Paper};
use crate::output::OutputFormat;
use crate::pdf::PdfDownloader;
use crate::source::{SearchParams, SourceKind, SourceManager, SourceBuilder};
use anyhow::Result;
use std::path::PathBuf;

/// 搜索排序方式
#[derive(Debug, Clone, Copy, Default)]
pub enum SortBy {
    #[default]
    Created,
    Citation,
    Title,
}

/// 引用图导出格式
#[derive(Debug, Clone, Copy, Default)]
pub enum GraphFormat {
    #[default]
    Dot,
    Json,
}

/// Crabref 核心库入口
pub struct CrabRef {
    db: Database,
    sources: SourceManager,
    config: Config,
}

impl CrabRef {
    /// 创建 CrabRef 实例
    pub async fn new(mut config: Config) -> Result<Self> {
        config.ensure_dirs()?;
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/crabref".to_string());
        let db = Database::new(&database_url).await?;
        // Fall back to SS_API_KEY env var if not set in config
        if config.semantic_scholar_api_key.is_none() {
            config.semantic_scholar_api_key = std::env::var("SS_API_KEY").ok();
        }
        let sources = build_source_manager(&config)?;
        Ok(Self { db, sources, config })
    }

    /// 使用默认配置创建
    pub async fn with_defaults() -> Result<Self> {
        Self::new(Config::default()).await
    }

    // === 搜索 ===

    /// 搜索论文（所有源）
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Paper>> {
        let results = self.sources.smart_search(query, limit).await?;
        Ok(results.into_iter().map(|(_, p)| p).collect())
    }

    /// 从指定源搜索
    pub async fn search_from(
        &self,
        source: SourceKind,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Paper>> {
        let src = self.sources.get(source)
            .ok_or_else(|| anyhow::anyhow!("源 {:?} 未注册", source))?;

        let params = SearchParams {
            query: query.to_string(),
            limit,
            ..Default::default()
        };

        let result = src.search(&params).await?;
        Ok(result.papers)
    }

    /// 通过标识符获取论文
    pub async fn fetch(&self, identifier: &str) -> Result<Option<Paper>> {
        let result = self.sources.fetch_by_identifier(identifier).await?;
        Ok(result.map(|(_, p)| p))
    }

    /// 通过标识符获取论文并缓存
    pub async fn fetch_and_cache(&self, identifier: &str) -> Result<Option<(i64, Paper)>> {
        if let Some((_, paper)) = self.sources.fetch_by_identifier(identifier).await? {
            let id = self.save(&paper).await?;
            return Ok(Some((id, paper)));
        }
        Ok(None)
    }

    // === 本地存储 ===

    /// 保存论文到本地
    pub async fn save(&self, paper: &Paper) -> Result<i64> {
        // 检查是否已存在
        if let Some(arxiv_id) = &paper.arxiv_id {
            if !arxiv_id.is_empty() {
                if let Some(existing) = self.db.get_paper_by_arxiv_id(arxiv_id).await? {
                    return Ok(existing.id.unwrap());
                }
            }
        }
        if let Some(ss_id) = &paper.semantic_scholar_id {
            if !ss_id.is_empty() {
                if let Some(existing) = self.db.get_paper_by_semantic_scholar_id(ss_id).await? {
                    return Ok(existing.id.unwrap());
                }
            }
        }

        self.db.insert_paper(paper).await
    }

    /// 批量保存
    pub async fn save_batch(&self, papers: &[Paper]) -> Result<Vec<i64>> {
        let mut ids = Vec::new();
        for paper in papers {
            let id = self.save(paper).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// 获取论文
    pub async fn get(&self, id: i64) -> Result<Option<Paper>> {
        self.db.get_paper_by_id(id).await
    }

    /// 列出论文
    pub async fn list(&self, limit: usize, sort: SortBy) -> Result<Vec<Paper>> {
        match sort {
            SortBy::Citation => self.db.top_cited_papers(limit as i64).await,
            _ => self.db.list_papers(limit as i64).await,
        }
    }

    /// 搜索本地数据库
    pub async fn search_local(&self, query: &str, limit: usize) -> Result<Vec<Paper>> {
        self.db.search_papers(query, limit as i64).await
    }

    /// 按作者搜索
    pub async fn search_by_author(&self, name: &str, limit: usize) -> Result<Vec<Paper>> {
        self.db.search_by_author(name, limit as i64).await
    }

    /// 删除论文
    pub async fn delete(&self, id: i64) -> Result<bool> {
        self.db.delete_paper(id).await
    }

    /// 更新论文（从源重新获取信息）
    pub async fn update(&self, id: i64) -> Result<bool> {
        let paper = self.db.get_paper_by_id(id).await?
            .ok_or_else(|| anyhow::anyhow!("论文 {} 不存在", id))?;

        if let Some(ss_id) = &paper.semantic_scholar_id {
            if let Some(source) = self.sources.get(SourceKind::SemanticScholar) {
                if let Some(updated) = source.get_by_id(ss_id).await? {
                    let mut paper = paper.clone();
                    paper.citation_count = updated.citation_count;
                    paper.updated_at = chrono::Utc::now();
                    self.db.update_paper(&paper).await?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// 统计
    pub async fn count(&self) -> Result<i64> {
        self.db.count_papers().await
    }

    // === 引用 ===

    /// 获取引用（被哪些论文引用）
    pub async fn citations(
        &self,
        paper_id: &str,
        limit: usize,
    ) -> Result<Vec<(Paper, Vec<Author>)>> {
        self.sources.get_citations(paper_id, limit).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// 获取参考文献
    pub async fn references(
        &self,
        paper_id: &str,
        limit: usize,
    ) -> Result<Vec<(Paper, Vec<Author>)>> {
        self.sources.get_references(paper_id, limit).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// 获取引用统计
    pub async fn citation_stats(&self) -> Result<CitationStats> {
        let mut graph = CitationGraph::new();

        let papers = self.db.list_papers(10000).await?;
        for paper in papers {
            if let Some(id) = paper.id {
                graph.add_paper(paper);

                if let Ok(cited_ids) = self.db.get_citations(id).await {
                    for cited_id in cited_ids {
                        graph.add_citation(id, cited_id);
                    }
                }

                if let Ok(citing_ids) = self.db.get_cited_by(id).await {
                    for citing_id in citing_ids {
                        graph.add_citation(citing_id, id);
                    }
                }
            }
        }

        Ok(CitationStats::from_graph(&graph))
    }

    /// 同步引用数
    pub async fn sync_citations(&self, batch: usize) -> Result<(usize, usize)> {
        let source = self.sources.get(SourceKind::SemanticScholar)
            .ok_or_else(|| anyhow::anyhow!("Semantic Scholar 源未注册"))?;

        let papers = self.db.list_papers(10000).await?;
        let mut updated = 0;
        let mut failed = 0;

        for paper in papers.iter().take(batch) {
            if let Some(ss_id) = &paper.semantic_scholar_id {
                if paper.id.is_some() {
                    match source.get_by_id(ss_id).await {
                        Ok(Some(updated_paper)) => {
                            let mut p = paper.clone();
                            p.citation_count = updated_paper.citation_count;
                            p.updated_at = chrono::Utc::now();
                            self.db.update_paper(&p).await?;
                            updated += 1;
                        }
                        _ => {
                            failed += 1;
                        }
                    }
                }
            }
        }

        Ok((updated, failed))
    }

    /// 构建引用图
    pub async fn build_citation_graph(&self) -> Result<CitationGraph> {
        let mut graph = CitationGraph::new();

        let papers = self.db.list_papers(10000).await?;
        for paper in papers {
            if let Some(id) = paper.id {
                graph.add_paper(paper);

                if let Ok(cited_ids) = self.db.get_citations(id).await {
                    for cited_id in cited_ids {
                        graph.add_citation(id, cited_id);
                    }
                }

                if let Ok(citing_ids) = self.db.get_cited_by(id).await {
                    for citing_id in citing_ids {
                        graph.add_citation(citing_id, id);
                    }
                }
            }
        }

        Ok(graph)
    }

    // === 导出 ===

    /// 导出论文列表
    pub fn export(&self, papers: &[Paper], format: OutputFormat) -> String {
        let formatter = format.formatter();
        formatter.format_papers(papers)
    }

    /// 导出引用图
    pub fn export_citation_graph(&self, graph: &CitationGraph, format: GraphFormat) -> Result<String> {
        match format {
            GraphFormat::Dot => Ok(graph.to_dot()),
            GraphFormat::Json => graph.to_json(),
        }
    }

    // === PDF ===

    /// 下载 PDF
    pub async fn download_pdf(&self, identifier: &str) -> Result<PathBuf> {
        let downloader = PdfDownloader::new_with_proxy(self.config.proxy.as_deref())?;

        let dest = if identifier.starts_with("http") {
            let filename = crate::pdf::PdfDownloader::extract_filename(identifier)
                .unwrap_or_else(|| "paper.pdf".to_string());
            self.config.pdf_storage_path.join(filename)
        } else {
            self.config.pdf_path(identifier)
        };

        if identifier.starts_with("http") {
            downloader.download(identifier, &dest).await?;
        } else {
            downloader.download_arxiv_pdf(identifier, &dest).await?;
        }

        Ok(dest)
    }

    // === 配置访问 ===

    /// 获取配置
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取数据库
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// 获取源管理器
    pub fn sources(&self) -> &SourceManager {
        &self.sources
    }
}

/// 构建器模式
pub struct CrabRefBuilder {
    config: Config,
}

impl CrabRefBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn pdf_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.pdf_storage_path = path.into();
        self
    }

    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.config.proxy = Some(proxy.into());
        self
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.semantic_scholar_api_key = Some(key.into());
        self
    }

    pub async fn build(self) -> Result<CrabRef> {
        CrabRef::new(self.config).await
    }
}

impl Default for CrabRefBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建源管理器
fn build_source_manager(config: &Config) -> Result<SourceManager> {
    let mut manager = SourceManager::new();

    // arXiv
    let mut arxiv_builder = SourceBuilder::new()
        .timeout(30)
        .max_retries(3);

    if let Some(ref proxy) = config.proxy {
        if !proxy.is_empty() {
            arxiv_builder = arxiv_builder.proxy(proxy);
        }
    }
    manager.register(arxiv_builder.build_arxiv()?);

    // Semantic Scholar
    let mut ss_builder = SourceBuilder::new()
        .timeout(30)
        .max_retries(5);

    if let Some(ref key) = config.semantic_scholar_api_key {
        if !key.is_empty() {
            ss_builder = ss_builder.api_key(key);
        }
    }
    if let Some(ref proxy) = config.proxy {
        if !proxy.is_empty() {
            ss_builder = ss_builder.proxy(proxy);
        }
    }
    manager.register(ss_builder.build_semantic_scholar()?);

    // OpenAlex
    let mut oa_builder = SourceBuilder::new()
        .timeout(30)
        .max_retries(3);

    if let Some(ref proxy) = config.proxy {
        if !proxy.is_empty() {
            oa_builder = oa_builder.proxy(proxy);
        }
    }
    manager.register(oa_builder.build_openalex()?);

    Ok(manager)
}