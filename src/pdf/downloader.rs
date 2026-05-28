use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// PDF 下载器
pub struct PdfDownloader {
    client: reqwest::Client,
}

impl PdfDownloader {
    pub fn new() -> Result<Self> {
        Self::new_with_proxy(None)
    }

    pub fn new_with_proxy(proxy: Option<&str>) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .read_timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(30));

        if let Some(proxy_url) = proxy {
            if !proxy_url.is_empty() {
                let proxy = reqwest::Proxy::all(proxy_url)
                    .context("Failed to configure proxy")?;
                builder = builder.proxy(proxy);
            }
        }

        let client = builder.build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client })
    }

    /// 下载 PDF 到指定路径
    pub async fn download(&self, url: &str, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create directory")?;
        }

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download PDF: HTTP {}", response.status());
        }

        let mut file = File::create(dest)
            .await
            .context("Failed to create file")?;

        let body = response
            .bytes()
            .await
            .context("Failed to read response body")?;
        file.write_all(&body)
            .await
            .context("Failed to write chunk")?;

        file.flush().await.context("Failed to flush file")?;

        Ok(())
    }

    /// 下载 PDF 并返回本地路径
    pub async fn download_to_dir(
        &self,
        url: &str,
        dir: &Path,
        filename: &str,
    ) -> Result<std::path::PathBuf> {
        let dest = dir.join(filename);
        self.download(url, &dest).await?;
        Ok(dest)
    }

    /// 使用 arXiv ID 构建 PDF URL 并下载
    pub async fn download_arxiv_pdf(&self, arxiv_id: &str, dest: &Path) -> Result<()> {
        let url = format!("https://arxiv.org/pdf/{}", arxiv_id);
        self.download(&url, dest).await
    }

    /// 检查 URL 是否为 PDF
    pub fn is_pdf_url(url: &str) -> bool {
        url.ends_with(".pdf") || url.contains("pdf")
    }

    /// 从 URL 提取文件名
    pub fn extract_filename(url: &str) -> Option<String> {
        url.split('/')
            .last()
            .and_then(|s| {
                if s.ends_with(".pdf") {
                    Some(s.to_string())
                } else {
                    Some(format!("{}.pdf", s))
                }
            })
    }
}

impl Default for PdfDownloader {
    fn default() -> Self {
        Self::new().expect("Failed to create PDF downloader")
    }
}