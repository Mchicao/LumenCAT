#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    lumencat::gpui_app::runtime::run(std::env::args().nth(1));
}
