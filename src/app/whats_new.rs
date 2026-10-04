//! What changed in the version you now run.
//!
//! The words are already written: `CHANGELOG.md` ships inside the program, in English, and the
//! page shows the section whose heading is the running version, and nothing else. The file is
//! read at compile time, so an installed binary needs nothing beside it, the way the language
//! files are compiled in. A build between releases, whose version has no section of its own,
//! shows nothing and invents nothing.
//!
//! The version whose entry was shown is kept in `seen.toml` in the data folder, next to
//! `latest.toml`, so the page opens once after an update and never again. A first start keeps
//! the running version too, so someone new meets the wizard and never this page. A file that cannot be
//! read is only a cache gone missing: the page opens once and the file is written whole again. A
//! file that cannot be written leaves the page opening at every start, which the page says
//! quietly rather than hiding.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use qframe::diagnostics::Severity;
use qframe::storage::{Settings, atomic_write};

use crate::machine::Machine;

/// The changelog as it ships with the program.
pub(super) const CHANGELOG: &str = include_str!("../../CHANGELOG.md");
/// The file in the data folder that keeps the version whose entry was shown.
const FILE: &str = "seen.toml";
/// The key of that version in it.
const VERSION: &str = "version";

/// The body of the section of `changelog` that belongs to `version`: its own lines, with the
/// heading left out, since the page has a title of its own, and with the `---` rules between the
/// entries left out, which are the file's spacing and not an entry. `None` when there is no such
/// section, or it has nothing in it: there is nothing to show then.
pub(super) fn section(changelog: &str, version: &str) -> Option<String> {
    let mut lines = changelog.lines();
    lines.find(|line| is_heading(line, version))?;
    let mut body: Vec<&str> =
        lines.take_while(|line| !line.starts_with("## ")).filter(|line| line.trim() != "---").collect();
    // The blank lines the file writes around a section are its own, not the entry's.
    while body.first().is_some_and(|line| line.trim().is_empty()) {
        body.remove(0);
    }
    while body.last().is_some_and(|line| line.trim().is_empty()) {
        body.pop();
    }
    (!body.is_empty()).then(|| format!("{}\n", body.join("\n")))
}

/// Whether `line` is the heading of `version`'s own section: `## 0.3.1`, with the date the file
/// writes after it. `### 0.3.1` is a heading inside a section, and `## 0.3.10` is another
/// version's section, not this one's.
fn is_heading(line: &str, version: &str) -> bool {
    line.strip_prefix("## ")
        .and_then(|heading| heading.strip_prefix(version))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
}

/// The running version's own entry out of the changelog that ships, read once: the same text is
/// on the screen every frame, and the framework's document is remembered by its source anyway.
pub(super) fn entry() -> Option<&'static str> {
    static ENTRY: OnceLock<Option<String>> = OnceLock::new();
    ENTRY.get_or_init(|| section(CHANGELOG, env!("CARGO_PKG_VERSION"))).as_deref()
}

/// Whether the page opens at this start, and whether the running version could be kept: a file
/// that could not be written leaves the page opening at every start, so the caller says so.
///
/// `None` when there is nothing to show: a build between releases, or a version that was seen
/// already. No file at all is someone who used quvyta before it kept one, and they are shown the
/// page like anyone who updated: a first start never gets here, since the wizard has the screen
/// then and [`first_start`] keeps the running version for it.
pub(super) fn offer(machine: &Machine) -> Option<io::Result<()>> {
    let path = seen_path(machine)?;
    entry()?;
    if seen(&path).is_some_and(|version| version == env!("CARGO_PKG_VERSION")) {
        return None;
    }
    Some(keep(&path))
}

/// Keeps the running version on a first start, while the wizard has the screen: a changelog is
/// no way to welcome someone, and without this the page would greet them at their second start.
pub(super) fn first_start(machine: &Machine) -> io::Result<()> {
    match seen_path(machine) {
        Some(path) => keep(&path),
        None => Ok(()),
    }
}

/// Where the version that was seen is kept; `None` when this machine names no data folder, where
/// there would be nowhere to keep it and the page would open at every start.
fn seen_path(machine: &Machine) -> Option<PathBuf> {
    machine.data_dir.as_ref().map(|data| data.join(FILE))
}

/// The version the file at `path` holds, `None` when it holds none that is ours: a file quvyta
/// did not write whole, or one that cannot be read at all. Either way it is only a cache gone
/// missing, and the page opens once for it.
fn seen(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let settings = Settings::parse_str(FILE, &text);
    if settings.diagnostics().iter().any(|diagnostic| diagnostic.severity == Severity::Error) {
        return None;
    }
    settings.get(VERSION)
}

/// Writes the running version to `path` in one step, so a crash never leaves half a file.
fn keep(path: &Path) -> io::Result<()> {
    let mut settings = Settings::parse_str(FILE, "");
    settings.set(VERSION, env!("CARGO_PKG_VERSION").to_owned());
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    atomic_write(path, settings.to_toml().as_bytes())
}
