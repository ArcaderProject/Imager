use std::pin::Pin;
use std::time::{Duration, Instant};

use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type ChunkStream = Pin<Box<dyn Stream<Item = Result<Vec<u8>, String>> + Send>>;

const SECTOR: usize = 4096;

fn is_http_url(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

fn source_open_error(path: &str, e: std::io::Error) -> String {
    #[cfg(target_os = "macos")]
    {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            return format!(
                "Cannot read the image at {path}: permission denied. macOS may be \
                 blocking access to a protected folder - move the image to a plain \
                 location (for example your home folder or /tmp) and try again."
            );
        }
    }
    format!("cannot open image {path}: {e}")
}

struct SectorWriter {
    file: tokio::fs::File,
    pending: Vec<u8>,
}

impl SectorWriter {
    fn new(file: tokio::fs::File) -> Self {
        Self {
            file,
            pending: Vec::with_capacity(SECTOR * 256),
        }
    }

    async fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.pending.extend_from_slice(data);
        let flush_len = (self.pending.len() / SECTOR) * SECTOR;
        if flush_len > 0 {
            self.file.write_all(&self.pending[..flush_len]).await?;
            self.pending.drain(..flush_len);
        }
        Ok(())
    }

    async fn finish(mut self) -> std::io::Result<()> {
        if !self.pending.is_empty() {
            let pad = (SECTOR - self.pending.len() % SECTOR) % SECTOR;
            self.pending.resize(self.pending.len() + pad, 0);
            self.file.write_all(&self.pending).await?;
        }
        self.file.flush().await?;
        self.file.sync_all().await?;
        Ok(())
    }
}

async fn open_http_source(url: &str) -> Result<(ChunkStream, u64), String> {
    let client = reqwest::Client::builder()
        .user_agent("ArcaderImager/1.0")
        .build()
        .map_err(|e| format!("http client error: {e}"))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("download failed: {e}"))?;
    let total = resp.content_length().unwrap_or(0);
    let stream = resp
        .bytes_stream()
        .map(|r| r.map(|b| b.to_vec()).map_err(|e| format!("network error: {e}")))
        .boxed();
    Ok((stream, total))
}

async fn open_file_source(path: &str) -> Result<(ChunkStream, u64), String> {
    let path = path.strip_prefix("file://").unwrap_or(path).to_string();
    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|e| source_open_error(&path, e))?;
    if !meta.is_file() {
        return Err(format!("not a file: {path}"));
    }
    let total = meta.len();
    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|e| source_open_error(&path, e))?;
    let stream = futures_util::stream::try_unfold(file, |mut f| async move {
        let mut buf = vec![0u8; 4 * 1024 * 1024];
        let n = f
            .read(&mut buf)
            .await
            .map_err(|e| format!("image read error: {e}"))?;
        if n == 0 {
            Ok::<_, String>(None)
        } else {
            buf.truncate(n);
            Ok(Some((buf, f)))
        }
    })
    .boxed();
    Ok((stream, total))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Progress {
    pub phase: String,
    pub done: u64,
    pub total: u64,
    pub rate: f64,
}

impl Progress {
    fn new(phase: &str, done: u64, total: u64, rate: f64) -> Self {
        Progress {
            phase: phase.to_string(),
            done,
            total,
            rate,
        }
    }
}

fn unmount_target(device: &str) {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Ok(out) = Command::new("lsblk")
            .args(["-ln", "-o", "PATH", device])
            .output()
        {
            for part in String::from_utf8_lossy(&out.stdout).lines() {
                let part = part.trim();
                if part.is_empty() || part == device {
                    continue;
                }
                let _ = Command::new("umount").arg(part).output();
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let whole = device.replace("/dev/rdisk", "/dev/disk");
        let _ = Command::new("diskutil")
            .args(["unmountDisk", &whole])
            .output();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = device;
    }
}

pub async fn run<P, C>(
    source: String,
    device: String,
    verify: bool,
    mut on_progress: P,
    is_cancelled: C,
) -> Result<(), String>
where
    P: FnMut(&Progress),
    C: Fn() -> bool,
{
    on_progress(&Progress::new("prepare", 0, 0, 0.0));

    unmount_target(&device);

    let from_network = is_http_url(&source);
    let (mut stream, total) = if from_network {
        open_http_source(&source).await?
    } else {
        open_file_source(&source).await?
    };

    let phase = if from_network { "download" } else { "write" };
    on_progress(&Progress::new(phase, 0, total, 0.0));

    let device_file = open_device_write(&device)
        .await
        .map_err(|e| format!("cannot open {device}: {e}"))?;
    let mut writer = SectorWriter::new(device_file);

    let mut hasher = Sha256::new();
    let mut written: u64 = 0;
    let started = Instant::now();
    let mut last_emit = Instant::now();

    while let Some(chunk) = stream.next().await {
        if is_cancelled() {
            return Err("Cancelled".into());
        }
        let chunk = chunk?;
        writer
            .write(&chunk)
            .await
            .map_err(|e| format!("write error: {e}"))?;
        hasher.update(&chunk);
        written += chunk.len() as u64;

        if last_emit.elapsed() >= Duration::from_millis(200) {
            let rate = written as f64 / started.elapsed().as_secs_f64().max(0.001);
            on_progress(&Progress::new(phase, written, total.max(written), rate));
            last_emit = Instant::now();
        }
    }

    on_progress(&Progress::new("finalize", written, total.max(written), 0.0));
    writer
        .finish()
        .await
        .map_err(|e| format!("finalize error: {e}"))?;

    let download_digest = hasher.finalize();
    let image_size = written;

    if verify {
        if is_cancelled() {
            return Err("Cancelled".into());
        }
        let mut rfile = open_device_read(&device)
            .await
            .map_err(|e| format!("cannot reopen {device} to verify: {e}"))?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 4 * 1024 * 1024];
        let mut read_total: u64 = 0;
        let started = Instant::now();
        let mut last_emit = Instant::now();
        while read_total < image_size {
            if is_cancelled() {
                return Err("Cancelled".into());
            }
            let remaining = (image_size - read_total) as usize;
            let want = remaining.div_ceil(SECTOR).saturating_mul(SECTOR).min(buf.len());
            let n = rfile
                .read(&mut buf[..want])
                .await
                .map_err(|e| format!("verify read error: {e}"))?;
            if n == 0 {
                break;
            }
            let take = (n as u64).min(image_size - read_total) as usize;
            hasher.update(&buf[..take]);
            read_total += take as u64;
            if last_emit.elapsed() >= Duration::from_millis(200) {
                let rate = read_total as f64 / started.elapsed().as_secs_f64().max(0.001);
                on_progress(&Progress::new("verify", read_total, image_size, rate));
                last_emit = Instant::now();
            }
        }
        if hasher.finalize() != download_digest {
            return Err(
                "Verification failed - the data on the device does not match the downloaded image."
                    .into(),
            );
        }
        on_progress(&Progress::new("verify", image_size, image_size, 0.0));
    }

    Ok(())
}

#[cfg(unix)]
async fn open_device_write(device: &str) -> std::io::Result<tokio::fs::File> {
    tokio::fs::OpenOptions::new()
        .write(true)
        .open(device)
        .await
}

#[cfg(unix)]
async fn open_device_read(device: &str) -> std::io::Result<tokio::fs::File> {
    tokio::fs::OpenOptions::new().read(true).open(device).await
}

#[cfg(windows)]
async fn open_device_write(device: &str) -> std::io::Result<tokio::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    tokio::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .share_mode(0x00000001 | 0x00000002)
        .open(device)
        .await
}

#[cfg(windows)]
async fn open_device_read(device: &str) -> std::io::Result<tokio::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    tokio::fs::OpenOptions::new()
        .read(true)
        .share_mode(0x00000001 | 0x00000002)
        .open(device)
        .await
}
