//! Which members are installed on this machine, from cargo's own record.
//!
//! cargo keeps the list of what it installed; quvyta keeps no record of its own, so the two can
//! never disagree. A member's command found somewhere cargo does not know about still counts as
//! installed, with an unknown version. Finding a command only looks at the file: running a
//! member to ask its version could open it.

use std::path::PathBuf;

use crate::cargo::{Installed, parse_install_list};
use crate::family::{FAMILY, Member};
use crate::machine::Machine;

/// How one member is installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Its command is nowhere to be found.
    Missing,
    /// cargo installed it: cargo can update and remove it.
    Cargo {
        /// The installed version.
        version: String,
        /// Its command in cargo's `bin` folder.
        path: PathBuf,
    },
    /// Its command is there, but not from cargo; whatever put it there looks after it.
    Elsewhere {
        /// Where the command was found.
        path: PathBuf,
    },
    /// The member is quvyta, which is running.
    This {
        /// The running version.
        version: &'static str,
        /// The version cargo's record names, when cargo installed quvyta: only then can cargo
        /// update it, and after an update it is newer than the one running.
        cargo: Option<String>,
    },
}

impl State {
    /// The version cargo installed, for a member cargo can update.
    pub fn cargo_version(&self) -> Option<&str> {
        match self {
            Self::Cargo { version, .. } | Self::This { cargo: Some(version), .. } => Some(version),
            Self::Missing | Self::Elsewhere { .. } | Self::This { cargo: None, .. } => None,
        }
    }

    /// The command that opens the member, for a member quvyta can open: an installed one that
    /// is not quvyta itself.
    pub fn program(&self) -> Option<&PathBuf> {
        match self {
            Self::Cargo { path, .. } | Self::Elsewhere { path } => Some(path),
            Self::Missing | Self::This { .. } => None,
        }
    }
}

/// The state of every member, in the order of [`FAMILY`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    states: Vec<State>,
}

impl Inventory {
    /// Asks cargo what it installed, then looks for the commands of the rest. Without cargo, or
    /// when it fails, only the files are looked at. Runs a program, so it belongs in the
    /// background.
    pub(crate) fn read(machine: &Machine) -> Self {
        let output = machine.cargo(["install", "--list"]).and_then(|mut cargo| cargo.output().ok());
        let listed = match output {
            Some(output) if output.status.success() => parse_install_list(&String::from_utf8_lossy(&output.stdout)),
            _ => Vec::new(),
        };
        Self::from_list(machine, &listed)
    }

    /// The states given cargo's list of installed packages.
    pub fn from_list(machine: &Machine, listed: &[Installed]) -> Self {
        Self { states: FAMILY.iter().map(|member| state(machine, listed, member)).collect() }
    }

    /// The state of the member at `index` of [`FAMILY`].
    pub(crate) fn state(&self, index: usize) -> &State {
        self.states.get(index).unwrap_or(&State::Missing)
    }
}

fn state(machine: &Machine, listed: &[Installed], member: &Member) -> State {
    let path = machine.cargo_bin().join(member.command);
    let by_cargo = listed.iter().find(|installed| {
        installed.package == member.package && installed.commands.iter().any(|c| c == member.command)
    });
    // cargo's record alone is not enough: a command deleted by hand cannot be opened.
    let by_cargo = by_cargo.filter(|_| machine.find_program(member.command).as_ref() == Some(&path));
    if member.is_self() {
        let cargo = by_cargo.map(|installed| installed.version.clone());
        return State::This { version: env!("CARGO_PKG_VERSION"), cargo };
    }
    // cargo can only have installed a member still to come from a local build; quvyta did not
    // put it there and crates.io has nothing to update it to, so it counts as installed elsewhere.
    if let Some(installed) = by_cargo.filter(|_| member.published()) {
        return State::Cargo { version: installed.version.clone(), path };
    }
    match machine.find_program(member.command) {
        Some(path) => State::Elsewhere { path },
        None => State::Missing,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::Path;

    use super::*;

    /// The stand-in for cargo, answering from scenario files beside it.
    const FAKE_CARGO: &str = include_str!("../tests/fake-cargo/cargo");

    /// Writes an executable `path`.
    pub(crate) fn write_program(path: &Path, text: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("folder");
        std::fs::write(path, text).expect("program");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("mode");
    }

    /// Puts the stand-in cargo on the `PATH` of a machine in `root`, answering `install --list`
    /// with `list` and `code`, and returns that machine. Its home folder exists, as a real one
    /// does: installs run in it.
    pub(crate) fn machine_with_cargo(root: &Path, list: &str, code: i32) -> Machine {
        std::fs::create_dir_all(root.join("home")).expect("home");
        let cargo = root.join("bin/cargo");
        write_program(&cargo, FAKE_CARGO);
        std::fs::write(root.join("bin/list.out"), list).expect("scenario");
        std::fs::write(root.join("bin/list.code"), code.to_string()).expect("scenario");
        // A CARGO_TARGET_DIR of the shell running the tests reaches every child; the stand-in
        // drops that one value, so it only ever sees a folder quvyta itself chose.
        if let Some(inherited) = std::env::var_os("CARGO_TARGET_DIR") {
            std::fs::write(root.join("bin/inherited-target"), inherited.as_encoded_bytes()).expect("scenario");
        }
        wait_until_runnable(&cargo);
        Machine::in_root(root)
    }

    /// Another test starting a program at the moment this one wrote its file can hold the file
    /// open for an instant, and Linux refuses to run a file open for writing; wait that out.
    pub(crate) fn wait_until_runnable(program: &Path) {
        for _ in 0..100 {
            match std::process::Command::new(program).arg("ready").output() {
                Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                _ => return,
            }
        }
    }

    /// A command file of `name` in cargo's `bin` folder, as `cargo install` leaves it.
    pub(crate) fn installed_command(machine: &Machine, name: &str) -> PathBuf {
        let path = machine.cargo_bin().join(name);
        write_program(&path, "#!/bin/sh\nexit 0\n");
        path
    }

    fn index(key: &str) -> usize {
        FAMILY.iter().position(|member| member.key == key).expect("a member")
    }

    const LIST: &str = "\
quvyta-code v0.1.1:
    qcode
    quvyta-code
quvyta-framework-showcase v0.1.4:
    qframe
    quvyta-framework-showcase
";

    #[test]
    fn cargo_record_gives_versions_and_full_paths() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), LIST, 0);
        let qcode = installed_command(&machine, "qcode");
        let qframe = installed_command(&machine, "qframe");
        let inventory = Inventory::read(&machine);
        assert_eq!(inventory.state(index("code")), &State::Cargo { version: "0.1.1".to_owned(), path: qcode });
        assert_eq!(inventory.state(index("framework")), &State::Cargo { version: "0.1.4".to_owned(), path: qframe });
        assert_eq!(inventory.state(index("focus")), &State::Missing);
        assert_eq!(inventory.state(index("quvyta")), &State::This { version: env!("CARGO_PKG_VERSION"), cargo: None });
    }

    #[test]
    fn quvyta_knows_whether_cargo_installed_it_and_which_version() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), &format!("{LIST}quvyta v0.1.9:\n    quvyta\n"), 0);
        let running = env!("CARGO_PKG_VERSION");
        assert_eq!(
            Inventory::read(&machine).state(index("quvyta")),
            &State::This { version: running, cargo: None },
            "a record without the file is not cargo's"
        );
        installed_command(&machine, "quvyta");
        let state = Inventory::read(&machine).state(index("quvyta")).clone();
        assert_eq!(state, State::This { version: running, cargo: Some("0.1.9".to_owned()) });
        assert_eq!(state.cargo_version(), Some("0.1.9"));
        assert_eq!(state.program(), None, "quvyta still does not open itself");
    }

    #[test]
    fn cargo_runs_with_the_cargo_home_of_the_machine() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), LIST, 0);
        Inventory::read(&machine);
        let calls = std::fs::read_to_string(root.path().join("bin/calls.log")).expect("cargo was called");
        let told = |line: &str| {
            let fields: Vec<&str> = line.split('\t').collect();
            fields.first() == Some(&machine.cargo_home.to_str().expect("utf-8"))
                && fields.last() == Some(&"install --list")
        };
        assert!(calls.lines().any(told), "{calls}");
    }

    #[test]
    fn a_command_on_path_that_cargo_does_not_know_is_installed_elsewhere() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), LIST, 0);
        let elsewhere = root.path().join("bin/qtools");
        write_program(&elsewhere, "#!/bin/sh\nexit 0\n");
        let inventory = Inventory::read(&machine);
        assert_eq!(inventory.state(index("tools")), &State::Elsewhere { path: elsewhere });
    }

    #[test]
    fn a_command_in_cargo_bin_without_a_record_is_installed_elsewhere() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), "", 0);
        let qpac = installed_command(&machine, "qpac");
        assert_eq!(Inventory::read(&machine).state(index("packages")), &State::Elsewhere { path: qpac });
    }

    #[test]
    fn a_record_whose_command_was_deleted_is_not_installed() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), LIST, 0);
        assert_eq!(Inventory::read(&machine).state(index("code")), &State::Missing);
    }

    #[test]
    fn without_cargo_the_files_are_enough() {
        let root = tempfile::tempdir().expect("temp");
        let machine = Machine::in_root(root.path());
        assert_eq!(machine.cargo, None);
        let qcode = installed_command(&machine, "qcode");
        let inventory = Inventory::read(&machine);
        assert_eq!(inventory.state(index("code")), &State::Elsewhere { path: qcode });
        assert_eq!(inventory.state(index("focus")), &State::Missing);
        assert_eq!(inventory.state(index("quvyta")), &State::This { version: env!("CARGO_PKG_VERSION"), cargo: None });
    }

    #[test]
    fn a_failing_cargo_falls_back_to_the_files() {
        let root = tempfile::tempdir().expect("temp");
        let machine = machine_with_cargo(root.path(), LIST, 101);
        let qcode = installed_command(&machine, "qcode");
        assert_eq!(Inventory::read(&machine).state(index("code")), &State::Elsewhere { path: qcode });
    }

    #[test]
    fn installed_members_open_with_their_full_path_and_quvyta_does_not_open_itself() {
        let path = PathBuf::from("/x/qcode");
        assert_eq!(State::Cargo { version: "0.1.1".to_owned(), path: path.clone() }.program(), Some(&path));
        assert_eq!(State::Elsewhere { path: path.clone() }.program(), Some(&path));
        assert_eq!(State::Missing.program(), None);
        assert_eq!(State::This { version: "0.1.2", cargo: None }.program(), None);
        assert_eq!(State::Elsewhere { path: path.clone() }.cargo_version(), None, "cargo does not manage it");
        assert_eq!(State::Cargo { version: "0.1.1".to_owned(), path }.cargo_version(), Some("0.1.1"));
    }
}
