//! Which members have a newer version on crates.io.
//!
//! One `cargo search quvyta` brings every Quvyta app at once, with no network library of our
//! own: cargo already knows how to reach crates.io. The answer is kept in `latest.toml` in the
//! data folder with the time it was asked, and asked again only after a day, so starting
//! quvyta does not wait on the network every time. Once a day is what the shared update notice
//! promises for every Quvyta application; `r` asks at any time, since that is someone asking. A file that cannot be read is only a cache
//! gone missing: it is ignored and asked again.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use qframe::diagnostics::Severity;
use qframe::storage::{Settings, atomic_write};

use crate::cargo::parse_search;
use crate::ecosystem::APPS;
use crate::machine::Machine;

/// The file in the data folder that keeps the last answer.
const FILE: &str = "latest.toml";
/// How long an answer is used before crates.io is asked again: a day, as the shared update
/// notice says of every member.
pub(crate) const FRESH: Duration = Duration::from_secs(24 * 60 * 60);
/// The key of the time the answer came, in seconds since 1970.
const CHECKED: &str = "checked";
/// The table of the versions, by package.
const VERSIONS: &str = "versions";
/// What cargo is asked. Every member's package starts with `quvyta`, and twenty is room for the
/// Quvyta apps and the crates around them that share the name.
pub(crate) const SEARCH: [&str; 4] = ["search", "quvyta", "--limit", "20"];

/// The newest version of each member's package on crates.io, and when that was asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Latest {
    /// When crates.io answered, in seconds since 1970.
    pub(crate) checked: u64,
    versions: BTreeMap<String, String>,
}

impl Latest {
    /// The answer of `cargo search`, `text`, at `checked`, keeping only the packages of the
    /// released Quvyta apps: another crate whose name starts the same way, or a member still
    /// to come, is none of quvyta's business.
    pub fn from_search(text: &str, checked: u64) -> Self {
        let versions = parse_search(text)
            .into_iter()
            .filter(|found| APPS.iter().any(|member| member.published() && member.package == found.package))
            .map(|found| (found.package, found.version))
            .collect();
        Self { checked, versions }
    }

    /// The newest version of `package`, when crates.io named it.
    pub(crate) fn version(&self, package: &str) -> Option<&str> {
        self.versions.get(package).map(String::as_str)
    }

    /// Whether the answer is younger than [`FRESH`] at `now`. One from the future, after the
    /// clock was turned back, is not trusted.
    pub(crate) fn fresh(&self, now: u64) -> bool {
        now.checked_sub(self.checked).is_some_and(|age| age < FRESH.as_secs())
    }

    /// Reads the answer kept at `path`; `None` when there is none, or when the file is not one
    /// quvyta wrote whole, which is only asked again.
    pub(crate) fn read(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let settings = Settings::parse_str(FILE, &text);
        if settings.diagnostics().iter().any(|diagnostic| diagnostic.severity == Severity::Error) {
            return None;
        }
        let checked = u64::try_from(settings.get::<i64>(CHECKED)?).ok()?;
        let prefix = format!("{VERSIONS}.");
        let mut versions = BTreeMap::new();
        for key in settings.keys() {
            let Some(package) = key.strip_prefix(&prefix) else { continue };
            // A version that is not text, or not a version, means the file is not ours.
            let version = settings.get::<String>(key)?;
            let line = format!("{package} = \"{version}\"");
            let [found] = parse_search(&line).try_into().ok()?;
            versions.insert(found.package, found.version);
        }
        Some(Self { checked, versions })
    }

    /// Writes the answer to `path` in one step, so a crash never leaves half a file.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file or its folder cannot be written.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        let mut settings = Settings::parse_str(FILE, "");
        settings.set(CHECKED, i64::try_from(self.checked).unwrap_or(i64::MAX));
        for (package, version) in &self.versions {
            settings.set(&format!("{VERSIONS}.{package}"), version.clone());
        }
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        atomic_write(path, settings.to_toml().as_bytes())
    }
}

/// Where the last answer is kept; `None` when there is no data folder.
pub fn cache_path(machine: &Machine) -> Option<PathBuf> {
    machine.data_dir.as_ref().map(|data| data.join(FILE))
}

/// Now, in seconds since 1970.
pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    /// The newest versions: from crates.io, or kept from an answer younger than a day.
    Known(Latest),
    /// crates.io could not be asked; the last answer, when there is one, still stands.
    Failed(Option<Latest>),
}

/// Finds the newest versions at `now`: the kept answer while it is fresh, unless `again` asks
/// crates.io anyway. Runs cargo and waits on the network, so it belongs in the background.
pub(crate) fn check(machine: &Machine, again: bool, now: u64) -> Check {
    let path = cache_path(machine);
    let kept = path.as_deref().and_then(Latest::read);
    if !again && let Some(kept) = kept.as_ref().filter(|kept| kept.fresh(now)) {
        return Check::Known(kept.clone());
    }
    match ask(machine, now) {
        Some(latest) => {
            if let Some(path) = &path {
                // An answer that cannot be kept is still the answer; the next start asks again.
                let _ = latest.write(path);
            }
            Check::Known(latest)
        }
        None => Check::Failed(kept),
    }
}

/// Asks crates.io through cargo; `None` when cargo is missing or fails.
fn ask(machine: &Machine, now: u64) -> Option<Latest> {
    let mut cargo = machine.cargo(SEARCH)?;
    // The home folder, so a toolchain file where quvyta was started does not make rustup fetch
    // another toolchain just to search.
    if machine.home.is_dir() {
        cargo.current_dir(&machine.home);
    }
    let output = cargo.output().ok()?;
    output.status.success().then(|| Latest::from_search(&String::from_utf8_lossy(&output.stdout), now))
}

/// Whether `candidate` is a newer version than `installed`. Versions are compared as numbers,
/// part by part, so `0.1.10` is newer than `0.1.9`; a pre-release such as `0.2.0-beta.1` comes
/// before its release. Anything that does not read as a version is never newer.
pub(crate) fn newer(candidate: &str, installed: &str) -> bool {
    compare(candidate, installed) == Some(Ordering::Greater)
}

/// Orders two versions as semantic versioning does; `None` when either is not a version.
fn compare(a: &str, b: &str) -> Option<Ordering> {
    let (a_numbers, a_pre) = parts(a)?;
    let (b_numbers, b_pre) = parts(b)?;
    let length = a_numbers.len().max(b_numbers.len());
    let number = |numbers: &[u64], at: usize| numbers.get(at).copied().unwrap_or(0);
    let by_numbers = (0..length)
        .map(|at| number(&a_numbers, at).cmp(&number(&b_numbers, at)))
        .find(|order| order.is_ne())
        .unwrap_or(Ordering::Equal);
    Some(by_numbers.then_with(|| match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => pre_release(a, b),
    }))
}

/// The numbers of a version and its pre-release, without build metadata (`+…`).
fn parts(version: &str) -> Option<(Vec<u64>, Option<&str>)> {
    let version = version.split('+').next().unwrap_or(version);
    let (release, pre) = match version.split_once('-') {
        Some((release, pre)) => (release, Some(pre)),
        None => (version, None),
    };
    let numbers = release.split('.').map(|part| part.parse().ok()).collect::<Option<Vec<u64>>>()?;
    Some((numbers, pre))
}

/// Pre-releases compare part by part: numbers as numbers and before words, words by their
/// letters, and a shorter one that the longer one starts with comes first.
fn pre_release(a: &str, b: &str) -> Ordering {
    let mut a = a.split('.');
    let mut b = b.split('.');
    loop {
        let order = match (a.next(), b.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) => match (a.parse::<u64>(), b.parse::<u64>()) {
                (Ok(a), Ok(b)) => a.cmp(&b),
                (Ok(_), Err(_)) => Ordering::Less,
                (Err(_), Ok(_)) => Ordering::Greater,
                (Err(_), Err(_)) => a.cmp(b),
            },
        };
        if order.is_ne() {
            return order;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::inventory::tests::machine_with_cargo;

    /// What `cargo search quvyta --limit 20` printed, recorded.
    pub(crate) const SEARCH_OUT: &str = include_str!("../tests/cargo-output/search.txt");

    /// Makes the stand-in cargo in `root` answer `search` with `out` and `code`.
    pub(crate) fn search_scenario(root: &Path, out: &str, code: i32) {
        std::fs::write(root.join("bin/search.out"), out).expect("scenario");
        std::fs::write(root.join("bin/search.code"), code.to_string()).expect("scenario");
    }

    fn searches(root: &Path) -> usize {
        std::fs::read_to_string(root.join("bin/calls.log"))
            .unwrap_or_default()
            .lines()
            .filter(|line| line.ends_with("\tsearch quvyta --limit 20"))
            .count()
    }

    const NOW: u64 = 1_790_000_000;
    const HOUR: u64 = 60 * 60;

    #[test]
    fn only_the_quvyta_apps_packages_are_kept() {
        let latest = Latest::from_search(SEARCH_OUT, NOW);
        assert_eq!(latest.version("quvyta-code"), Some("0.1.1"));
        assert_eq!(latest.version("quvyta-framework-showcase"), Some("0.1.5"));
        assert_eq!(latest.version("quvyta-framework"), None, "the library is not a member");
        assert_eq!(latest.version("quvyta-packages-core"), None);
        assert_eq!(latest.checked, NOW);
    }

    #[test]
    fn a_crate_named_like_a_member_still_to_come_is_not_kept() {
        let _soon = crate::ecosystem::tests::unreleased("desk");
        let latest = Latest::from_search(&format!("{SEARCH_OUT}quvyta-desktop = \"0.0.1\"\n"), NOW);
        assert_eq!(latest.version("quvyta-desktop"), None);
        assert_eq!(latest.version("quvyta-code"), Some("0.1.1"));
    }

    #[test]
    fn versions_compare_as_numbers_and_pre_releases_come_first() {
        for (candidate, installed) in [
            ("0.1.2", "0.1.1"),
            ("0.1.10", "0.1.9"),
            ("1.0.0", "0.9.99"),
            ("0.2.0", "0.2.0-beta.1"),
            ("0.2.0-beta.2", "0.2.0-beta.1"),
            ("0.2.0-beta.11", "0.2.0-beta.2"),
            ("0.2.0-beta", "0.2.0-alpha.9"),
            ("0.2.0-beta.1", "0.2.0-beta"),
            ("0.2.0-rc.1", "0.2.0-1"),
            ("0.1.1", "0.1"),
        ] {
            assert!(newer(candidate, installed), "{candidate} is newer than {installed}");
            assert!(!newer(installed, candidate), "{installed} is not newer than {candidate}");
        }
        for (a, b) in [("0.1.2", "0.1.2"), ("0.1", "0.1.0"), ("0.1.2+build.5", "0.1.2")] {
            assert!(!newer(a, b) && !newer(b, a), "{a} and {b} are the same version");
        }
        for (a, b) in [("0.1.x", "0.1.1"), ("", "0.1.1"), ("0.1.2", "unknown")] {
            assert!(!newer(a, b) && !newer(b, a), "{a:?} or {b:?} is not a version");
        }
    }

    #[test]
    fn an_answer_is_fresh_for_a_day() {
        let latest = Latest { checked: NOW, ..Latest::default() };
        assert!(latest.fresh(NOW));
        assert!(latest.fresh(NOW + 7 * HOUR), "the shared notice asks once a day, not every few hours");
        assert!(latest.fresh(NOW + 24 * HOUR - 1));
        assert!(!latest.fresh(NOW + 24 * HOUR));
        assert!(!latest.fresh(NOW - 1), "an answer from the future is not trusted");
    }

    #[test]
    fn the_answer_is_written_and_read_back() {
        let root = tempfile::tempdir().expect("temp");
        let path = root.path().join("data/latest.toml");
        let latest = Latest::from_search(SEARCH_OUT, NOW);
        latest.write(&path).expect("written");
        let text = std::fs::read_to_string(&path).expect("file");
        assert!(text.starts_with(&format!("checked = {NOW}\n")), "{text}");
        assert!(text.contains("[versions]\nquvyta = \"0.1.2\"\n"), "{text}");
        assert_eq!(Latest::read(&path), Some(latest));
    }

    #[test]
    fn a_broken_or_foreign_file_is_ignored() {
        let root = tempfile::tempdir().expect("temp");
        let path = root.path().join("latest.toml");
        assert_eq!(Latest::read(&path), None, "no file");
        for text in [
            "",
            "checked = \n[[",
            "checked = \"yesterday\"\n",
            "checked = -5\n",
            "checked = 1790000000\n[versions]\nquvyta-code = 3\n",
            "checked = 1790000000\n[versions]\nquvyta-code = \"0.1.2 --force\"\n",
            "\u{0}\u{ff}garbage",
        ] {
            std::fs::write(&path, text).expect("file");
            assert_eq!(Latest::read(&path), None, "{text:?}");
        }
        std::fs::write(&path, [0xff, 0xfe, 0x00]).expect("file");
        assert_eq!(Latest::read(&path), None, "not text");
    }

    #[test]
    fn a_fresh_answer_is_used_without_asking_and_a_stale_one_asks_again() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        search_scenario(root.path(), SEARCH_OUT, 0);
        let Check::Known(first) = check(&machine, false, NOW) else { panic!("asked") };
        assert_eq!((first.version("quvyta-tools"), searches(root.path())), (Some("0.1.2"), 1));
        assert_eq!(Latest::read(&cache_path(&machine).expect("data")), Some(first.clone()), "kept");

        search_scenario(root.path(), &SEARCH_OUT.replace("quvyta-tools = \"0.1.2\"", "quvyta-tools = \"0.1.3\""), 0);
        assert_eq!(check(&machine, false, NOW + 5 * HOUR), Check::Known(first), "fresh: not asked");
        assert_eq!(searches(root.path()), 1);
        let Check::Known(again) = check(&machine, true, NOW + 5 * HOUR) else { panic!("asked") };
        assert_eq!((again.version("quvyta-tools"), searches(root.path())), (Some("0.1.3"), 2), "asked when told to");
        let Check::Known(later) = check(&machine, false, NOW + 30 * HOUR) else { panic!("asked") };
        assert_eq!((later.checked, searches(root.path())), (NOW + 30 * HOUR, 3), "stale: asked");
    }

    #[test]
    fn a_failed_search_keeps_the_last_answer() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        search_scenario(root.path(), "", 101);
        assert_eq!(check(&machine, false, NOW), Check::Failed(None));
        let kept = Latest::from_search(SEARCH_OUT, NOW);
        kept.write(&cache_path(&machine).expect("data")).expect("written");
        assert_eq!(check(&machine, false, NOW + 25 * HOUR), Check::Failed(Some(kept.clone())));
        assert_eq!(Latest::read(&cache_path(&machine).expect("data")), Some(kept), "a failure overwrites nothing");
    }

    #[test]
    fn a_broken_kept_file_is_asked_again() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        search_scenario(root.path(), SEARCH_OUT, 0);
        let path = cache_path(&machine).expect("data");
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, "checked = [[").expect("broken");
        assert!(matches!(check(&machine, false, NOW), Check::Known(_)));
        assert_eq!(searches(root.path()), 1);
        assert!(Latest::read(&path).is_some(), "and written whole again");
    }

    #[test]
    fn without_cargo_nothing_is_known() {
        let root = tempfile::tempdir().expect("temp");
        assert_eq!(check(&Machine::in_root(root.path()), false, NOW), Check::Failed(None));
    }
}
