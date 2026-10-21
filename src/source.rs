use crate::config::Config;
use anyhow::{Context, Result, ensure};
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use object_store::{ObjectStore, aws::AmazonS3Builder, path::Path};
use std::{env, path::PathBuf, sync::Arc};
use tokio::io::AsyncReadExt;

pub enum Source {
    Local(PathBuf),
    S3(Arc<dyn ObjectStore>),
}

impl Source {
    pub async fn new(c: &Config) -> Result<Self> {
        if let Some(path) = &c.source_dir {
            return Ok(Self::Local(
                tokio::fs::canonicalize(path)
                    .await
                    .context("SOURCE_DIR unavailable")?,
            ));
        }
        let endpoint =
            env::var("S3_ENDPOINT").context("S3_ENDPOINT required without SOURCE_DIR")?;
        let store = AmazonS3Builder::new()
            .with_access_key_id(env::var("S3_ACCESS_KEY_ID").context("S3_ACCESS_KEY_ID required")?)
            .with_secret_access_key(
                env::var("S3_ACCESS_KEY_SECRET").context("S3_ACCESS_KEY_SECRET required")?,
            )
            .with_bucket_name(env::var("S3_BUCKET").context("S3_BUCKET required")?)
            .with_endpoint(&endpoint)
            .with_region(env::var("S3_REGION").unwrap_or_else(|_| "auto".into()))
            .with_allow_http(endpoint.starts_with("http://"))
            .with_virtual_hosted_style_request(false)
            .build()?;
        Ok(Self::S3(Arc::new(store)))
    }

    pub async fn read(&self, key: &str, limit: usize) -> Result<Bytes> {
        match self {
            Self::Local(root) => {
                let path = tokio::fs::canonicalize(root.join(key)).await?;
                ensure!(path.starts_with(root), "Source escapes SOURCE_DIR");
                let file = tokio::fs::File::open(path).await?;
                ensure!(
                    file.metadata().await?.len() <= limit as u64,
                    "Source too large"
                );
                let mut buf = Vec::new();
                file.take(limit as u64 + 1).read_to_end(&mut buf).await?;
                ensure!(buf.len() <= limit, "Source too large");
                Ok(Bytes::from(buf))
            }
            Self::S3(store) => {
                let result = store.get(&Path::from(key)).await?;
                ensure!(result.meta.size <= limit as u64, "Source too large");
                let mut stream = result.into_stream();
                let mut buf = BytesMut::new();
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    ensure!(
                        chunk.len() <= limit.saturating_sub(buf.len()),
                        "Source too large"
                    );
                    buf.extend_from_slice(&chunk);
                }
                Ok(buf.freeze())
            }
        }
    }
}
