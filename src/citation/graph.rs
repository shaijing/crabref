use std::collections::HashMap;
use crate::models::Paper;
use anyhow::Result;
use serde_json::json;

pub struct CitationGraph {
    papers: HashMap<i64, Paper>,
    citations: HashMap<i64, Vec<i64>>,
    cited_by: HashMap<i64, Vec<i64>>,
}

impl CitationGraph {
    pub fn new() -> Self {
        Self {
            papers: HashMap::new(),
            citations: HashMap::new(),
            cited_by: HashMap::new(),
        }
    }

    /// 添加论文节点
    pub fn add_paper(&mut self, paper: Paper) {
        if let Some(id) = paper.id {
            self.papers.insert(id, paper);
        }
    }

    /// 添加引用关系 (citing_id 引用了 cited_id)
    pub fn add_citation(&mut self, citing_id: i64, cited_id: i64) {
        self.citations.entry(citing_id).or_default().push(cited_id);
        self.cited_by.entry(cited_id).or_default().push(citing_id);
    }

    /// 获取论文的引用（该论文引用了哪些论文）
    pub fn get_citations(&self, paper_id: i64) -> Option<&[i64]> {
        self.citations.get(&paper_id).map(|v| v.as_slice())
    }

    /// 获取论文被引（哪些论文引用了该论文）
    pub fn get_cited_by(&self, paper_id: i64) -> Option<&[i64]> {
        self.cited_by.get(&paper_id).map(|v| v.as_slice())
    }

    /// 获取所有论文
    pub fn papers(&self) -> impl Iterator<Item = &Paper> {
        self.papers.values()
    }

    /// 获取引用数（被引次数）
    pub fn citation_count(&self, paper_id: i64) -> usize {
        self.cited_by.get(&paper_id).map(|v| v.len()).unwrap_or(0)
    }

    /// 获取论文总数
    pub fn paper_count(&self) -> usize {
        self.papers.len()
    }

    /// 获取引用边总数
    pub fn edge_count(&self) -> usize {
        self.citations.values().map(|v| v.len()).sum()
    }

    /// 导出为 Graphviz DOT 格式
    pub fn to_dot(&self) -> String {
        let mut output = String::from("digraph citations {\n");
        output.push_str("  rankdir=LR;\n");
        output.push_str("  node [shape=box];\n\n");

        // 节点
        for (id, paper) in &self.papers {
            let label = paper.title.chars().take(50).collect::<String>();
            output.push_str(&format!("  {} [label=\"{}\"];\n", id, label));
        }

        // 边
        output.push_str("\n");
        for (citing_id, cited_ids) in &self.citations {
            for cited_id in cited_ids {
                output.push_str(&format!("  {} -> {};\n", citing_id, cited_id));
            }
        }

        output.push_str("}\n");
        output
    }

    /// 导出为 JSON 图结构
    pub fn to_json(&self) -> Result<String> {
        let nodes: Vec<_> = self.papers.values().collect();
        let edges: Vec<(i64, i64)> = self.citations.iter()
            .flat_map(|(from, tos)| tos.iter().map(move |to| (*from, *to)))
            .collect();

        serde_json::to_string_pretty(&json!({
            "nodes": nodes,
            "edges": edges
        })).map_err(|e| anyhow::anyhow!(e))
    }
}

impl Default for CitationGraph {
    fn default() -> Self {
        Self::new()
    }
}