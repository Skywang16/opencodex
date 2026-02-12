// Prevent additional console window in Windows release builds, do not delete!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    terminal_lib::run()
}
