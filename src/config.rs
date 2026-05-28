use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// PDF 存储路径
    pub pdf_storage_path: PathBuf,
    /// Semantic Scholar API Key（可选）
    pub semantic_scholar_api_key: Option<String>,
    /// 引用图获取深度
    pub citation_depth: usize,
    /// HTTP/HTTPS 代理地址（可选）
    pub proxy: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("crabref");

        Self {
            pdf_storage_path: data_dir.join("papers"),
            semantic_scholar_api_key: None,
            citation_depth: 1,
            proxy: None,
        }
    }
}

impl Config {
    /// 从文件加载配置
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 确保存储目录存在
    pub fn ensure_dirs(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.pdf_storage_path)?;
        Ok(())
    }

    /// 获取 PDF 存储路径
    pub fn pdf_path(&self, paper_id: &str) -> PathBuf {
        self.pdf_storage_path.join(format!("{}.pdf", paper_id))
    }
}