//! The static tree: an http(s) base URL (Range GETs) or a local directory
//! (slice reads). Any host that serves bytes and honours HTTP Range works —
//! S3, R2, GitHub Pages, nginx, a CDN, or a local folder for offline use.

use anyhow::{Context, Result};
use std::io::Read as _;
use std::path::PathBuf;

pub enum StaticSource {
    Http { base: String, agent: ureq::Agent },
    Dir(PathBuf),
}

impl StaticSource {
    /// A convenience constructor with a default ureq agent.
    pub fn new(base: &str) -> Self {
        Self::with_agent(base, ureq::Agent::new())
    }

    /// Interpret `base` as an http(s) URL or a filesystem path.
    pub fn with_agent(base: &str, agent: ureq::Agent) -> Self {
        if base.starts_with("http://") || base.starts_with("https://") {
            StaticSource::Http {
                base: base.trim_end_matches('/').to_string(),
                agent,
            }
        } else {
            StaticSource::Dir(PathBuf::from(base))
        }
    }

    /// Like [`Self::get_all`], but a definitive "not there" (HTTP 404/410,
    /// or a missing file) is `Ok(None)` instead of an error, so callers can
    /// negative-cache absence without conflating it with transport failures.
    pub(crate) fn get_all_opt(&self, rel: &str) -> Result<Option<Vec<u8>>> {
        match self {
            StaticSource::Http { base, agent } => {
                let url = format!("{base}/{rel}");
                match agent.get(&url).call() {
                    Ok(resp) => {
                        let mut out = Vec::new();
                        resp.into_reader()
                            .read_to_end(&mut out)
                            .with_context(|| format!("reading {url}"))?;
                        Ok(Some(out))
                    }
                    Err(ureq::Error::Status(404 | 410, _)) => Ok(None),
                    Err(e) => Err(anyhow::anyhow!("GET {url}: {e}")),
                }
            }
            StaticSource::Dir(root) => {
                let path = root.join(rel);
                match std::fs::read(&path) {
                    Ok(bytes) => Ok(Some(bytes)),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(e) => {
                        Err(anyhow::Error::new(e).context(format!("reading {}", path.display())))
                    }
                }
            }
        }
    }

    pub(crate) fn get_all(&self, rel: &str) -> Result<Vec<u8>> {
        match self {
            StaticSource::Http { base, agent } => {
                let url = format!("{base}/{rel}");
                let resp = agent
                    .get(&url)
                    .call()
                    .map_err(|e| anyhow::anyhow!("GET {url}: {e}"))?;
                let mut out = Vec::new();
                resp.into_reader()
                    .read_to_end(&mut out)
                    .with_context(|| format!("reading {url}"))?;
                Ok(out)
            }
            StaticSource::Dir(root) => {
                let path = root.join(rel);
                std::fs::read(&path).with_context(|| format!("reading {}", path.display()))
            }
        }
    }

    pub(crate) fn get_range(&self, rel: &str, offset: u64, len: u64) -> Result<Vec<u8>> {
        match self {
            StaticSource::Http { base, agent } => {
                let url = format!("{base}/{rel}");
                let end = offset + len - 1;
                let range = format!("bytes={offset}-{end}");
                let resp = agent
                    .get(&url)
                    .set("range", &range)
                    .call()
                    .map_err(|e| anyhow::anyhow!("GET {url} [{range}]: {e}"))?;
                let mut out = Vec::with_capacity(len as usize);
                resp.into_reader()
                    .read_to_end(&mut out)
                    .with_context(|| format!("reading range of {url}"))?;
                // A host that ignored Range returns the whole object; slice.
                if out.len() as u64 > len {
                    let start = offset as usize;
                    let stop = start + len as usize;
                    if stop <= out.len() {
                        return Ok(out[start..stop].to_vec());
                    }
                }
                Ok(out)
            }
            StaticSource::Dir(root) => {
                use std::io::{Seek, SeekFrom};
                let path = root.join(rel);
                let mut f = std::fs::File::open(&path)
                    .with_context(|| format!("opening {}", path.display()))?;
                f.seek(SeekFrom::Start(offset))?;
                let mut out = vec![0u8; len as usize];
                f.read_exact(&mut out)
                    .with_context(|| format!("reading range of {}", path.display()))?;
                Ok(out)
            }
        }
    }
}
