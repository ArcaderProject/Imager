use std::path::Path;
use std::process::Command;

pub struct WorkerInvocation<'a> {
    pub exe: &'a Path,
    pub source: &'a str,
    pub device: &'a str,
    pub verify: bool,
    pub progress: &'a str,
    pub cancel: &'a str,
}

impl<'a> WorkerInvocation<'a> {
    fn argv(&self) -> Vec<String> {
        vec![
            "flash-worker".into(),
            "--source".into(),
            self.source.into(),
            "--device".into(),
            self.device.into(),
            "--verify".into(),
            if self.verify { "true" } else { "false" }.into(),
            "--progress".into(),
            self.progress.into(),
            "--cancel".into(),
            self.cancel.into(),
        ]
    }
}

pub fn elevated_command(inv: &WorkerInvocation) -> Command {
    let exe = inv.exe.to_string_lossy().to_string();
    let argv = inv.argv();

    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("pkexec");
        match std::env::var("APPIMAGE") {
            Ok(appimage) if !appimage.is_empty() => {
                cmd.arg("env");
                cmd.arg("APPIMAGE_EXTRACT_AND_RUN=1");
                cmd.arg(&appimage);
            }
            _ => {
                cmd.arg(&exe);
            }
        }
        cmd.args(&argv);
        cmd
    }

    #[cfg(target_os = "macos")]
    {
        let mut shell = shell_quote(&exe);
        for a in &argv {
            shell.push(' ');
            shell.push_str(&shell_quote(a));
        }
        let script = format!(
            "do shell script {} with administrator privileges",
            applescript_quote(&shell)
        );
        let mut cmd = Command::new("osascript");
        cmd.arg("-e").arg(script);
        cmd
    }

    #[cfg(target_os = "windows")]
    {
        let arg_list = argv
            .iter()
            .map(|a| format!("'{}'", a.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",");
        let ps = format!(
            "$p = Start-Process -FilePath '{}' -ArgumentList {} -Verb RunAs -Wait -PassThru; exit $p.ExitCode",
            exe.replace('\'', "''"),
            arg_list
        );
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps]);
        cmd
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let mut cmd = Command::new(&exe);
        cmd.args(&argv);
        cmd
    }
}

#[cfg(target_os = "macos")]
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn applescript_quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

pub fn classify_exit(code: Option<i32>, stderr: &str) -> Option<String> {
    match code {
        Some(0) => None,
        Some(3) => Some("Cancelled".into()),
        Some(126) | Some(127) => {
            Some("Administrator permission was denied or dismissed.".into())
        }
        Some(_) | None => {
            let trimmed = stderr.trim();
            if trimmed.is_empty() {
                Some("The privileged writer exited unexpectedly.".into())
            } else {
                Some(trimmed.to_string())
            }
        }
    }
}
