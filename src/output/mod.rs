//! 输出格式抽象层

mod terminal;
mod json;
mod bibtex;
mod markdown;

pub use terminal::TerminalFormatter;
pub use json::JsonFormatter;
pub use bibtex::BibtexFormatter;
pub use markdown::MarkdownFormatter;

use crate::models::{Author, Paper};

/// 引用方向
#[derive(Debug, Clone, Copy)]
pub enum CitationDirection {
    /// 被哪些论文引用
    Citing,
    /// 引用了哪些论文
    Cited,
}

/// 输出格式类型
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Terminal,
    Json,
    Bibtex,
    Markdown,
}

impl OutputFormat {
    pub fn formatter(&self) -> Box<dyn OutputFormatter> {
        match self {
            Self::Terminal => Box::new(TerminalFormatter::new()),
            Self::Json => Box::new(JsonFormatter),
            Self::Bibtex => Box::new(BibtexFormatter),
            Self::Markdown => Box::new(MarkdownFormatter),
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Terminal => "txt",
            Self::Json => "json",
            Self::Bibtex => "bib",
            Self::Markdown => "md",
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "terminal" | "text" => Ok(Self::Terminal),
            "json" => Ok(Self::Json),
            "bibtex" | "bib" => Ok(Self::Bibtex),
            "markdown" | "md" => Ok(Self::Markdown),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// 输出格式化器 trait
pub trait OutputFormatter: Send + Sync {
    /// 格式化论文列表
    fn format_papers(&self, papers: &[Paper]) -> String;

    /// 格式化单篇论文详情
    fn format_paper_detail(&self, paper: &Paper) -> String;

    /// 格式化引用列表
    fn format_citations(
        &self,
        citations: &[(Paper, Vec<Author>)],
        direction: CitationDirection,
    ) -> String;

    /// 格式化引用统计
    fn format_stats(&self, stats: &crate::citation::CitationStats) -> String;

    /// 获取文件扩展名
    fn extension(&self) -> &'static str;
}