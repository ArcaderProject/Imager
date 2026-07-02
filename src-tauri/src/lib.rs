mod drives;
mod elevate;
mod flash;
mod worker;

use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use drives::Drive;
use elevate::{classify_exit, elevated_command, WorkerInvocation};

pub use worker::run as run_worker;

#[derive(Default)]
pub struct FlashState {
    cancel_file: Mutex<Option<PathBuf>>,
}

#[derive(Serialize, Clone)]
struct OsBuild {
    arch: String,
    label: String,
    url: String,
}

#[derive(Serialize, Clone)]
struct OsInfo {
    name: String,
    version: String,
    builds: Vec<OsBuild>,
}

const OS_RELEASES_API: &str = "https://api.github.com/repos/ArcaderProject/System/releases/latest";
const OS_ASSETS: &[(&str, &str, &str)] = &[
    ("amd64", "64-bit", "arcader-kiosk-amd64.iso"),
    ("i386", "32-bit", "arcader-kiosk-i386.iso"),
];

#[tauri::command]
async fn get_os_info() -> Result<OsInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent("ArcaderImager/1.0")
        .build()
        .map_err(|e| format!("http client error: {e}"))?;

    let body = client
        .get(OS_RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("cannot reach GitHub: {e}"))?
        .error_for_status()
        .map_err(|e| format!("GitHub returned an error: {e}"))?
        .text()
        .await
        .map_err(|e| format!("cannot read release: {e}"))?;

    let release: Value =
        serde_json::from_str(&body).map_err(|e| format!("bad release data: {e}"))?;

    let version = release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or("latest release has no tag_name")?
        .to_string();

    let assets = release
        .get("assets")
        .and_then(|a| a.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[]);

    let asset_url = |name: &str| -> Option<String> {
        assets
            .iter()
            .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(name))
            .and_then(|a| a.get("browser_download_url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
    };

    let builds: Vec<OsBuild> = OS_ASSETS
        .iter()
        .filter_map(|(arch, label, name)| {
            asset_url(name).map(|url| OsBuild {
                arch: arch.to_string(),
                label: label.to_string(),
                url,
            })
        })
        .collect();

    if builds.is_empty() {
        return Err(format!("release {version} has no Arcader kiosk ISO assets"));
    }

    Ok(OsInfo {
        name: "Arcader OS".into(),
        version,
        builds,
    })
}

#[tauri::command]
fn list_drives() -> Result<Vec<Drive>, String> {
    drives::list_drives()
}

#[derive(Serialize)]
struct LocalImage {
    path: String,
    name: String,
    size: u64,
}

#[tauri::command]
async fn pick_image_file(app: AppHandle) -> Option<LocalImage> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose a disk image")
        .add_filter("Disk images", &["iso", "img", "raw", "bin"])
        .add_filter("All files", &["*"])
        .pick_file(move |f| {
            let _ = tx.send(f);
        });

    let path = rx.await.ok().flatten()?.into_path().ok()?;
    let meta = std::fs::metadata(&path).ok()?;
    Some(LocalImage {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        size: meta.len(),
        path: path.to_string_lossy().to_string(),
    })
}

fn unique_stamp() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}", std::process::id(), nanos)
}

fn last_nonempty_line(content: &str) -> Option<&str> {
    content.lines().rev().find(|l| !l.trim().is_empty())
}

fn read_last_json(path: &PathBuf) -> Option<Value> {
    let content = fs::read_to_string(path).ok()?;
    last_nonempty_line(&content).and_then(|l| serde_json::from_str::<Value>(l).ok())
}

#[tauri::command]
async fn start_flash(
    app: AppHandle,
    state: State<'_, FlashState>,
    source: String,
    device: String,
    verify: bool,
) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot locate self: {e}"))?;
    let stamp = unique_stamp();

    let dir = std::env::temp_dir().join(format!("arcader-imager-{stamp}"));
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create work dir: {e}"))?;
    let progress = dir.join("progress");
    let cancel = dir.join("cancel");

    fs::write(&progress, b"").map_err(|e| format!("cannot create progress file: {e}"))?;

    *state.cancel_file.lock().unwrap() = Some(cancel.clone());

    let progress_str = progress.to_string_lossy().to_string();
    let cancel_str = cancel.to_string_lossy().to_string();

    let mut cmd = elevated_command(&WorkerInvocation {
        exe: &exe,
        source: &source,
        device: &device,
        verify,
        progress: &progress_str,
        cancel: &cancel_str,
    });
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    let child = tokio::task::spawn_blocking(move || cmd.output());

    let mut last_emitted = String::new();
    loop {
        if let Ok(content) = fs::read_to_string(&progress) {
            if let Some(line) = last_nonempty_line(&content) {
                if line != last_emitted {
                    if let Ok(p) = serde_json::from_str::<flash::Progress>(line) {
                        let _ = app.emit("flash://progress", p);
                    }
                    last_emitted = line.to_string();
                }
            }
        }
        if child.is_finished() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(120)).await;
    }

    let output = child
        .await
        .map_err(|e| format!("worker join error: {e}"))?
        .map_err(|e| format!("could not start privileged writer: {e} (is pkexec/osascript available?)"))?;

    let terminal = read_last_json(&progress);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let _ = fs::remove_dir_all(&dir);
    *state.cancel_file.lock().unwrap() = None;

    if let Some(v) = terminal {
        match v.get("phase").and_then(|p| p.as_str()) {
            Some("complete") if output.status.success() => return Ok(()),
            Some("failed") => {
                let msg = v
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Write failed")
                    .to_string();
                return Err(msg);
            }
            _ => {}
        }
    }

    match classify_exit(output.status.code(), &stderr) {
        None => Ok(()),
        Some(msg) => Err(msg),
    }
}

#[tauri::command]
fn cancel_flash(state: State<'_, FlashState>) {
    if let Some(path) = state.cancel_file.lock().unwrap().clone() {
        let _ = fs::write(&path, b"cancel");
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(FlashState::default())
        .invoke_handler(tauri::generate_handler![
            get_os_info,
            list_drives,
            pick_image_file,
            start_flash,
            cancel_flash
        ])
        .run(tauri::generate_context!())
        .expect("error while running Arcader Imager");
}
