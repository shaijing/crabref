use super::{CitationDirection, OutputFormatter};
use crate::models::{Author, Paper};

pub struct TerminalFormatter;

impl TerminalFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl TerminalFormatter {
    fn format_identifier(paper: &Paper) -> String {
        let mut parts = Vec::new();
        if let Some(arxiv_id) = &paper.arxiv_id {
            if !arxiv_id.is_empty() {
                parts.push(format!("arXiv: {}", arxiv_id));
            }
        }
        if let Some(ss_id) = &paper.semantic_scholar_id {
            if !ss_id.is_empty() {
                parts.push(format!("SS: {}", ss_id));
            }
        }
        parts.join(" | ")
    }

    fn format_authors(authors: &[Author]) -> String {
        if authors.is_empty() {
            return "N/A".to_string();
        }
        let names: Vec<&str> = authors.iter().map(|a| a.name.as_str()).collect();
        if names.len() <= 3 {
            names.join(", ")
        } else {
            format!("{}, et al.", names[0])
        }
    }
}

impl OutputFormatter for TerminalFormatter {
    fn format_papers(&self, papers: &[Paper]) -> String {
        let mut output = format!("共 {} 篇论文:\n", papers.len());

        for paper in papers {
            output.push_str("\n---\n");
            output.push_str(&format!("ID: {}\n", paper.id.unwrap_or(0)));
            output.push_str(&format!("标题：{}\n", paper.title));

            let identifier = Self::format_identifier(paper);
            if !identifier.is_empty() {
                output.push_str(&format!("{}\n", identifier));
            }

            output.push_str(&format!("作者：{}\n", Self::format_authors(&paper.authors)));
            output.push_str(&format!("引用数：{}\n", paper.citation_count));
        }

        output
    }

    fn format_paper_detail(&self, paper: &Paper) -> String {
        let mut output = String::new();

        output.push_str(&format!("[DB ID: {}]\n", paper.id.unwrap_or(0)));
        output.push_str(&format!("标题：{}\n", paper.title));

        let identifier = Self::format_identifier(paper);
        if !identifier.is_empty() {
            output.push_str(&format!("{}\n", identifier));
        }

        output.push_str(&format!("作者：{}\n", Self::format_authors(&paper.authors)));
        output.push_str(&format!("DOI: {}\n", paper.doi.as_deref().unwrap_or("N/A")));
        output.push_str(&format!("引用数：{}\n", paper.citation_count));
        output.push_str(&format!("PDF: {}\n", paper.pdf_url.as_deref().unwrap_or("N/A")));
        output.push_str(&format!("\n摘要:\n{}\n", paper.abstract_text.as_deref().unwrap_or("N/A")));

        output
    }

    fn format_citations(&self, citations: &[(Paper, Vec<Author>)], direction: CitationDirection) -> String {
        let header = match direction {
            CitationDirection::Citing => format!("该论文被以下 {} 篇论文引用:", citations.len()),
            CitationDirection::Cited => format!("该论文引用了以下 {} 篇论文:", citations.len()),
        };

        let mut output = header;

        for (paper, authors) in citations {
            output.push_str("\n---\n");
            output.push_str(&format!("标题：{}\n", paper.title));

            if let Some(ss_id) = &paper.semantic_scholar_id {
                if !ss_id.is_empty() {
                    output.push_str(&format!("SS ID: {}\n", ss_id));
                }
            }

            output.push_str(&format!("作者：{}\n", Self::format_authors(authors)));
        }

        output
    }

    fn format_stats(&self, stats: &crate::citation::CitationStats) -> String {
        let mut output = String::new();

        output.push_str("=== 引用统计 ===\n\n");
        output.push_str(&format!("论文总数：{}\n", stats.total_papers));
        output.push_str(&format!("引用关系数：{}\n", stats.total_citation_edges));
        output.push_str(&format!("平均引用数：{:.2}\n", stats.average_citations));
        output.push_str(&format!("H-Index: {}\n", stats.h_index));
        output.push_str(&format!("最大引用数：{}\n", stats.max_citations));

        output.push_str("\n=== 引用最多论文 Top 10 ===\n");
        for (i, (paper, count)) in stats.most_cited_papers.iter().take(10).enumerate() {
            output.push_str(&format!("\n{}. {} (引用：{})\n", i + 1, paper.title, count));
        }

        if !stats.isolated_papers.is_empty() {
            output.push_str(&format!("\n孤立节点（无引用关系）: {} 篇\n", stats.isolated_papers.len()));
        }

        output
    }

    fn extension(&self) -> &'static str {
        "txt"
    }
}

impl Default for TerminalFormatter {
    fn default() -> Self {
        Self::new()
    }
}