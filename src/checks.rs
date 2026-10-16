//! What an install needs before it starts: cargo, a Rust new enough, a C linker, and whatever the
//! app itself needs from the machine. The checks run when the install dialog opens, so a
//! problem is told with its fix before anything starts rather than as a failed build minutes
//! later; the same checks run before an app that needs something of its own is opened.

use std::path::PathBuf;
use std::time::Duration;

use qframe::runtime::{Keep, Process};

use crate::ecosystem::QuvytaApp;
use crate::machine::Machine;

/// The oldest Rust the Quvyta apps build with, as in `install.sh`.
pub const MIN_RUST: (u32, u32) = (1, 95);

/// The line rustup's own page gives to install Rust. Not the Quvyta install script: that one
/// would build quvyta again, and quvyta is already here.
pub const RUSTUP_INSTALL: &str = "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh";

/// Where rustup, the official Rust installer, lives.
pub const RUSTUP_SITE: &str = "https://rustup.rs";

/// What rustup runs to bring Rust up to date.
pub const RUSTUP_UPDATE: [&str; 2] = ["update", "stable"];

/// The programs a terminal browser is drawn by, in the order `qbrow` itself looks for them. This
/// is qbrow's list rather than a list of quvyta's own: quvyta must not tell a person a browser is
/// missing while the program it is about to start would have drawn the page with it, nor the
/// other way round. qbrow also reads a program path it is given outright, which quvyta has no way
/// of passing on.
pub const BROWSERS: [&str; 4] = ["chromium", "chromium-browser", "google-chrome-stable", "google-chrome"];

/// Something that keeps an install from working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// There is no cargo to build with.
    NoCargo,
    /// Rust is older than [`MIN_RUST`].
    OldRust {
        /// The installed version, such as `1.80.1`.
        version: String,
        /// rustup, when it is there to update Rust.
        rustup: Option<PathBuf>,
    },
    /// There is no C linker to finish a build with.
    NoLinker(Distro),
    /// There is no browser for an app that draws its pages with one.
    NoBrowser(Distro),
}

/// The Linux distributions whose linker package `install.sh` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distro {
    /// Arch Linux and the distributions built on it.
    Arch,
    /// Debian, Ubuntu and the distributions built on them.
    Debian,
    /// Fedora and the distributions built on it.
    Fedora,
    /// Any other: every known command is shown.
    Other,
}

impl Distro {
    /// Every distribution with a known command, in `install.sh`'s order.
    pub const KNOWN: [Self; 3] = [Self::Arch, Self::Debian, Self::Fedora];

    /// The distribution `/etc/os-release` names in `ID` or `ID_LIKE`.
    pub fn from_os_release(text: &str) -> Self {
        let value = |key: &str| {
            text.lines()
                .filter_map(|line| line.trim().strip_prefix(key)?.strip_prefix('='))
                .map(|value| value.trim().trim_matches(['"', '\'']).to_owned())
                .next()
                .unwrap_or_default()
        };
        let ids = format!("{} {}", value("ID"), value("ID_LIKE"));
        let has = |id: &str| ids.split_whitespace().any(|word| word == id);
        if has("arch") {
            Self::Arch
        } else if has("debian") || has("ubuntu") {
            Self::Debian
        } else if has("fedora") || has("rhel") {
            Self::Fedora
        } else {
            Self::Other
        }
    }

    /// The distribution's name as people write it, as in `install.sh`'s table.
    pub fn name(self) -> &'static str {
        match self {
            Self::Arch => "Arch Linux",
            Self::Debian => "Debian, Ubuntu",
            Self::Fedora => "Fedora",
            Self::Other => "",
        }
    }

    /// The command that installs a C linker there. It needs sudo, so quvyta runs it only when the
    /// person presses the button beside it, on the terminal where sudo asks for the password.
    pub fn linker_command(self) -> Option<&'static str> {
        match self {
            Self::Arch => Some("sudo pacman -S --needed base-devel"),
            Self::Debian => Some("sudo apt install build-essential"),
            Self::Fedora => Some("sudo dnf install gcc"),
            Self::Other => None,
        }
    }

    /// The command that installs a browser there, the same package `qbrow` offers on its own first
    /// start, so the two say the same line for the same system. It needs sudo, so quvyta runs it
    /// only when the person presses the button beside it.
    pub fn browser_command(self) -> Option<&'static str> {
        match self {
            Self::Arch => Some("sudo pacman -S --needed chromium"),
            Self::Debian => Some("sudo apt install chromium"),
            Self::Fedora => Some("sudo dnf install chromium"),
            Self::Other => None,
        }
    }
}

impl Problem {
    /// Whether this keeps the build from working. A missing browser does not: the app builds and
    /// installs without one, and only needs it to draw pages, so an install is warned about it
    /// and goes ahead.
    pub fn blocks(&self) -> bool {
        !matches!(self, Self::NoBrowser(_))
    }

    /// The one line quvyta can run to put this right, shown to the person before they choose to
    /// run it; `None` when there is no single line it can be sure of. rustup's line is a shell
    /// pipe, which only a Unix shell runs.
    pub fn fix(&self) -> Option<&'static str> {
        match self {
            Self::NoCargo => cfg!(unix).then_some(RUSTUP_INSTALL),
            Self::OldRust { .. } => None,
            Self::NoLinker(distro) => distro.linker_command(),
            Self::NoBrowser(distro) => distro.browser_command(),
        }
    }
}

/// Everything in the way of an install of `app` on `machine`, most basic first; empty when it
/// can go ahead. `app` is `None` where the machine is asked on its own, as after a build has
/// already failed, so an app's own requirement is only ever asked about for an app it is
/// about. Runs `rustc --version`, so it belongs in the background.
pub fn run(machine: &Machine, app: Option<&QuvytaApp>) -> Vec<Problem> {
    let mut problems = Vec::new();
    // cargo may have been installed since quvyta started, so its file is looked for again.
    if machine.cargo.is_none() && machine.find_program("cargo").is_none() {
        problems.push(Problem::NoCargo);
    } else if let Some(version) = rustc_version(machine)
        && !new_enough(&version)
    {
        problems.push(Problem::OldRust { version, rustup: machine.find_program("rustup") });
    }
    if machine.linker().is_none() {
        problems.push(Problem::NoLinker(distro(machine)));
    }
    // An app's own requirement comes after the toolchain: whatever the app is, it is built
    // the same way first, so a missing linker is the more basic thing to say.
    if app.is_some_and(QuvytaApp::needs_browser) && browser(machine).is_none() {
        problems.push(Problem::NoBrowser(distro(machine)));
    }
    problems
}

/// A browser on `machine`, or `None` when there is none of the names [`BROWSERS`] holds. Only the
/// files are looked at, so a browser installed while quvyta runs is found the next time.
fn browser(machine: &Machine) -> Option<PathBuf> {
    BROWSERS.iter().find_map(|name| machine.find_program(name))
}

/// The distribution this machine runs, from the file that names it. The one place the question is
/// answered: the linker command of a failed install and the apps that run on Arch Linux only
/// both read it here.
pub fn distro(machine: &Machine) -> Distro {
    let text = std::fs::read_to_string(&machine.os_release).unwrap_or_default();
    Distro::from_os_release(&text)
}

/// How long `rustc --version` may take. rustup may be fetching a toolchain behind it, and a
/// network that hangs would hold the install question until the connection gave up.
const RUSTC_LIMIT: Duration = Duration::from_secs(120);

/// The version `rustc --version` reports, run from the home folder so that a project's own
/// toolchain file where quvyta was started does not decide; `None` when it cannot tell.
fn rustc_version(machine: &Machine) -> Option<String> {
    let rustc = machine.find_program("rustc")?;
    let collected = Process::new(rustc)
        .arg("--version")
        .dir(&machine.home)
        .env("CARGO_HOME", &machine.cargo_home)
        .no_stdin()
        .collect(Keep::bytes(16 * 1024).limit(RUSTC_LIMIT), &|| false)
        .ok()?;
    parse_rustc_version(&collected.text)
}

/// `1.80.1` from `rustc 1.80.1 (3f5fd8dd4 2024-08-06)`. rustup may write what it is doing first,
/// on the other stream, so the version is looked for on a line of its own.
fn parse_rustc_version(text: &str) -> Option<String> {
    let line = text.lines().find_map(|line| line.trim().strip_prefix("rustc "))?;
    let version = line.split_whitespace().next()?;
    numbers(version).map(|_| version.to_owned())
}

/// The major and minor numbers of `1.95.0` or `1.96.0-nightly`.
fn numbers(version: &str) -> Option<(u32, u32)> {
    let mut parts = version.split(['.', '-']);
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

fn new_enough(version: &str) -> bool {
    numbers(version).is_none_or(|numbers| numbers >= MIN_RUST)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::ecosystem::APPS;
    use crate::inventory::tests::{wait_until_runnable, write_program};

    /// A machine in `root` with cargo, a `rustc` answering `rustc_answer` and, when `linker`, a
    /// `cc`.
    fn machine(root: &Path, rustc_answer: &str, linker: bool) -> Machine {
        write_program(&root.join("bin/cargo"), "#!/bin/sh\nexit 0\n");
        std::fs::create_dir_all(root.join("home")).expect("home");
        write_program(&root.join("bin/rustc"), &format!("#!/bin/sh\necho '{rustc_answer}'\n"));
        wait_until_runnable(&root.join("bin/rustc"));
        if linker {
            write_program(&root.join("bin/cc"), "#!/bin/sh\nexit 0\n");
        }
        Machine::in_root(root)
    }

    #[test]
    fn a_ready_machine_has_no_problems() {
        let root = tempfile::tempdir().expect("temp");
        assert_eq!(run(&machine(root.path(), "rustc 1.95.0 (29483883e 2026-08-07)", true), None), []);
        let root = tempfile::tempdir().expect("temp");
        assert_eq!(run(&machine(root.path(), "rustc 1.97.0-nightly (a1b2c3d4e 2026-09-01)", true), None), []);
    }

    #[test]
    fn without_cargo_nothing_else_about_rust_is_asked() {
        let root = tempfile::tempdir().expect("temp");
        write_program(&root.path().join("bin/cc"), "#!/bin/sh\nexit 0\n");
        assert_eq!(run(&Machine::in_root(root.path()), None), [Problem::NoCargo]);
    }

    #[test]
    fn an_old_rust_is_found_with_rustup_or_without() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine(root.path(), "rustc 1.80.1 (3f5fd8dd4 2024-08-06)", true);
        assert_eq!(run(&machine, None), [Problem::OldRust { version: "1.80.1".to_owned(), rustup: None }]);
        let rustup = machine.cargo_bin().join("rustup");
        write_program(&rustup, "#!/bin/sh\nexit 0\n");
        assert_eq!(run(&machine, None), [Problem::OldRust { version: "1.80.1".to_owned(), rustup: Some(rustup) }]);
    }

    #[test]
    fn a_rustc_that_says_nothing_useful_is_not_blamed() {
        let root = tempfile::tempdir().expect("temp");
        assert_eq!(run(&machine(root.path(), "error: no default toolchain", true), None), []);
    }

    #[test]
    fn a_missing_linker_names_the_distribution() {
        for (os_release, distro) in [
            ("NAME=\"Arch Linux\"\nID=arch\n", Distro::Arch),
            ("NAME=\"EndeavourOS\"\nID=\"endeavouros\"\nID_LIKE=\"arch\"\n", Distro::Arch),
            ("ID=ubuntu\nID_LIKE=debian\n", Distro::Debian),
            ("ID=linuxmint\nID_LIKE=\"ubuntu debian\"\n", Distro::Debian),
            ("ID=fedora\n", Distro::Fedora),
            ("ID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\n", Distro::Fedora),
            ("ID=alpine\n", Distro::Other),
            ("", Distro::Other),
        ] {
            let root = tempfile::tempdir().expect("temp");
            let machine = machine(root.path(), "rustc 1.95.0 (29483883e 2026-08-07)", false);
            std::fs::create_dir_all(root.path().join("etc")).expect("folder");
            std::fs::write(root.path().join("etc/os-release"), os_release).expect("os-release");
            assert_eq!(run(&machine, None), [Problem::NoLinker(distro)], "{os_release}");
        }
    }

    #[test]
    fn a_machine_without_os_release_shows_every_command() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine(root.path(), "rustc 1.95.0 (29483883e 2026-08-07)", false);
        assert_eq!(run(&machine, None), [Problem::NoLinker(Distro::Other)]);
        assert_eq!(Distro::Other.linker_command(), None);
        assert!(Distro::KNOWN.iter().all(|distro| distro.linker_command().is_some_and(|c| c.starts_with("sudo "))));
    }

    #[test]
    fn only_a_known_distribution_gets_a_linker_fix_to_run() {
        assert_eq!(Problem::NoLinker(Distro::Arch).fix(), Some("sudo pacman -S --needed base-devel"));
        assert_eq!(Problem::NoLinker(Distro::Other).fix(), None);
        assert_eq!(Problem::OldRust { version: "1.80.1".to_owned(), rustup: None }.fix(), None);
        assert_eq!(Problem::NoCargo.fix(), cfg!(unix).then_some(RUSTUP_INSTALL));
    }

    /// The exact line for each distribution quvyta knows, and none at all for one it does not:
    /// which of several lines is right cannot be known here, so none of them is run.
    #[test]
    fn the_command_is_the_one_for_the_distribution() {
        assert_eq!(Distro::Arch.browser_command(), Some("sudo pacman -S --needed chromium"));
        assert_eq!(Distro::Debian.browser_command(), Some("sudo apt install chromium"));
        assert_eq!(Distro::Fedora.browser_command(), Some("sudo dnf install chromium"));
        assert_eq!(Distro::Other.browser_command(), None);
        assert!(Distro::KNOWN.iter().all(|distro| distro.browser_command().is_some_and(|c| c.starts_with("sudo "))));
        // The problem runs the very line its distribution names, and nothing else.
        assert_eq!(Problem::NoBrowser(Distro::Debian).fix(), Some("sudo apt install chromium"));
        assert_eq!(Problem::NoBrowser(Distro::Other).fix(), None);
    }

    /// The names are qbrow's own, in its own order: a person with any one of them has no problem,
    /// and quvyta must not disagree with the program it is about to start.
    #[test]
    fn any_of_the_names_qbrow_accepts_leaves_no_problem() {
        assert_eq!(BROWSERS, ["chromium", "chromium-browser", "google-chrome-stable", "google-chrome"]);
        for name in BROWSERS {
            let root = tempfile::tempdir().expect("temp");
            let machine = machine(root.path(), "rustc 1.95.0 (29483883e 2026-08-07)", true);
            write_program(&root.path().join(format!("bin/{name}")), "#!/bin/sh\nexit 0\n");
            assert_eq!(run(&machine, Some(browser())), [], "{name} is what qbrow looks for");
        }
        // A file that cannot run is not a browser, as the shell sees it too.
        let root = tempfile::tempdir().expect("temp");
        let machine = machine(root.path(), "rustc 1.95.0 (29483883e 2026-08-07)", true);
        std::fs::write(root.path().join("bin/chromium"), "").expect("a file");
        assert_eq!(run(&machine, Some(browser())), [Problem::NoBrowser(Distro::Other)]);
    }

    /// Only the app with the requirement is held up by it, and it comes after the toolchain
    /// every app is built with.
    #[test]
    fn an_apps_own_requirement_is_a_problem_only_of_that_app() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine(root.path(), "rustc 1.95.0 (29483883e 2026-08-07)", true);
        assert_eq!(run(&machine, None), [], "nothing is asked of the machine on its own");
        assert_eq!(run(&machine, Some(tools())), [], "an app that needs nothing is not held up");
        assert_eq!(run(&machine, Some(browser())), [Problem::NoBrowser(Distro::Other)]);
        // A machine with neither cargo nor a linker says so first, and the requirement last.
        let root = tempfile::tempdir().expect("temp");
        let bare = Machine::in_root(root.path());
        let found = run(&bare, Some(browser()));
        assert_eq!(found, [Problem::NoCargo, Problem::NoLinker(Distro::Other), Problem::NoBrowser(Distro::Other)]);
    }

    /// The requirement belongs to the browser alone, and these are the two sides of that.
    fn browser() -> &'static QuvytaApp {
        APPS.iter().find(|app| app.key == "browser").expect("qbrow is listed")
    }

    fn tools() -> &'static QuvytaApp {
        APPS.iter().find(|app| app.key == "tools").expect("qtools is listed")
    }

    #[test]
    fn versions_are_compared_by_their_numbers() {
        assert_eq!(parse_rustc_version("rustc 1.95.0 (29483883e 2026-08-07)\n").as_deref(), Some("1.95.0"));
        assert_eq!(parse_rustc_version("cargo 1.95.0"), None);
        // rustup's own words come first, on the other stream, when it fetches the toolchain.
        let fetched = "info: syncing channel updates for 'stable-x86_64-unknown-linux-gnu'\nrustc 1.95.0 (29483883e 2026-08-07)\n";
        assert_eq!(parse_rustc_version(fetched).as_deref(), Some("1.95.0"));
        assert!(new_enough("1.95.0") && new_enough("1.100.0") && new_enough("2.0.0"));
        assert!(!new_enough("1.94.1") && !new_enough("1.9.0"));
    }
}
