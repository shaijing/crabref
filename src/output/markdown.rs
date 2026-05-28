use super::{CitationDirection, OutputFormatter};
use crate::models::{Author, Paper};

pub struct MarkdownFormatter;

impl MarkdownFormatter {
    fn format_authors(authors: &[Author]) -> String {
        if authors.is_empty() {
            return "N/A".to_string();
        }
        let names: Vec<&str> = authors.iter().map(|a| a.name.as_str()).collect();
        names.join(", ")
    }

    fn escape_markdown(s: &str) -> String {
        s.replace('|', "\\|")
            .replace('\n', " ")
    }
}

impl OutputFormatter for MarkdownFormatter {
    fn format_papers(&self, papers: &[Paper]) -> String {
        let mut output = String::from("# Paper List\n\n");
        output.push_str("| ID | Title | Authors | Citations |\n");
        output.push_str("|----|-------|---------|----------|\n");

        for paper in papers {
            let title = Self::escape_markdown(&paper.title);
            let authors = Self::escape_markdown(&Self::format_authors(&paper.authors));
            output.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                paper.id.unwrap_or(0),
                title,
                authors,
                paper.citation_count
            ));
        }

        output
    }

    fn format_paper_detail(&self, paper: &Paper) -> String {
        let mut output = String::new();

        output.push_str(&format!("# {}\n\n", paper.title));

        output.push_str("## Metadata\n\n");
        output.push_str(&format!("- **ID:** {}\n", paper.id.unwrap_or(0)));

        if let Some(arxiv_id) = &paper.arxiv_id {
            if !arxiv_id.is_empty() {
                output.push_str(&format!("- **arXiv:** {}\n", arxiv_id));
            }
        }

        if let Some(ss_id) = &paper.semantic_scholar_id {
            if !ss_id.is_empty() {
                output.push_str(&format!("- **Semantic Scholar:** {}\n", ss_id));
            }
        }

        if let Some(doi) = &paper.doi {
            output.push_str(&format!("- **DOI:** {}\n", doi));
        }

        output.push_str(&format!("- **Authors:** {}\n", Self::format_authors(&paper.authors)));
        output.push_str(&format!("- **Citations:** {}\n", paper.citation_count));

        if let Some(venue) = &paper.venue {
            output.push_str(&format!("- **Venue:** {}\n", venue));
        }

        if let Some(url) = &paper.pdf_url {
            output.push_str(&format!("- **PDF:** {}\n", url));
        }

        if let Some(abs) = &paper.abstract_text {
            output.push_str("\n## Abstract\n\n");
            output.push_str(abs);
            output.push('\n');
        }

        output
    }

    fn format_citations(&self, citations: &[(Paper, Vec<Author>)], direction: CitationDirection) -> String {
        let title = match direction {
            CitationDirection::Citing => "# Citing Papers",
            CitationDirection::Cited => "# Cited Papers",
        };

        let mut output = format!("{}\n\n", title);
        output.push_str("| Title | Authors |\n");
        output.push_str("|-------|--------|\n");

        for (paper, authors) in citations {
            let title = Self::escape_markdown(&paper.title);
            let author_str = Self::escape_markdown(&Self::format_authors(authors));
            output.push_str(&format!("| {} | {} |\n", title, author_str));
        }

        output
    }

    fn format_stats(&self, stats: &crate::citation::CitationStats) -> String {
        let mut output = String::from("# Citation Statistics\n\n");

        output.push_str("## Overview\n\n");
        output.push_str(&format!("- **Total Papers:** {}\n", stats.total_papers));
        output.push_str(&format!("- **Total Citation Edges:** {}\n", stats.total_citation_edges));
        output.push_str(&format!("- **Average Citations:** {:.2}\n", stats.average_citations));
        output.push_str(&format!("- **H-Index:** {}\n", stats.h_index));
        output.push_str(&format!("- **Max Citations:** {}\n", stats.max_citations));

        output.push_str("\n## Most Cited Papers\n\n");
        output.push_str("| Rank | Title | Citations |\n");
        output.push_str("|------|-------|----------|\n");

        for (i, (paper, count)) in stats.most_cited_papers.iter().take(10).enumerate() {
            let title = Self::escape_markdown(&paper.title);
            output.push_str(&format!("| {} | {} | {} |\n", i + 1, title, count));
        }

        output
    }

    fn extension(&self) -> &'static str {
        "md"
    }
}