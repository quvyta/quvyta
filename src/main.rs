//! quvyta: lists the Quvyta family of terminal applications, installs them, opens the members
//! that are installed, and updates and removes them.

mod app;
mod cargo;
mod checks;
mod family;
mod install;
mod inventory;
mod launcher;
mod machine;
mod shell_path;
mod updates;

use qframe::runtime::Runtime;

/// The language files, compiled in so an installed binary needs nothing beside it.
const LOCALES: [(&str, &str); 2] =
    [("en.toml", include_str!("../assets/locales/en.toml")), ("tr.toml", include_str!("../assets/locales/tr.toml"))];

/// The application's own keys, layered over the framework's.
const KEYS: &str = "[app]\nprimary = \"enter\"\nback = \"esc\"\nrefresh = \"r\"\nupdate = \"u\"\n";

fn main() -> std::io::Result<()> {
    let app = app::Quvyta::new(machine::Machine::from_env());
    LOCALES
        .iter()
        .fold(Runtime::new(app), |runtime, (file, text)| runtime.locale_source(*file, *text))
        .keymap_source("keymap.toml", KEYS)
        .run()
}
