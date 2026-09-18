//! quvyta's own settings, in `launcher.conf`:
//!
//! ```toml
//! # What happens when a member opened from quvyta closes: "return" to quvyta, or "shell" to
//! # quit quvyta too and go back to the shell it was started from.
//! after_close = "return"
//! # Whether quvyta offers to put cargo's folder on PATH: "ask", or "dismissed" once the user
//! # said not now.
//! path_prompt = "ask"
//! # Whether quvyta asks crates.io for newer versions when it starts. `r` asks at any time.
//! check_updates = true
//! ```

use std::io;
use std::path::Path;

use qframe::diagnostics::Diagnostic;
use qframe::storage::{Schema, Settings};

/// The key saying what happens after a member closes.
const AFTER_CLOSE: &str = "after_close";
/// The key saying whether quvyta offers to put cargo's folder on `PATH`.
const PATH_PROMPT: &str = "path_prompt";
/// The key saying whether quvyta looks for updates when it starts.
const CHECK_UPDATES: &str = "check_updates";

/// What happens when a member opened from quvyta closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AfterClose {
    /// quvyta comes back.
    #[default]
    Return,
    /// quvyta quits too, back to the shell it was started from.
    Shell,
}

/// Whether quvyta offers to put cargo's folder on `PATH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathPrompt {
    /// It offers, at start and after an install.
    #[default]
    Ask,
    /// The user said not now; the offer is not made again.
    Dismissed,
}

/// The settings of `launcher.conf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launcher {
    /// What happens when a member closes.
    pub after_close: AfterClose,
    /// Whether the PATH offer is made.
    pub path_prompt: PathPrompt,
    /// Whether crates.io is asked for newer versions at start.
    pub check_updates: bool,
    /// What was wrong in the file, with the line and column; the defaults stand in for it.
    pub diagnostics: Vec<Diagnostic>,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            after_close: AfterClose::default(),
            path_prompt: PathPrompt::default(),
            check_updates: true,
            diagnostics: Vec::new(),
        }
    }
}

impl Launcher {
    /// Reads `path`. A missing file, or no settings folder at all, gives the defaults; a broken
    /// one gives the defaults for what is broken and says what is wrong.
    pub fn load(path: Option<&Path>) -> Self {
        let Some(path) = path else { return Self::default() };
        let settings = Settings::open(path).schema(schema());
        let after_close = match settings.get::<String>(AFTER_CLOSE).as_deref() {
            Some("shell") => AfterClose::Shell,
            _ => AfterClose::Return,
        };
        let path_prompt = match settings.get::<String>(PATH_PROMPT).as_deref() {
            Some("dismissed") => PathPrompt::Dismissed,
            _ => PathPrompt::Ask,
        };
        let check_updates = settings.get::<bool>(CHECK_UPDATES).unwrap_or(true);
        Self { after_close, path_prompt, check_updates, diagnostics: settings.diagnostics().to_vec() }
    }

    /// Writes `path_prompt = "dismissed"` to `path`, keeping every other setting as the file has
    /// it now. The file is read again rather than taken from what quvyta loaded at start, so a
    /// change made meanwhile is not lost; it is replaced in one step, never left half written.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file or its folder cannot be written.
    pub fn dismiss_path_prompt(path: &Path) -> io::Result<()> {
        let mut settings = Settings::open(path).schema(schema());
        settings.set(PATH_PROMPT, "dismissed".to_owned());
        settings.save()
    }
}

/// Every key quvyta reads and what it accepts.
fn schema() -> Schema {
    Schema::default()
        .choice(AFTER_CLOSE, ["return", "shell"], "return")
        .choice(PATH_PROMPT, ["ask", "dismissed"], "ask")
        .flag(CHECK_UPDATES, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(text: &str) -> Launcher {
        let root = tempfile::tempdir().expect("temp");
        let path = root.path().join("launcher.conf");
        std::fs::write(&path, text).expect("settings");
        Launcher::load(Some(&path))
    }

    #[test]
    fn without_a_file_quvyta_comes_back() {
        let root = tempfile::tempdir().expect("temp");
        assert_eq!(Launcher::load(Some(&root.path().join("launcher.conf"))), Launcher::default());
        assert_eq!(Launcher::load(None), Launcher::default());
        assert_eq!(Launcher::default().after_close, AfterClose::Return);
    }

    #[test]
    fn both_choices_are_read() {
        assert_eq!(load("after_close = \"shell\"\n").after_close, AfterClose::Shell);
        let back = load("after_close = \"return\"\n");
        assert_eq!(back.after_close, AfterClose::Return);
        assert_eq!(back.diagnostics, []);
    }

    #[test]
    fn an_unknown_value_falls_back_and_says_where() {
        let launcher = load("\nafter_close = \"exit\"\n");
        assert_eq!(launcher.after_close, AfterClose::Return);
        let [diagnostic] = launcher.diagnostics.as_slice() else {
            panic!("one problem: {:?}", launcher.diagnostics);
        };
        assert!(diagnostic.to_string().starts_with("launcher.conf:2:"), "{diagnostic}");
    }

    #[test]
    fn dismissing_the_path_prompt_keeps_the_other_settings() {
        let root = tempfile::tempdir().expect("temp");
        let path = root.path().join("launcher.conf");
        std::fs::write(&path, "after_close = \"shell\"\n").expect("settings");
        Launcher::dismiss_path_prompt(&path).expect("saved");
        let launcher = Launcher::load(Some(&path));
        assert_eq!(launcher.path_prompt, PathPrompt::Dismissed);
        assert_eq!(launcher.after_close, AfterClose::Shell);
        assert_eq!(launcher.diagnostics, []);
    }

    #[test]
    fn dismissing_creates_the_file_and_its_folder() {
        let root = tempfile::tempdir().expect("temp");
        let path = root.path().join("quvyta/launcher.conf");
        Launcher::dismiss_path_prompt(&path).expect("saved");
        assert_eq!(std::fs::read_to_string(&path).expect("written"), "path_prompt = \"dismissed\"\n");
        assert_eq!(load("path_prompt = \"ask\"\n").path_prompt, PathPrompt::Ask);
    }

    #[test]
    fn updates_are_checked_unless_turned_off() {
        assert!(Launcher::default().check_updates);
        assert!(load("after_close = \"shell\"\n").check_updates);
        let off = load("check_updates = false\n");
        assert_eq!((off.check_updates, off.diagnostics.as_slice()), (false, &[][..]));
        let broken = load("check_updates = \"no\"\n");
        assert!(broken.check_updates, "a value that is not true or false falls back to checking");
        assert!(broken.diagnostics[0].to_string().starts_with("launcher.conf:1:"), "{:?}", broken.diagnostics);
    }

    #[test]
    fn dismissing_keeps_updates_turned_off() {
        let root = tempfile::tempdir().expect("temp");
        let path = root.path().join("launcher.conf");
        std::fs::write(&path, "check_updates = false\n").expect("settings");
        Launcher::dismiss_path_prompt(&path).expect("saved");
        assert!(!Launcher::load(Some(&path)).check_updates);
    }

    #[test]
    fn a_broken_file_does_not_stop_anything() {
        let launcher = load("after_close = \nafter_close = 3\n[[");
        assert_eq!(launcher.after_close, AfterClose::Return);
        assert!(!launcher.diagnostics.is_empty());
    }
}
