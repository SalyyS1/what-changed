// Release builds must not attach a console: this app starts from the tray and a
// flashing console window would be the first thing a non-technical user sees.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    what_changed_lib::run();
}
