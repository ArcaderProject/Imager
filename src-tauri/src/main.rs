#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(|s| s == "flash-worker").unwrap_or(false) {
        arcader_imager_lib::run_worker(&args[2..]);
    }

    arcader_imager_lib::run()
}
