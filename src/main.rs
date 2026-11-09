#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod calc;
mod i18n;
mod pubchem;
mod settings;
#[cfg(windows)]
mod substance_editor;
#[cfg(windows)]
mod substance_picker;
mod substances;
#[cfg(windows)]
mod theme;
#[cfg(windows)]
mod ui;
mod user_db;

fn main() {
    #[cfg(windows)]
    ui::run();
    #[cfg(not(windows))]
    eprintln!("Solcalc's native interface requires Windows 10 or newer.");
}
