//! What an install needs before it starts: cargo, a Rust new enough and a C linker. The checks
//! run when the install dialog opens, so a problem is told with its fix before anything starts
//! rather than as a failed build minutes later.

use std::path::PathBuf;

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
}

impl Problem {
    /// The one line quvyta can run to put this right, shown to the person before they choose to
    /// run it; `None` when there is no single line it can be sure of. rustup's line is a shell
    /// pipe, which only a Unix shell runs.
    pub fn fix(&self) -> Option<&'static str> {
        match self {
            Self::NoCargo => cfg!(unix).then_some(RUSTUP_INSTALL),
            Self::OldRust { .. } => None,
            Self::NoLinker(distro) => distro.linker_command(),
        }
    }
}

/// Everything in the way of an install on `machine`, most basic first; empty when it can go
/// ahead. Runs `rustc --version`, so it belongs in the background.
pub fn run(machine: &Machine) -> Vec<Problem> {
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
    problems
}

/// The distribution this machine runs, from the file that names it. The one place the question is
/// answered: the linker command of a failed install and the members that run on Arch Linux only
/// both read it here.
pub fn distro(machine: &Machine) -> Distro {
    let text = std::fs::read_to_string(&machine.os_release).unwrap_or_default();
    Distro::from_os_release(&text)
}

/// The version `rustc --version` reports, run from the home folder so that a project's own
/// toolchain file where quvyta was started does not decide; `None` when it cannot tell.
fn rustc_version(machine: &Machine) -> Option<String> {
    let rustc = machine.find_program("rustc")?;
    let output = std::process::Command::new(rustc)
        .arg("--version")
        .current_dir(&machine.home)
        .env("CARGO_HOME", &machine.cargo_home)
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    parse_rustc_version(&String::from_utf8_lossy(&output.stdout))
}

/// `1.80.1` from `rustc 1.80.1 (3f5fd8dd4 2024-08-06)`.
fn parse_rustc_version(text: &str) -> Option<String> {
    let version = text.trim().strip_prefix("rustc ")?.split_whitespace().next()?;
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
        assert_eq!(run(&machine(root.path(), "rustc 1.95.0 (29483883e 2026-08-07)", true)), []);
        let root = tempfile::tempdir().expect("temp");
        assert_eq!(run(&machine(root.path(), "rustc 1.97.0-nightly (a1b2c3d4e 2026-09-01)", true)), []);
    }

    #[test]
    fn without_cargo_nothing_else_about_rust_is_asked() {
        let root = tempfile::tempdir().expect("temp");
        write_program(&root.path().join("bin/cc"), "#!/bin/sh\nexit 0\n");
        assert_eq!(run(&Machine::in_root(root.path())), [Problem::NoCargo]);
    }

    #[test]
    fn an_old_rust_is_found_with_rustup_or_without() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine(root.path(), "rustc 1.80.1 (3f5fd8dd4 2024-08-06)", true);
        assert_eq!(run(&machine), [Problem::OldRust { version: "1.80.1".to_owned(), rustup: None }]);
        let rustup = machine.cargo_bin().join("rustup");
        write_program(&rustup, "#!/bin/sh\nexit 0\n");
        assert_eq!(run(&machine), [Problem::OldRust { version: "1.80.1".to_owned(), rustup: Some(rustup) }]);
    }

    #[test]
    fn a_rustc_that_says_nothing_useful_is_not_blamed() {
        let root = tempfile::tempdir().expect("temp");
        assert_eq!(run(&machine(root.path(), "error: no default toolchain", true)), []);
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
            assert_eq!(run(&machine), [Problem::NoLinker(distro)], "{os_release}");
        }
    }

    #[test]
    fn a_machine_without_os_release_shows_every_command() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine(root.path(), "rustc 1.95.0 (29483883e 2026-08-07)", false);
        assert_eq!(run(&machine), [Problem::NoLinker(Distro::Other)]);
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

    #[test]
    fn versions_are_compared_by_their_numbers() {
        assert_eq!(parse_rustc_version("rustc 1.95.0 (29483883e 2026-08-07)\n").as_deref(), Some("1.95.0"));
        assert_eq!(parse_rustc_version("cargo 1.95.0"), None);
        assert!(new_enough("1.95.0") && new_enough("1.100.0") && new_enough("2.0.0"));
        assert!(!new_enough("1.94.1") && !new_enough("1.9.0"));
    }
}
