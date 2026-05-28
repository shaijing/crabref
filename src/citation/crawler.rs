use std::sync::Arc;
use crate::source::PaperSource;
use crate::source::SourceError;
use super::graph::CitationGraph as Graph;

pub enum CrawlDirection {
    Citations,
    References,
    Both,
}

pub struct CitationCrawler {
    source: Arc<dyn PaperSource>,
    max_depth: usize,
    max_papers: usize,
}

impl CitationCrawler {
    pub fn new(source: Arc<dyn PaperSource>, max_depth: usize, max_papers: usize) -> Self {
        Self {
            source,
            max_depth,
            max_papers,
        }
    }

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_max_papers(mut self, max: usize) -> Self {
        self.max_papers = max;
        self
    }

    /// 从种子论文爬取引用网络
    pub async fn crawl(&self, seed_paper_id: &str, direction: CrawlDirection) -> Result<Graph, SourceError> {
        let mut graph = Graph::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // 获取种子论文
        if let Some(seed) = self.source.get_by_identifier(seed_paper_id).await? {
            let seed_id = seed.semantic_scholar_id.clone().unwrap_or_default();
            visited.insert(seed_id.clone());
            queue.push_back((seed, seed_id, 0));
        }

        while let Some((paper, paper_id, depth)) = queue.pop_front() {
            if graph.paper_count() >= self.max_papers {
                break;
            }

            graph.add_paper(paper.clone());

            if depth >= self.max_depth {
                continue;
            }

            // 爬取引用/被引
            if matches!(direction, CrawlDirection::Citations | CrawlDirection::Both) {
                let citations = self.source.get_citations(&paper_id, 50).await?;
                for (citing, authors) in citations {
                    if let Some(citing_id) = citing.semantic_scholar_id.clone() {
                        if !visited.contains(&citing_id) {
                            visited.insert(citing_id.clone());
                            let mut citing_paper = citing;
                            citing_paper.authors = authors;
                            queue.push_back((citing_paper, citing_id, depth + 1));
                        }
                    }
                }
            }

            if matches!(direction, CrawlDirection::References | CrawlDirection::Both) {
                let refs = self.source.get_references(&paper_id, 50).await?;
                for (cited, authors) in refs {
                    if let Some(cited_id) = cited.semantic_scholar_id.clone() {
                        if !visited.contains(&cited_id) {
                            visited.insert(cited_id.clone());
                            let mut cited_paper = cited;
                            cited_paper.authors = authors;
                            queue.push_back((cited_paper, cited_id, depth + 1));
                        }
                    }
                }
            }
        }

        Ok(graph)
    }
}