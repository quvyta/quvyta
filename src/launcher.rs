//! quvyta's own settings, in `launcher.conf`:
//!
//! ```toml
//! # What happens when a member opened from quvyta closes: "return" to quvyta, or "shell" to
//! # quit quvyta too and go back to the shell it was started from.
//! after_close = "return"
//! # Whether quvyta offers to put cargo's folder on PATH: "ask", or "dismissed" once the user
//! # said not now.
//! path_prompt = "ask"
//! ```
//!
//! Whether quvyta asks crates.io for newer versions when it starts is not kept here: it is the
//! family's update notice, one switch in the shared file for every Quvyta application. An older
//! `check_updates` line is handed over to it once, by [`hand_over_check_updates`].
//!
//! The appearance of quvyta itself is not read here: `language`, `theme` and `icons` are the
//! family's shared keys, resolved for every member alike by the framework, and the Settings tab
//! writes them where the box under each row says. Written in this file they hold for quvyta
//! alone; written as `"quvyta"` they follow the family's shared file.

use std::io;
use std::path::Path;

use qframe::diagnostics::Diagnostic;
use qframe::storage::{Family, Schema, Setting, Settings};

/// The key saying what happens after a member closes.
const AFTER_CLOSE: &str = "after_close";
/// The key saying whether quvyta offers to put cargo's folder on `PATH`.
const PATH_PROMPT: &str = "path_prompt";
/// The key that said whether quvyta looked for updates when it started, before the family had
/// one switch for that; still known, so a file holding it is not a broken file.
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Launcher {
    /// What happens when a member closes.
    pub after_close: AfterClose,
    /// Whether the PATH offer is made.
    pub path_prompt: PathPrompt,
    /// What was wrong in the file, with the line and column; the defaults stand in for it.
    pub diagnostics: Vec<Diagnostic>,
}

impl Launcher {
    /// quvyta's own settings as they stand in `settings`, which [`open`] read from
    /// `launcher.conf`: a missing file, or no settings folder at all, gives the defaults, and a
    /// broken one gives the defaults for what is broken and says what is wrong.
    ///
    /// The shared keys of the family are not here: they are resolved together with every other
    /// member's, from the shared file.
    pub(crate) fn from_settings(settings: &Settings) -> Self {
        let after_close = match settings.get::<String>(AFTER_CLOSE).as_deref() {
            Some("shell") => AfterClose::Shell,
            _ => AfterClose::Return,
        };
        let path_prompt = match settings.get::<String>(PATH_PROMPT).as_deref() {
            Some("dismissed") => PathPrompt::Dismissed,
            _ => PathPrompt::Ask,
        };
        Self { after_close, path_prompt, diagnostics: settings.diagnostics().to_vec() }
    }

    /// Writes `path_prompt = "dismissed"` to `path`, keeping every other setting as the file has
    /// it now. The file is read again rather than taken from what quvyta loaded at start, so a
    /// change made meanwhile is not lost; it is replaced in one step, never left half written.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file or its folder cannot be written.
    pub fn dismiss_path_prompt(path: &Path) -> io::Result<()> {
        write(path, PATH_PROMPT, "dismissed".to_owned())
    }

    /// Puts quvyta's own defaults into `settings`, for the first start: the setup wizard has just
    /// made `launcher.conf` with the family's shared keys, and these are the keys that are
    /// quvyta's own. `path_prompt` is left out: not having answered is its default, and the offer
    /// is still to be made.
    pub(crate) fn write_defaults(settings: &mut Settings) {
        let defaults = Self::default();
        settings.set(AFTER_CLOSE, after_close_value(defaults.after_close).to_owned());
    }

    /// Writes `after_close` to `path`, keeping every other setting as the file has it now.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file or its folder cannot be written.
    pub fn save_after_close(path: &Path, after_close: AfterClose) -> io::Result<()> {
        write(path, AFTER_CLOSE, after_close_value(after_close).to_owned())
    }
}

#[cfg(test)]
impl Launcher {
    /// The settings at `path`, read as quvyta reads them when it starts. The application reads
    /// the file once, into the [`Settings`] the appearance rows write too, so only a test reads a
    /// file of its own.
    pub(crate) fn load(path: Option<&Path>) -> Self {
        Self::from_settings(&open(path))
    }
}

/// The value `after_close` is written as.
fn after_close_value(after_close: AfterClose) -> &'static str {
    match after_close {
        AfterClose::Return => "return",
        AfterClose::Shell => "shell",
    }
}

/// quvyta's own settings, read from `launcher.conf` at `path`, or kept in memory when the
/// platform names no settings folder.
///
/// They are marked a member of the family, so `"quvyta"` under `language`, `theme` or `icons`
/// means "follow the family's shared value" instead of being an unknown theme or language.
pub(crate) fn open(path: Option<&Path>) -> Settings {
    let settings = match path {
        Some(path) => Settings::open(path),
        None => Settings::in_memory(),
    };
    settings.member_of(&Family::QUVYTA).schema(schema())
}

/// Hands a `check_updates` line of `launcher.conf` at `path` over to the family's update notice
/// in `family_folder`, and takes the line out.
///
/// Until 0.2.9 quvyta had a switch of its own for asking crates.io at start; now every Quvyta
/// application reads the family's one switch. Someone who turned quvyta's off asked for no
/// question at start, so `false` turns the family's off too: the quieter choice is kept, and it
/// can be turned back on in any member's settings. `true` was the default and says nothing, so
/// the family's switch is left as it is. A broken file is left alone, as quvyta never rewrites a
/// file it could not read whole; so is a file without the line.
///
/// # Errors
///
/// Returns the I/O error when either file cannot be written; the line then stays for the next
/// start to try again.
pub(crate) fn hand_over_check_updates(path: &Path, family_folder: &Path) -> io::Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let mut settings = Settings::open(path).member_of(&Family::QUVYTA).schema(schema());
    if !settings.diagnostics().is_empty() || !settings.keys().any(|key| key == CHECK_UPDATES) {
        return Ok(());
    }
    if settings.get::<bool>(CHECK_UPDATES) == Some(false) {
        Family::QUVYTA.set_update_notice_in(family_folder, false)?;
    }
    settings.remove(CHECK_UPDATES);
    settings.save()
}

/// Sets `key` in the file at `path` and writes it back. The file is read again rather than taken
/// from what quvyta loaded at start, so a change made meanwhile is not lost; it is replaced in one
/// step, never left half written.
fn write(path: &Path, key: &str, value: impl Setting) -> io::Result<()> {
    let mut settings = Settings::open(path).member_of(&Family::QUVYTA).schema(schema());
    settings.set(key, value);
    settings.save()
}

/// Every key quvyta reads and what it accepts, over the framework's own: the appearance rows of
/// the Settings tab write the family's shared keys, reduced motion and the pillar into this same
/// file, so they are known settings here too.
fn schema() -> Schema {
    Schema::builtin()
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
    fn an_old_check_updates_line_is_still_a_known_setting() {
        let old = load("check_updates = false\nafter_close = \"shell\"\n");
        assert_eq!((old.after_close, old.diagnostics.as_slice()), (AfterClose::Shell, &[][..]));
    }

    /// A family folder of the test's own with `launcher.conf` holding `text`.
    fn handed(text: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let root = tempfile::tempdir().expect("temp");
        let path = root.path().join("launcher.conf");
        std::fs::write(&path, text).expect("settings");
        (root, path)
    }

    #[test]
    fn quvyta_s_old_switch_turned_off_turns_the_family_s_notice_off_and_goes() {
        let (root, path) = handed("path_prompt = \"dismissed\"\ncheck_updates = false\nafter_close = \"shell\"\n");
        assert!(Family::QUVYTA.update_notice_in(root.path()), "on until someone turns it off");
        hand_over_check_updates(&path, root.path()).expect("handed over");
        assert!(!Family::QUVYTA.update_notice_in(root.path()), "the quieter choice is kept");
        let text = std::fs::read_to_string(&path).expect("written");
        assert!(!text.contains("check_updates"), "{text}");
        let launcher = Launcher::load(Some(&path));
        assert_eq!((launcher.after_close, launcher.path_prompt), (AfterClose::Shell, PathPrompt::Dismissed));
    }

    #[test]
    fn quvyta_s_old_switch_left_on_says_nothing_to_the_family_and_goes() {
        let (root, path) = handed("check_updates = true\n");
        Family::QUVYTA.set_update_notice_in(root.path(), false).expect("the family chose off elsewhere");
        hand_over_check_updates(&path, root.path()).expect("handed over");
        assert!(!Family::QUVYTA.update_notice_in(root.path()), "a default line does not turn it back on");
        assert!(!std::fs::read_to_string(&path).expect("written").contains("check_updates"));
    }

    #[test]
    fn a_file_without_the_line_or_a_broken_one_is_left_as_it_was() {
        for text in ["after_close = \"shell\"\n# mine\n", "check_updates = false\nafter_close = \"exit\"\n"] {
            let (root, path) = handed(text);
            hand_over_check_updates(&path, root.path()).expect("nothing to do");
            assert_eq!(std::fs::read_to_string(&path).expect("read"), text, "left byte for byte");
            assert!(Family::QUVYTA.update_notice_in(root.path()));
            assert!(!root.path().join("quvyta.conf").exists(), "the family's file is not touched");
        }
        let missing = tempfile::tempdir().expect("temp");
        hand_over_check_updates(&missing.path().join("launcher.conf"), missing.path()).expect("no file");
        assert!(!missing.path().join("launcher.conf").exists());
    }

    #[test]
    fn each_setting_is_written_alone_and_the_others_stay() {
        let root = tempfile::tempdir().expect("temp");
        let path = root.path().join("launcher.conf");
        std::fs::write(&path, "path_prompt = \"dismissed\"\nafter_close = \"shell\"\n").expect("settings");
        Launcher::save_after_close(&path, AfterClose::Return).expect("saved");
        let launcher = Launcher::load(Some(&path));
        assert_eq!(launcher.after_close, AfterClose::Return);
        assert_eq!((launcher.path_prompt, launcher.diagnostics.as_slice()), (PathPrompt::Dismissed, &[][..]));

        Launcher::save_after_close(&path, AfterClose::Shell).expect("saved");
        let text = std::fs::read_to_string(&path).expect("written");
        assert!(text.contains("after_close = \"shell\"") && text.contains("path_prompt = \"dismissed\""), "{text}");
    }

    #[test]
    fn a_folder_that_cannot_be_written_is_an_error() {
        let root = tempfile::tempdir().expect("temp");
        // A file where the settings folder should be: nothing can go inside it.
        std::fs::write(root.path().join("quvyta"), "").expect("file");
        let path = root.path().join("quvyta/launcher.conf");
        assert!(Launcher::dismiss_path_prompt(&path).is_err());
        assert!(Launcher::save_after_close(&path, AfterClose::Shell).is_err());
    }

    #[test]
    fn a_broken_file_does_not_stop_anything() {
        let launcher = load("after_close = \nafter_close = 3\n[[");
        assert_eq!(launcher.after_close, AfterClose::Return);
        assert!(!launcher.diagnostics.is_empty());
    }
}
