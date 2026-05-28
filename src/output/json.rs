use super::{CitationDirection, OutputFormatter};
use crate::models::{Author, Paper};
use serde_json::json;

pub struct JsonFormatter;

impl JsonFormatter {
    fn paper_to_json(paper: &Paper) -> serde_json::Value {
        json!({
            "id": paper.id,
            "title": paper.title,
            "abstract": paper.abstract_text,
            "arxiv_id": paper.arxiv_id,
            "semantic_scholar_id": paper.semantic_scholar_id,
            "doi": paper.doi,
            "pdf_url": paper.pdf_url,
            "local_pdf_path": paper.local_pdf_path,
            "publication_date": paper.publication_date,
            "venue": paper.venue,
            "citation_count": paper.citation_count,
            "authors": paper.authors.iter().map(|a| json!({
                "id": a.id,
                "name": a.name,
                "semantic_scholar_id": a.semantic_scholar_id
            })).collect::<Vec<_>>(),
            "created_at": paper.created_at.to_rfc3339(),
            "updated_at": paper.updated_at.to_rfc3339()
        })
    }
}

impl OutputFormatter for JsonFormatter {
    fn format_papers(&self, papers: &[Paper]) -> String {
        let json = json!({
            "total": papers.len(),
            "papers": papers.iter().map(Self::paper_to_json).collect::<Vec<_>>()
        });
        serde_json::to_string_pretty(&json).unwrap_or_default()
    }

    fn format_paper_detail(&self, paper: &Paper) -> String {
        serde_json::to_string_pretty(&Self::paper_to_json(paper)).unwrap_or_default()
    }

    fn format_citations(&self, citations: &[(Paper, Vec<Author>)], direction: CitationDirection) -> String {
        let dir_str = match direction {
            CitationDirection::Citing => "citing",
            CitationDirection::Cited => "cited",
        };

        let json = json!({
            "direction": dir_str,
            "total": citations.len(),
            "papers": citations.iter().map(|(paper, authors)| {
                json!({
                    "paper": Self::paper_to_json(paper),
                    "authors": authors.iter().map(|a| json!({
                        "id": a.id,
                        "name": a.name,
                        "semantic_scholar_id": a.semantic_scholar_id
                    })).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&json).unwrap_or_default()
    }

    fn format_stats(&self, stats: &crate::citation::CitationStats) -> String {
        let json = json!({
            "total_papers": stats.total_papers,
            "total_citation_edges": stats.total_citation_edges,
            "average_citations": stats.average_citations,
            "h_index": stats.h_index,
            "max_citations": stats.max_citations,
            "most_cited_papers": stats.most_cited_papers.iter().map(|(paper, count)| {
                json!({
                    "paper": Self::paper_to_json(paper),
                    "citation_count": count
                })
            }).collect::<Vec<_>>(),
            "isolated_papers_count": stats.isolated_papers.len()
        });

        serde_json::to_string_pretty(&json).unwrap_or_default()
    }

    fn extension(&self) -> &'static str {
        "json"
    }
}