use crate::models::Paper;
use super::graph::CitationGraph;

pub struct CitationStats {
    pub total_papers: usize,
    pub total_citation_edges: usize,
    pub average_citations: f64,
    pub h_index: i32,
    pub max_citations: i64,
    pub most_cited_papers: Vec<(Paper, i64)>,
    pub isolated_papers: Vec<Paper>,
}

impl CitationStats {
    /// 从 CitationGraph 计算统计
    pub fn from_graph(graph: &CitationGraph) -> Self {
        let total_papers = graph.paper_count();
        let total_citation_edges = graph.edge_count();

        // 计算各论文的引用数
        let citation_counts: Vec<(i64, usize)> = graph.papers()
            .filter_map(|p| p.id.map(|id| (id, graph.citation_count(id))))
            .collect();

        // 平均引用数
        let average_citations = if total_papers > 0 {
            citation_counts.iter().map(|(_, c)| *c).sum::<usize>() as f64 / total_papers as f64
        } else {
            0.0
        };

        // 计算 H-index
        let mut sorted_counts: Vec<usize> = citation_counts.iter()
            .map(|(_, c)| *c)
            .collect();
        sorted_counts.sort_by(|a, b| b.cmp(a));

        let h_index: i32 = (0..sorted_counts.len())
            .take_while(|i| sorted_counts[*i] >= *i + 1)
            .count() as i32;

        // 最大引用数
        let max_citations = citation_counts.iter()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(0) as i64;

        // 引用最多的论文
        let mut most_cited: Vec<(Paper, i64)> = graph.papers()
            .filter_map(|p| {
                p.id.map(|id| {
                    let count = graph.citation_count(id) as i64;
                    (p.clone(), count)
                })
            })
            .collect();
        most_cited.sort_by(|a, b| b.1.cmp(&a.1));
        most_cited.truncate(10);

        // 孤立论文（无引用关系）
        let isolated_papers: Vec<Paper> = graph.papers()
            .filter(|p| {
                p.id.map(|id| {
                    graph.get_citations(id).is_none() && graph.get_cited_by(id).is_none()
                }).unwrap_or(true)
            })
            .cloned()
            .collect();

        Self {
            total_papers,
            total_citation_edges,
            average_citations,
            h_index,
            max_citations,
            most_cited_papers: most_cited,
            isolated_papers,
        }
    }
}

impl Default for CitationStats {
    fn default() -> Self {
        Self {
            total_papers: 0,
            total_citation_edges: 0,
            average_citations: 0.0,
            h_index: 0,
            max_citations: 0,
            most_cited_papers: Vec::new(),
            isolated_papers: Vec::new(),
        }
    }
}