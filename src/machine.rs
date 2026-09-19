//! What quvyta knows about the machine it runs on: the folders and programs it touches.
//!
//! Everything outside the program comes from a [`Machine`], and only [`Machine::from_env`] reads
//! the process environment. Tests build one over a temporary folder instead, so they never
//! change the environment and can run side by side.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use qframe::storage::Family;

use crate::shell_path::Environment;

/// The id quvyta's own settings go under: `launcher.conf`, since `quvyta.conf` is the file the
/// whole family shares.
const LAUNCHER: &str = "launcher";

/// The name of quvyta's data folder.
const DATA: &str = "quvyta";

/// The C linkers Rust can finish a build with, in the order `install.sh` looks for them.
const LINKERS: [&str; 3] = ["cc", "gcc", "clang"];

/// The folders and programs of one machine, as quvyta sees them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    /// The user's home folder; members open in it.
    pub home: PathBuf,
    /// Cargo's folder: `CARGO_HOME`, or `~/.cargo`.
    pub cargo_home: PathBuf,
    /// The folders of `PATH`, in order.
    pub path: Vec<PathBuf>,
    /// The cargo program to run, found in `CARGO_HOME/bin` or on `PATH`; `None` when there is
    /// none.
    pub cargo: Option<PathBuf>,
    /// quvyta's settings file, `launcher.conf`, when the platform names a settings folder.
    pub launcher_conf: Option<PathBuf>,
    /// The family's settings folder, which removing a member leaves alone; `None` when the
    /// platform names none.
    pub settings_dir: Option<PathBuf>,
    /// quvyta's data folder, which holds the build folder of installs and their logs; `None`
    /// when the platform names none.
    pub data_dir: Option<PathBuf>,
    /// The file naming the Linux distribution, which says how to install a C linker.
    pub os_release: PathBuf,
    /// `SHELL`: which start-up file puts cargo's folder on `PATH`.
    pub shell: Option<String>,
    /// `XDG_CONFIG_HOME`, where fish keeps its start-up file.
    pub xdg_config_home: Option<PathBuf>,
    /// `ZDOTDIR`, where zsh keeps its start-up file.
    pub zdotdir: Option<PathBuf>,
}

impl Machine {
    /// The machine as the environment of this process describes it.
    pub fn from_env() -> Self {
        let lookup = |name: &str| std::env::var_os(name);
        Self {
            data_dir: qframe::storage::data_dir(DATA),
            settings_dir: Family::QUVYTA.config_dir(),
            ..Self::resolve(lookup, Family::QUVYTA.app_file(LAUNCHER))
        }
    }

    /// A machine whose home is `root/home`, whose `PATH` is `root/bin`, whose settings are in
    /// `root/config` (the family's folder), whose data in `root/data` and whose distribution is named in
    /// `root/etc/os-release`, with no `SHELL`, `XDG_CONFIG_HOME` or `ZDOTDIR`. Nothing outside
    /// `root` is touched: the screen can be shown, and tried, without the real home.
    pub fn in_root(root: &Path) -> Self {
        let home = root.join("home");
        let path = root.join("bin");
        let lookup = |name: &str| match name {
            "HOME" => Some(home.clone().into_os_string()),
            "PATH" => Some(path.clone().into_os_string()),
            _ => None,
        };
        Self {
            data_dir: Some(root.join("data")),
            settings_dir: Some(root.join("config")),
            os_release: root.join("etc/os-release"),
            ..Self::resolve(lookup, Some(root.join("config/launcher.conf")))
        }
    }

    /// Reads the variables through `lookup`; the one place that decides what they mean.
    fn resolve(lookup: impl Fn(&str) -> Option<OsString>, launcher_conf: Option<PathBuf>) -> Self {
        let absolute = |name: &str| lookup(name).map(PathBuf::from).filter(|path| path.is_absolute());
        let home = absolute("HOME").unwrap_or_default();
        let cargo_home = absolute("CARGO_HOME").unwrap_or_else(|| home.join(".cargo"));
        let path = lookup("PATH").map(|value| std::env::split_paths(&value).collect()).unwrap_or_default();
        let mut machine = Self {
            home,
            cargo_home,
            path,
            cargo: None,
            launcher_conf,
            settings_dir: None,
            data_dir: None,
            os_release: PathBuf::from("/etc/os-release"),
            shell: lookup("SHELL").and_then(|value| value.into_string().ok()),
            xdg_config_home: lookup("XDG_CONFIG_HOME").map(PathBuf::from),
            zdotdir: lookup("ZDOTDIR").map(PathBuf::from),
        };
        machine.cargo = machine.find_program("cargo");
        machine
    }

    /// Where cargo puts the programs it installs.
    pub fn cargo_bin(&self) -> PathBuf {
        self.cargo_home.join("bin")
    }

    /// The first executable file named `name` in cargo's `bin` folder or on `PATH`. Only the file
    /// is looked at; nothing is run.
    pub(crate) fn find_program(&self, name: &str) -> Option<PathBuf> {
        std::iter::once(self.cargo_bin())
            .chain(self.path.iter().cloned())
            .map(|dir| dir.join(name))
            .find(|candidate| is_executable(candidate))
    }

    /// The first C linker on `PATH`, or `None` when there is none. Like [`Machine::find_program`]
    /// it only looks at the files, so a linker installed while quvyta runs is found the next time.
    pub(crate) fn linker(&self) -> Option<PathBuf> {
        LINKERS.iter().find_map(|name| self.find_program(name))
    }

    /// Cargo with `args`, told this machine's `CARGO_HOME` so it never falls back to its own
    /// idea of it; `None` when there is no cargo.
    pub(crate) fn cargo(&self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Option<Command> {
        let mut command = Command::new(self.cargo.as_ref()?);
        command.args(args).env("CARGO_HOME", &self.cargo_home);
        Some(command)
    }

    /// What the PATH rules look at on this machine.
    pub(crate) fn shell_env(&self) -> Environment<'_> {
        Environment {
            home: &self.home,
            cargo_home: Some(&self.cargo_home),
            path: &self.path,
            shell: self.shell.as_deref(),
            xdg_config_home: self.xdg_config_home.as_deref(),
            zdotdir: self.zdotdir.as_deref(),
        }
    }

    /// `path` for people to read: the home folder is written as `~`.
    pub(crate) fn show(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rest) if !self.home.as_os_str().is_empty() => Path::new("~").join(rest).display().to_string(),
            _ => path.display().to_string(),
        }
    }
}

/// Whether `path` is a file this user may run.
fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else { return false };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_program(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("folder");
        std::fs::write(path, "#!/bin/sh\n").expect("program");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("mode");
    }

    #[test]
    fn cargo_home_defaults_to_the_home_folder() {
        let root = tempfile::tempdir().expect("temp");
        let machine = Machine::in_root(root.path());
        assert_eq!(machine.home, root.path().join("home"));
        assert_eq!(machine.cargo_home, root.path().join("home/.cargo"));
        assert_eq!(machine.path, [root.path().join("bin")]);
        assert_eq!(machine.cargo, None, "no cargo in an empty root");
    }

    #[test]
    fn cargo_home_and_path_come_from_the_variables() {
        let vars = |name: &str| match name {
            "HOME" => Some(OsString::from("/home/ayse")),
            "CARGO_HOME" => Some(OsString::from("/opt/cargo")),
            "PATH" => Some(OsString::from("/usr/bin:/bin")),
            _ => None,
        };
        let machine = Machine::resolve(vars, None);
        assert_eq!(machine.cargo_home, Path::new("/opt/cargo"));
        assert_eq!(machine.cargo_bin(), Path::new("/opt/cargo/bin"));
        assert_eq!(machine.path, [PathBuf::from("/usr/bin"), PathBuf::from("/bin")]);
    }

    #[test]
    fn the_shell_and_its_folders_come_from_the_variables() {
        let vars = |name: &str| match name {
            "HOME" => Some(OsString::from("/home/ayse")),
            "SHELL" => Some(OsString::from("/usr/bin/zsh")),
            "XDG_CONFIG_HOME" => Some(OsString::from("/home/ayse/conf")),
            "ZDOTDIR" => Some(OsString::from("/home/ayse/.zsh")),
            _ => None,
        };
        let machine = Machine::resolve(vars, None);
        let env = machine.shell_env();
        assert_eq!(env.shell, Some("/usr/bin/zsh"));
        assert_eq!(env.xdg_config_home, Some(Path::new("/home/ayse/conf")));
        assert_eq!(env.zdotdir, Some(Path::new("/home/ayse/.zsh")));
        assert_eq!(env.bin_dir(), Path::new("/home/ayse/.cargo/bin"));
        let bare = Machine::resolve(|name: &str| (name == "HOME").then(|| OsString::from("/home/ayse")), None);
        assert_eq!((bare.shell, bare.xdg_config_home, bare.zdotdir), (None, None, None));
    }

    #[test]
    fn a_relative_cargo_home_is_not_trusted() {
        let vars = |name: &str| match name {
            "HOME" => Some(OsString::from("/home/ayse")),
            "CARGO_HOME" => Some(OsString::from("cargo")),
            _ => None,
        };
        let machine = Machine::resolve(vars, None);
        assert_eq!(
            machine.cargo_home,
            Path::new("/home/ayse/.cargo"),
            "a relative path depends on where quvyta starts"
        );
    }

    #[test]
    fn cargo_is_found_in_its_own_folder_before_path_and_only_when_executable() {
        let root = tempfile::tempdir().expect("temp");
        write_program(&root.path().join("bin/cargo"), 0o755);
        assert_eq!(Machine::in_root(root.path()).cargo, Some(root.path().join("bin/cargo")));
        write_program(&root.path().join("home/.cargo/bin/cargo"), 0o755);
        assert_eq!(Machine::in_root(root.path()).cargo, Some(root.path().join("home/.cargo/bin/cargo")));
        write_program(&root.path().join("home/.cargo/bin/cargo"), 0o644);
        assert_eq!(
            Machine::in_root(root.path()).cargo,
            Some(root.path().join("bin/cargo")),
            "a file that cannot run is not a program"
        );
    }

    #[test]
    fn cargo_is_told_the_cargo_home_of_the_machine() {
        let root = tempfile::tempdir().expect("temp");
        write_program(&root.path().join("bin/cargo"), 0o755);
        let machine = Machine::in_root(root.path());
        let command = machine.cargo(["install", "--list"]).expect("cargo is there");
        let cargo_home = command.get_envs().find(|(key, _)| *key == "CARGO_HOME").and_then(|(_, value)| value);
        assert_eq!(cargo_home, Some(machine.cargo_home.as_os_str()));
        assert_eq!(command.get_args().collect::<Vec<_>>(), ["install", "--list"]);
    }

    #[test]
    fn a_linker_is_any_of_cc_gcc_and_clang() {
        let root = tempfile::tempdir().expect("temp");
        assert_eq!(Machine::in_root(root.path()).linker(), None);
        write_program(&root.path().join("bin/clang"), 0o755);
        assert_eq!(Machine::in_root(root.path()).linker(), Some(root.path().join("bin/clang")));
        write_program(&root.path().join("bin/cc"), 0o755);
        assert_eq!(Machine::in_root(root.path()).linker(), Some(root.path().join("bin/cc")));
    }

    #[test]
    fn paths_under_home_are_shown_with_a_tilde() {
        let root = tempfile::tempdir().expect("temp");
        let machine = Machine::in_root(root.path());
        assert_eq!(machine.show(&machine.cargo_bin().join("qcode")), "~/.cargo/bin/qcode");
        assert_eq!(machine.show(Path::new("/usr/bin/qcode")), "/usr/bin/qcode");
    }
}
