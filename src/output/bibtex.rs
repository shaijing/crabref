use super::{CitationDirection, OutputFormatter};
use crate::models::{Author, Paper};

pub struct BibtexFormatter;

impl BibtexFormatter {
    /// 生成 citation key: 第一作者姓氏 + 年份
    fn generate_key(paper: &Paper) -> String {
        let author_part = if let Some(first_author) = paper.authors.first() {
            first_author.name
                .split_whitespace()
                .last()
                .unwrap_or(&first_author.name)
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric(), "")
        } else {
            "unknown".to_string()
        };

        let year_part = paper.publication_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .unwrap_or("nodate");

        format!("{}{}", author_part, year_part)
    }

    /// 转义 BibTeX 特殊字符
    fn escape_bibtex(s: &str) -> String {
        s.replace('&', "\\&")
            .replace('%', "\\%")
            .replace('$', "\\$")
            .replace('#', "\\#")
            .replace('_', "\\_")
            .replace('{', "\\{")
            .replace('}', "\\}")
    }

    fn format_authors(authors: &[Author]) -> String {
        authors
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>()
            .join(" and ")
    }
}

impl OutputFormatter for BibtexFormatter {
    fn format_papers(&self, papers: &[Paper]) -> String {
        papers
            .iter()
            .map(|p| self.format_paper_detail(p))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn format_paper_detail(&self, paper: &Paper) -> String {
        let key = Self::generate_key(paper);
        let entry_type = if paper.arxiv_id.is_some() {
            "misc"
        } else {
            "article"
        };

        let mut output = format!("@{}{{{},\n", entry_type, key);
        output.push_str(&format!("  title = {{{}}},\n", Self::escape_bibtex(&paper.title)));

        if !paper.authors.is_empty() {
            output.push_str(&format!("  author = {{{}}},\n", Self::format_authors(&paper.authors)));
        }

        if let Some(year) = &paper.publication_date {
            if let Some(y) = year.split('-').next() {
                output.push_str(&format!("  year = {{{}}},\n", y));
            }
        }

        if let Some(venue) = &paper.venue {
            output.push_str(&format!("  journal = {{{}}},\n", venue));
        }

        if let Some(doi) = &paper.doi {
            output.push_str(&format!("  doi = {{{}}},\n", doi));
        }

        if let Some(arxiv) = &paper.arxiv_id {
            output.push_str(&format!("  eprint = {{{}}},\n", arxiv));
            output.push_str("  archiveprefix = {arXiv},\n");
        }

        if let Some(url) = &paper.pdf_url {
            output.push_str(&format!("  url = {{{}}},\n", url));
        }

        output.push_str("}");
        output
    }

    fn format_citations(&self, citations: &[(Paper, Vec<Author>)], _direction: CitationDirection) -> String {
        citations
            .iter()
            .map(|(paper, _)| self.format_paper_detail(paper))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn format_stats(&self, _stats: &crate::citation::CitationStats) -> String {
        "% Citation statistics not available in BibTeX format\n".to_string()
    }

    fn extension(&self) -> &'static str {
        "bib"
    }
}