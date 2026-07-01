use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

struct WorkerArgs {
    source: String,
    device: String,
    verify: bool,
    progress: PathBuf,
    cancel: PathBuf,
}

fn parse(args: &[String]) -> Result<WorkerArgs, String> {
    let mut source = None;
    let mut device = None;
    let mut verify = false;
    let mut progress = None;
    let mut cancel = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--source" => {
                source = args.get(i + 1).cloned();
                i += 2;
            }
            "--device" => {
                device = args.get(i + 1).cloned();
                i += 2;
            }
            "--verify" => {
                verify = args.get(i + 1).map(|v| v == "true").unwrap_or(false);
                i += 2;
            }
            "--progress" => {
                progress = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--cancel" => {
                cancel = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            _ => i += 1,
        }
    }
    Ok(WorkerArgs {
        source: source.ok_or("missing --source")?,
        device: device.ok_or("missing --device")?,
        verify,
        progress: progress.ok_or("missing --progress")?,
        cancel: cancel.ok_or("missing --cancel")?,
    })
}

fn append_line(path: &Path, line: &str) {
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
        let _ = f.write_all(b"\n");
        let _ = f.flush();
    }
}

pub fn run(args: &[String]) -> ! {
    let parsed = match parse(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("worker: {e}");
            std::process::exit(2);
        }
    };

    let progress_path = parsed.progress.clone();
    let cancel_path = parsed.cancel.clone();

    let sink = Mutex::new(());
    let on_progress = |p: &crate::flash::Progress| {
        let _guard = sink.lock();
        if let Ok(json) = serde_json::to_string(p) {
            append_line(&progress_path, &json);
        }
    };
    let is_cancelled = || cancel_path.exists();

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            append_line(
                &parsed.progress,
                &format!("{{\"phase\":\"failed\",\"message\":\"runtime: {e}\"}}"),
            );
            std::process::exit(1);
        }
    };

    let result = runtime.block_on(crate::flash::run(
        parsed.source,
        parsed.device,
        parsed.verify,
        on_progress,
        is_cancelled,
    ));

    match result {
        Ok(()) => {
            append_line(&parsed.progress, "{\"phase\":\"complete\"}");
            std::process::exit(0);
        }
        Err(msg) => {
            let payload = serde_json::json!({ "phase": "failed", "message": msg });
            append_line(&parsed.progress, &payload.to_string());
            if msg == "Cancelled" {
                std::process::exit(3);
            }
            std::process::exit(1);
        }
    }
}
