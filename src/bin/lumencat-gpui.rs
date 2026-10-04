#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let project = args
        .windows(2)
        .find(|pair| pair[0] == "--project")
        .map(|pair| pair[1].clone())
        .or_else(|| args.first().filter(|arg| !arg.starts_with("--")).cloned());
    lumencat::gpui_app::runtime::run(project);
}
