//! Installing a member with `cargo install`, one at a time, in a shared build folder, and
//! removing one with `cargo uninstall`.
//!
//! cargo builds everything in a folder of its own and puts the program in place only once the
//! build has succeeded, so an install that is stopped or fails leaves nothing half-installed.
//! quvyta's part is the build folder, which it shares across a queue so the members' common
//! crates are built once and deletes when the queue ends, and the log of each member's last
//! install.
//!
//! Two quvyta windows must not share the folder: the one installing holds an [`AppLock`] beside
//! it, and the lock, not the folder, says whether it is in use. A folder nobody holds is what a
//! crash or a closed terminal left behind, and is deleted.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use qframe::runtime::{Line, Process, ProcessOutcome};
use qframe::storage::{AppLock, atomic_write};

use crate::cargo::{self, Failure};
use crate::ecosystem::Member;
use crate::machine::Machine;

/// The folder under the data folder that builds go to.
const BUILD: &str = "build";
/// The lock beside it. Beside rather than inside: deleting the folder never deletes a lock
/// someone holds.
const BUILD_LOCK: &str = "build.lock";
/// The folder under the data folder that logs go to.
const LOGS: &str = "logs";

/// The size of the terminal cargo is given. Wide enough that crate names are not cut. cargo
/// draws its progress line on it and ends each frame with `\r`, so the frames are not lines:
/// they arrive on their own, beside the lines, and say how far the build has got.
pub const TERMINAL: (u16, u16) = (120, 30);

/// One `cargo install`, with everything it needs from the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    /// The package on crates.io.
    pub package: &'static str,
    /// The version to install; `None` for the latest one.
    pub version: Option<String>,
    /// The cargo program.
    pub cargo: PathBuf,
    /// Where cargo installs, told to it explicitly.
    pub cargo_home: PathBuf,
    /// The folder cargo runs in: the home folder, so a toolchain file where quvyta was started
    /// does not choose the compiler.
    pub dir: PathBuf,
    /// The shared build folder; `None` when there is no data folder and cargo picks its own.
    pub build: Option<PathBuf>,
    /// Where the whole log of this install is written.
    pub log: Option<PathBuf>,
}

impl Job {
    /// The install of `member` at `version` on `machine`; `None` without cargo.
    pub fn new(machine: &Machine, member: &Member, version: Option<String>) -> Option<Self> {
        let cargo = machine.cargo.clone().or_else(|| machine.find_program("cargo"))?;
        Some(Self {
            package: member.package,
            version,
            cargo,
            cargo_home: machine.cargo_home.clone(),
            dir: machine.home.clone(),
            build: build_dir(machine),
            log: log_path(machine, member),
        })
    }

    /// cargo's arguments: always `--locked`, so the versions the member was released and tested
    /// with are the ones built.
    pub fn args(&self) -> Vec<String> {
        let mut args = vec!["install".to_owned(), "--locked".to_owned(), self.package.to_owned()];
        if let Some(version) = &self.version {
            args.extend(["--version".to_owned(), version.clone()]);
        }
        args
    }

    /// The command as a user would type it to do the same, with cargo's full path.
    pub fn command_line(&self) -> String {
        std::iter::once(self.cargo.display().to_string()).chain(self.args()).collect::<Vec<_>>().join(" ")
    }

    /// Runs the install, handing every line, without its terminal escapes, to `on_line`, and
    /// writes the whole log. `cancel` is asked between lines; when it turns true cargo and
    /// everything it started are killed.
    ///
    /// Each frame of cargo's progress line, which a carriage return overwrites in place, goes to
    /// `on_frame` instead: it says how far the build has got, and it is not a line of output, so
    /// it is neither logged nor written to the log file.
    pub fn run(
        &self,
        cancel: &dyn Fn() -> bool,
        on_line: &mut dyn FnMut(String),
        on_frame: &mut dyn FnMut(String),
    ) -> Outcome {
        let mut process = Process::new(&self.cargo)
            .args(self.args())
            .dir(&self.dir)
            .env("CARGO_HOME", &self.cargo_home)
            .pty(TERMINAL.0, TERMINAL.1)
            .no_stdin();
        if let Some(build) = &self.build {
            process = process.env("CARGO_TARGET_DIR", build);
        }
        let mut lines = Vec::new();
        let result = process.run_with_overwritten(
            cancel,
            &mut |line| {
                let (Line::Out(text) | Line::Err(text)) = line;
                let text = cargo::plain(&text);
                lines.push(text.clone());
                on_line(text);
            },
            &mut |frame| {
                let (Line::Out(text) | Line::Err(text)) = frame;
                on_frame(cargo::plain(&text));
            },
        );
        let outcome = match result {
            Ok(ProcessOutcome::Finished { code: Some(0) }) => {
                Outcome::Installed { version: cargo::installed_version(self.package, &lines).or(self.version.clone()) }
            }
            Ok(ProcessOutcome::Finished { .. }) => Outcome::Failed(cargo::failure(&lines)),
            Ok(ProcessOutcome::Cancelled) => Outcome::Cancelled,
            Err(error) => {
                let reason = error.to_string();
                lines.push(reason.clone());
                Outcome::NotStarted(reason)
            }
        };
        if let Some(log) = &self.log {
            // A log that cannot be written must not turn an install into a failure.
            let _ = write_log(log, &lines);
        }
        outcome
    }
}

/// One `cargo uninstall`. It only deletes the program and cargo's record of it: nothing is
/// built, so it needs no build folder, and it keeps no log, which would replace the one of the
/// member's last install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removal {
    /// The package cargo installed.
    pub package: &'static str,
    /// The cargo program.
    pub cargo: PathBuf,
    /// Where cargo installed it, told to it explicitly.
    pub cargo_home: PathBuf,
    /// The folder cargo runs in: the home folder, as for an install.
    pub dir: PathBuf,
}

impl Removal {
    /// The removal of `member` on `machine`; `None` without cargo.
    pub fn new(machine: &Machine, member: &Member) -> Option<Self> {
        let cargo = machine.cargo.clone().or_else(|| machine.find_program("cargo"))?;
        Some(Self { package: member.package, cargo, cargo_home: machine.cargo_home.clone(), dir: machine.home.clone() })
    }

    /// cargo's arguments.
    pub fn args(&self) -> [&'static str; 2] {
        ["uninstall", self.package]
    }

    /// The command as a user would type it to do the same, with cargo's full path.
    pub fn command_line(&self) -> String {
        format!("{} uninstall {}", self.cargo.display(), self.package)
    }

    /// Runs the removal, handing every line to `on_line`. `cancel` is asked between lines.
    pub fn run(&self, cancel: &dyn Fn() -> bool, on_line: &mut dyn FnMut(String)) -> Outcome {
        let process =
            Process::new(&self.cargo).args(self.args()).dir(&self.dir).env("CARGO_HOME", &self.cargo_home).no_stdin();
        let result = process.run(cancel, &mut |line| {
            let (Line::Out(text) | Line::Err(text)) = line;
            on_line(cargo::plain(&text));
        });
        match result {
            Ok(ProcessOutcome::Finished { code: Some(0) }) => Outcome::Removed,
            Ok(ProcessOutcome::Finished { .. }) => Outcome::NotRemoved,
            Ok(ProcessOutcome::Cancelled) => Outcome::Cancelled,
            Err(error) => {
                let reason = error.to_string();
                on_line(reason.clone());
                Outcome::NotStarted(reason)
            }
        }
    }
}

/// How an install or a removal ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The program is in place, at this version when cargo said which.
    Installed {
        /// The version installed.
        version: Option<String>,
    },
    /// cargo failed, for this reason.
    Failed(Failure),
    /// The program is removed.
    Removed,
    /// `cargo uninstall` failed; its output says why.
    NotRemoved,
    /// It was stopped.
    Cancelled,
    /// cargo could not be started, for this reason.
    NotStarted(String),
}

/// The build folder: `build/` in quvyta's data folder.
pub fn build_dir(machine: &Machine) -> Option<PathBuf> {
    machine.data_dir.as_ref().map(|data| data.join(BUILD))
}

/// Where the log of `member`'s last install goes: `logs/<command>.log` in the data folder.
pub fn log_path(machine: &Machine, member: &Member) -> Option<PathBuf> {
    machine.data_dir.as_ref().map(|data| data.join(LOGS).join(format!("{}.log", member.command)))
}

fn write_log(path: &Path, lines: &[String]) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut text = lines.join("\n");
    text.push('\n');
    atomic_write(path, text.as_bytes())
}

/// Whether this window may use the build folder.
#[derive(Debug)]
pub enum Claim {
    /// It may, for as long as it holds this lock.
    Held(AppLock),
    /// Another quvyta window is installing.
    Busy,
    /// There is no data folder, or its lock cannot be taken; installs go ahead without a shared
    /// folder, as cargo would on its own.
    Unshared,
}

/// Takes the build folder for this window. The folder is left as it is: a leftover is removed
/// when quvyta starts, and a folder this window left is reused.
pub fn claim(machine: &Machine) -> Claim {
    let Some(data) = &machine.data_dir else { return Claim::Unshared };
    match std::fs::create_dir_all(data).and_then(|()| AppLock::acquire(&data.join(BUILD_LOCK))) {
        Ok(Some(lock)) => Claim::Held(lock),
        Ok(None) => Claim::Busy,
        Err(_) => Claim::Unshared,
    }
}

/// Whether another quvyta window holds the build folder, checked without keeping it.
pub fn held_elsewhere(machine: &Machine) -> bool {
    matches!(claim(machine), Claim::Busy)
}

/// Deletes the build folder when nobody holds it: what an install that never finished, in a
/// window that crashed or whose terminal closed, left behind. Answers whether another window
/// holds it. Reads the disk and may delete gigabytes, so it belongs in the background.
pub fn clear_leftover(machine: &Machine) -> bool {
    // Nothing was ever built here: nothing to look at, and nothing is created by looking.
    let untouched = |data: &PathBuf| !data.join(BUILD).exists() && !data.join(BUILD_LOCK).exists();
    if machine.data_dir.as_ref().is_none_or(untouched) {
        return false;
    }
    match claim(machine) {
        Claim::Held(lock) => {
            release(machine, lock);
            false
        }
        Claim::Busy => true,
        Claim::Unshared => false,
    }
}

/// Deletes the build folder, then lets go of `lock`: once the queue has ended the folder only
/// takes room, a gigabyte or two, and another window may start its own. Deleting may take a
/// while, so it belongs in the background.
pub fn release(machine: &Machine, lock: AppLock) {
    if let Some(build) = build_dir(machine) {
        // A folder that cannot be deleted is found again, and tried again, next time.
        let _ = std::fs::remove_dir_all(build);
    }
    drop(lock);
}

/// The arguments of `rustup update stable`.
pub fn rustup_update() -> Vec<OsString> {
    crate::checks::RUSTUP_UPDATE.iter().map(OsString::from).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::ecosystem::APPS;
    use crate::inventory::tests::machine_with_cargo;

    pub(crate) fn member(key: &str) -> &'static Member {
        APPS.iter().find(|member| member.key == key).expect("a member")
    }

    /// Makes the stand-in cargo in `root` answer `install` with `out` and `code`.
    pub(crate) fn scenario(root: &Path, out: &str, code: i32) {
        std::fs::write(root.join("bin/install.out"), out).expect("scenario");
        std::fs::write(root.join("bin/install.code"), code.to_string()).expect("scenario");
    }

    /// Tells the stand-in cargo which command each package installs, and at which version.
    pub(crate) fn packages(root: &Path) {
        let lines: String =
            APPS.iter().map(|member| format!("{} {} 0.1.2\n", member.package, member.command)).collect();
        std::fs::write(root.join("bin/packages"), lines).expect("scenario");
    }

    const INSTALL: &str = include_str!("../tests/cargo-output/install-ok.txt");

    /// Whether the build folder is free within a moment. A program another test starts at the
    /// instant a lock is let go holds a copy of it until it has started, so a released lock is
    /// free in a moment rather than at once.
    pub(crate) fn free_soon(machine: &Machine) -> bool {
        (0..200).any(|_| {
            let free = !held_elsewhere(machine);
            if !free {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            free
        })
    }

    /// The build folder taken, waiting out the same moment [`free_soon`] waits out: between a
    /// program being started and it running, the child holds a copy of every open file of this
    /// process, the lock among them, so a folder nobody holds can still refuse a claim.
    pub(crate) fn claim_soon(machine: &Machine) -> Claim {
        for _ in 0..200 {
            match claim(machine) {
                Claim::Busy => std::thread::sleep(std::time::Duration::from_millis(5)),
                taken => return taken,
            }
        }
        claim(machine)
    }

    fn run(job: &Job) -> (Outcome, Vec<String>) {
        let (outcome, lines, _) = run_with_frames(job);
        (outcome, lines)
    }

    /// The install with its lines and, apart from them, the frames of cargo's progress line.
    fn run_with_frames(job: &Job) -> (Outcome, Vec<String>, Vec<String>) {
        let (mut lines, mut frames) = (Vec::new(), Vec::new());
        let outcome = job.run(&|| false, &mut |line| lines.push(line), &mut |frame| frames.push(frame));
        (outcome, lines, frames)
    }

    #[test]
    fn the_command_is_locked_and_pins_a_known_version() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        let job = Job::new(&machine, member("tools"), None).expect("cargo is there");
        assert_eq!(job.args(), ["install", "--locked", "quvyta-tools"]);
        assert_eq!(
            job.command_line(),
            format!("{} install --locked quvyta-tools", root.path().join("bin/cargo").display())
        );
        let pinned = Job::new(&machine, member("tools"), Some("0.1.2".to_owned())).expect("cargo is there");
        assert_eq!(pinned.args(), ["install", "--locked", "quvyta-tools", "--version", "0.1.2"]);
        assert_eq!(pinned.build, Some(root.path().join("data/build")));
        assert_eq!(pinned.log, Some(root.path().join("data/logs/qtools.log")));
    }

    #[test]
    fn without_cargo_there_is_nothing_to_run() {
        let root = tempfile::tempdir().expect("temp");
        assert_eq!(Job::new(&Machine::in_root(root.path()), member("tools"), None), None);
        assert_eq!(Removal::new(&Machine::in_root(root.path()), member("tools")), None);
    }

    #[test]
    fn a_removal_deletes_the_program_and_cargo_s_record_of_it() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "quvyta-tools v0.1.2:\n    qtools\n", 0);
        packages(root.path());
        crate::inventory::tests::installed_command(&machine, "qtools");
        let removal = Removal::new(&machine, member("tools")).expect("cargo");
        assert_eq!(
            removal.command_line(),
            format!("{} uninstall quvyta-tools", root.path().join("bin/cargo").display())
        );
        let mut lines = Vec::new();
        assert_eq!(removal.run(&|| false, &mut |line| lines.push(line)), Outcome::Removed);
        assert!(!machine.cargo_bin().join("qtools").exists());
        assert_eq!(std::fs::read_to_string(root.path().join("bin/list.out")).expect("list"), "");
        let calls = std::fs::read_to_string(root.path().join("bin/calls.log")).expect("calls");
        let expected =
            format!("{}\t\t{}\tuninstall quvyta-tools", machine.cargo_home.display(), machine.home.display());
        assert!(calls.lines().any(|line| line == expected), "no build folder, in the home folder:\n{calls}");
        assert!(!root.path().join("data/logs").exists(), "no log replaces the install's");
    }

    #[test]
    fn a_failed_removal_says_so_and_keeps_the_program() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "quvyta-tools v0.1.2:\n    qtools\n", 0);
        packages(root.path());
        crate::inventory::tests::installed_command(&machine, "qtools");
        std::fs::write(root.path().join("bin/uninstall.err"), "error: Permission denied (os error 13)\n")
            .expect("scenario");
        std::fs::write(root.path().join("bin/uninstall.code"), "101").expect("scenario");
        let mut lines = Vec::new();
        let outcome =
            Removal::new(&machine, member("tools")).expect("cargo").run(&|| false, &mut |line| lines.push(line));
        assert_eq!(outcome, Outcome::NotRemoved);
        assert_eq!(lines, ["error: Permission denied (os error 13)"]);
        assert!(machine.cargo_bin().join("qtools").exists());
    }

    #[test]
    fn a_successful_install_puts_the_command_in_place_and_says_which_version() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        scenario(root.path(), INSTALL, 0);
        packages(root.path());
        let job = Job::new(&machine, member("tools"), None).expect("cargo");
        let (outcome, lines) = run(&job);
        assert_eq!(outcome, Outcome::Installed { version: Some("0.1.2".to_owned()) });
        assert!(machine.cargo_bin().join("qtools").is_file(), "the stand-in put the command in place");
        assert!(lines.iter().all(|line| !line.contains('\u{1b}')), "escapes are taken out: {lines:?}");
        assert!(lines.iter().any(|line| line.trim() == "Compiling ratatui v0.29.0"), "{lines:?}");
        assert!(
            !lines.iter().any(|line| line.contains("Building")),
            "a progress line erased in place is a frame, never a line: {lines:?}"
        );
        let log = std::fs::read_to_string(root.path().join("data/logs/qtools.log")).expect("log written");
        assert!(log.contains("Installed package `quvyta-tools v0.1.2`"), "{log}");
        let calls = std::fs::read_to_string(root.path().join("bin/calls.log")).expect("calls");
        let expected = format!(
            "{}\t{}\t{}\tinstall --locked quvyta-tools",
            machine.cargo_home.display(),
            root.path().join("data/build").display(),
            machine.home.display()
        );
        assert!(calls.lines().any(|line| line == expected), "{calls}");
    }

    /// A real install's bytes, recorded on a pseudo-terminal the size of [`TERMINAL`], played
    /// back through the same reader cargo's output goes through.
    #[test]
    fn a_recorded_install_delivers_its_progress_frames_beside_its_lines() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        let recording = include_str!("../tests/cargo-output/install-pty.txt");
        scenario(root.path(), recording, 0);
        packages(root.path());
        let (outcome, lines, frames) = run_with_frames(&Job::new(&machine, member("tools"), None).expect("cargo"));
        assert!(matches!(outcome, Outcome::Installed { .. }), "{outcome:?}");
        let steps: Vec<cargo::Step> = lines.iter().filter_map(|line| cargo::step(line)).collect();
        let compiling = steps.iter().filter(|step| matches!(step, cargo::Step::Compiling { .. })).count();
        assert_eq!(compiling, 33, "every `Compiling` line arrives, as before: {lines:?}");
        assert_eq!(steps.last(), Some(&cargo::Step::Placing));

        // The counts cargo redraws in place now arrive too, every frame of them, and they climb.
        let counted: Vec<(u32, u32, String)> = frames
            .iter()
            .filter_map(|frame| match cargo::step(frame) {
                Some(cargo::Step::Counted { done, total, krate }) => Some((done, total, krate)),
                _ => None,
            })
            .collect();
        assert_eq!(counted.len(), recording.matches("Building\u{1b}[0m [").count(), "{counted:?}");
        assert_eq!(counted.first(), Some(&(0, 46, "anstyle".to_owned())), "{counted:?}");
        assert_eq!(counted.last(), Some(&(45, 46, "hexyl".to_owned())), "{counted:?}");
        assert!(counted.windows(2).all(|pair| pair[0].0 <= pair[1].0), "the counts only climb: {counted:?}");

        // A frame is redrawn in place, not written: it belongs in no log.
        assert!(!lines.iter().any(|line| line.contains("Building")), "a frame is not a line: {lines:?}");
        let log = std::fs::read_to_string(root.path().join("data/logs/qtools.log")).expect("log written");
        assert!(!log.contains("Building"), "the log file holds only real lines:\n{log}");
        assert_eq!(log.lines().count(), lines.len(), "the log is the lines and nothing else");
    }

    #[test]
    fn a_failed_install_says_why_and_keeps_its_log() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        scenario(root.path(), "   Compiling proc-macro2 v1.0.95\nerror: linker `cc` not found\n", 101);
        packages(root.path());
        let (outcome, _) = run(&Job::new(&machine, member("tools"), None).expect("cargo"));
        assert_eq!(outcome, Outcome::Failed(Failure::NoLinker));
        assert!(!machine.cargo_bin().join("qtools").exists());
        let log = std::fs::read_to_string(root.path().join("data/logs/qtools.log")).expect("log written");
        assert!(log.contains("linker `cc` not found"), "{log}");
    }

    #[test]
    fn only_the_last_install_of_a_member_is_logged() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        let job = Job::new(&machine, member("tools"), None).expect("cargo");
        scenario(root.path(), "first attempt\n", 101);
        run(&job);
        scenario(root.path(), "second attempt\n", 101);
        run(&job);
        let log = std::fs::read_to_string(root.path().join("data/logs/qtools.log")).expect("log");
        assert_eq!(log, "second attempt\n");
    }

    #[test]
    fn stopping_kills_cargo_and_installs_nothing() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        scenario(root.path(), "   Compiling serde v1.0.219\n", 0);
        packages(root.path());
        std::fs::write(root.path().join("bin/install.sleep"), "30").expect("scenario");
        let job = Job::new(&machine, member("tools"), None).expect("cargo");
        let stop = Arc::new(AtomicBool::new(false));
        let started = std::time::Instant::now();
        let flag = Arc::clone(&stop);
        let outcome =
            job.run(&move || flag.load(Ordering::Relaxed), &mut |_| stop.store(true, Ordering::Relaxed), &mut |_| {});
        assert_eq!(outcome, Outcome::Cancelled);
        assert!(started.elapsed() < std::time::Duration::from_secs(10), "cargo did not run its thirty seconds");
        assert!(!machine.cargo_bin().join("qtools").exists(), "nothing was installed");
    }

    #[test]
    fn a_cargo_that_cannot_start_is_told() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        let mut job = Job::new(&machine, member("tools"), None).expect("cargo");
        job.cargo = root.path().join("bin/missing-cargo");
        let (outcome, _) = run(&job);
        assert!(matches!(outcome, Outcome::NotStarted(_)), "{outcome:?}");
    }

    #[test]
    fn a_leftover_build_folder_is_deleted_and_a_held_one_kept() {
        let root = tempfile::tempdir().expect("temp");
        let machine = Machine::in_root(root.path());
        let build = root.path().join("data/build");
        std::fs::create_dir_all(build.join("release")).expect("leftover");
        assert!(!clear_leftover(&machine));
        assert!(!build.exists(), "nobody held it, so it was left behind");

        std::fs::create_dir_all(build.join("release")).expect("in use");
        // Children other tests start meanwhile can hold the released lock for a moment.
        let Claim::Held(lock) = claim_soon(&machine) else { panic!("the folder is free") };
        assert!(held_elsewhere(&machine), "a second claim sees it taken");
        assert!(clear_leftover(&machine), "another window holds it");
        assert!(build.exists(), "a folder in use is kept");
        release(&machine, lock);
        assert!(!build.exists(), "the queue's end deletes it");
        assert!(free_soon(&machine));
    }

    #[test]
    fn looking_for_a_leftover_creates_nothing() {
        let root = tempfile::tempdir().expect("temp");
        assert!(!clear_leftover(&Machine::in_root(root.path())));
        assert!(!root.path().join("data").exists());
    }

    #[test]
    fn without_a_data_folder_installs_still_run() {
        let root = tempfile::tempdir().expect("temp");
        let mut machine = machine_with_cargo(root.path(), "", 0);
        machine.data_dir = None;
        assert!(matches!(claim(&machine), Claim::Unshared));
        let job = Job::new(&machine, member("tools"), None).expect("cargo");
        assert_eq!((job.build, job.log), (None, None));
    }

    /// A real install from crates.io into a temporary cargo folder: run only in a disposable
    /// container, never on a machine someone uses, with
    /// `cargo test -- --ignored a_real_install` (`QUVYTA_E2E_PACKAGE` picks the package,
    /// `quvyta-tools` by default). It needs the network and takes a few minutes.
    #[test]
    #[ignore = "installs from crates.io; run in a disposable container"]
    fn a_real_install_from_crates_io() {
        let package = std::env::var("QUVYTA_E2E_PACKAGE").unwrap_or_else(|_| "quvyta-tools".to_owned());
        let member = APPS.iter().find(|member| member.package == package).expect("a member");
        let root = tempfile::tempdir().expect("temp");
        let home = root.path().join("home");
        std::fs::create_dir_all(&home).expect("home");
        let cargo = std::env::var_os("PATH")
            .and_then(|path| std::env::split_paths(&path).map(|dir| dir.join("cargo")).find(|cargo| cargo.is_file()))
            .expect("cargo on PATH");
        let job = Job {
            package: member.package,
            version: None,
            cargo,
            cargo_home: home.join(".cargo"),
            dir: home.clone(),
            build: Some(root.path().join("data/build")),
            log: Some(root.path().join("data/logs").join(format!("{}.log", member.command))),
        };
        let (mut steps, mut frames) = (Vec::new(), Vec::new());
        let outcome = job.run(
            &|| false,
            &mut |line| {
                if let Some(step) = cargo::step(&line) {
                    steps.push(step);
                }
            },
            &mut |frame| {
                if let Some(step) = cargo::step(&frame) {
                    frames.push(step);
                }
            },
        );
        let log = std::fs::read_to_string(job.log.as_ref().expect("log")).expect("log written");
        let Outcome::Installed { version: Some(version) } = &outcome else { panic!("{outcome:?}\n{log}") };
        println!("installed {package} {version}");
        assert!(home.join(".cargo/bin").join(member.command).is_file(), "{log}");
        assert!(steps.first().is_some_and(|step| *step == cargo::Step::Downloading), "{steps:?}");
        assert!(steps.iter().any(|step| matches!(step, cargo::Step::Compiling { .. })), "{steps:?}");
        assert_eq!(steps.last(), Some(&cargo::Step::Placing), "{steps:?}");
        assert!(!steps.iter().any(|step| matches!(step, cargo::Step::Counted { .. })), "a frame is no line: {steps:?}");
        let counted: Vec<(u32, u32, &str)> = frames
            .iter()
            .filter_map(|step| match step {
                cargo::Step::Counted { done, total, krate } => Some((*done, *total, krate.as_str())),
                _ => None,
            })
            .collect();
        println!("{} lines with a step, {} frames, {} of them counted", steps.len(), frames.len(), counted.len());
        println!("first {:?}, last {:?}", counted.first(), counted.last());
        assert!(!counted.is_empty(), "cargo's progress frames arrive: {frames:?}");
        assert!(counted.windows(2).all(|pair| pair[0].0 <= pair[1].0), "the counts only climb: {counted:?}");
        assert!(!log.contains("Building"), "no frame reaches the log:\n{log}");
        assert!(!log.contains('\u{1b}'), "escapes are taken out of the log");
    }
}
