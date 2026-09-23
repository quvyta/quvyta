//! quvyta: lists the Quvyta family of terminal applications, installs them, opens the members
//! that are installed, and updates and removes them.
//!
//! The program is a thin start over this library: [`cli`] reads its arguments and [`Quvyta`]
//! is the screen, run by the framework's runtime with [`LOCALES`] and [`KEYS`]. Everything the
//! screen touches outside itself comes from a [`Machine`], so the screen can be shown on a
//! machine rooted in any folder ([`Machine::in_root`]) and told what cargo installed and what
//! its installs print through [`Msg`], the way its own background tasks tell it.

mod app;
mod cargo;
mod checks;
pub mod cli;
mod family;
mod install;
mod inventory;
mod launcher;
mod machine;
mod shell_path;
pub mod updates;

pub use app::{Change, Following, InstallMsg, MemberFollowing, Msg, PathMsg, Quvyta, SettingMsg, Tab, UpdateMsg};
pub use cargo::Installed;
pub use family::{FAMILY, Member, Status};
pub use install::Outcome;
pub use inventory::Inventory;
pub use launcher::AfterClose;
pub use machine::Machine;

/// The language files, compiled in so an installed binary needs nothing beside it: file name
/// and text, each handed to the runtime as a locale source.
pub const LOCALES: [(&str, &str); 9] = [
    ("en.toml", include_str!("../assets/locales/en.toml")),
    ("tr.toml", include_str!("../assets/locales/tr.toml")),
    ("de.toml", include_str!("../assets/locales/de.toml")),
    ("es.toml", include_str!("../assets/locales/es.toml")),
    ("fr.toml", include_str!("../assets/locales/fr.toml")),
    ("pt-BR.toml", include_str!("../assets/locales/pt-BR.toml")),
    ("ru.toml", include_str!("../assets/locales/ru.toml")),
    ("zh-Hans.toml", include_str!("../assets/locales/zh-Hans.toml")),
    ("ja.toml", include_str!("../assets/locales/ja.toml")),
];

/// The application's own keys, layered over the framework's as the keymap source `keymap.toml`.
pub const KEYS: &str = "[app]\nprimary = \"enter\"\nback = \"esc\"\nrefresh = \"r\"\nupdate = \"u\"\n";
