//! Whether the programs cargo installs can be started by name, and the line that makes them so.
//!
//! The rules are the ones `install.sh` follows, so the installer and quvyta give the same answer
//! on the same machine and each recognises the line the other added. Both are held to one table
//! of cases, `tests/path-cases.toml`.
//!
//! Nothing here reads the process environment: the caller passes it in as an [`Environment`], so
//! the rules can be checked against any home folder.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The comment written above the line. The installer writes the same one, so either tool can see
/// what the other added.
pub const COMMENT: &str = "# cargo programs, added by the Quvyta installer";

/// What the rules look at, as plain values.
#[derive(Debug, Clone, Copy)]
pub struct Environment<'a> {
    /// The home folder, `HOME`.
    pub home: &'a Path,
    /// `CARGO_HOME`, when it is set. An empty value counts as not set, as it does for the shell.
    pub cargo_home: Option<&'a Path>,
    /// The folders of `PATH`, in order.
    pub path: &'a [PathBuf],
    /// `SHELL`, when it is set.
    pub shell: Option<&'a str>,
    /// `XDG_CONFIG_HOME`, when it is set; empty counts as not set.
    pub xdg_config_home: Option<&'a Path>,
    /// `ZDOTDIR`, when it is set; empty counts as not set.
    pub zdotdir: Option<&'a Path>,
}

impl Environment<'_> {
    /// The folder cargo installs programs into: `CARGO_HOME/bin`, or `~/.cargo/bin`.
    #[must_use]
    pub fn bin_dir(&self) -> PathBuf {
        set(self.cargo_home).map_or_else(|| self.home.join(".cargo"), Path::to_path_buf).join("bin")
    }
}

/// A shell whose start-up file the rules know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    /// bash, which reads `~/.bashrc`.
    Bash,
    /// zsh, which reads `.zshrc` in `ZDOTDIR` or the home folder.
    Zsh,
    /// fish, which reads `fish/config.fish` in `XDG_CONFIG_HOME` or `~/.config`.
    Fish,
}

/// What it takes for cargo's programs to start by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathAction {
    /// Cargo's folder is already on this process's `PATH`.
    OnPath,
    /// A start-up file already adds the folder, with our line or rustup's; only this terminal
    /// predates it, and a new one finds the programs.
    AlreadyConfigured {
        /// The file that adds it.
        file: PathBuf,
    },
    /// `line` would put the folder on `PATH` when appended to `file`.
    Add {
        /// The shell the file belongs to.
        shell: Shell,
        /// The start-up file.
        file: PathBuf,
        /// The line to append, exactly as the installer writes it.
        line: String,
    },
    /// The shell is not one the rules know: the line can only be shown for the user to add.
    Unknown {
        /// The line for a POSIX shell.
        line: String,
    },
}

/// Decides what it takes for cargo's programs to start by name.
///
/// `read` gives the content of a start-up file, or `None` when it is not a readable regular file
/// (a link counts as its target); [`read_file`] reads them from disk.
#[must_use]
pub fn decide(env: &Environment<'_>, read: impl Fn(&Path) -> Option<String>) -> PathAction {
    let bin_dir = env.bin_dir();
    if env.path.contains(&bin_dir) {
        return PathAction::OnPath;
    }
    // The line names the folder through $HOME when it can, so the start-up file still reads well
    // and keeps working if the home folder moves.
    let shown_dir = match bin_dir.strip_prefix(env.home) {
        Ok(rest) => format!("$HOME/{}", rest.display()),
        Err(_) => bin_dir.display().to_string(),
    };
    let export = format!("export PATH=\"{shown_dir}:$PATH\"");
    let config = || set(env.xdg_config_home).map_or_else(|| env.home.join(".config"), Path::to_path_buf);
    let (shell, file, line) = match env.shell.map(Path::new).and_then(Path::file_name).and_then(|name| name.to_str()) {
        Some("bash") => (Shell::Bash, env.home.join(".bashrc"), export),
        Some("zsh") => (Shell::Zsh, set(env.zdotdir).unwrap_or(env.home).join(".zshrc"), export),
        Some("fish") => (Shell::Fish, config().join("fish/config.fish"), format!("fish_add_path {shown_dir}")),
        _ => return PathAction::Unknown { line: export },
    };
    // rustup's own lines source cargo's default folder, so they only count when that is the one.
    let rustup_folder = bin_dir == env.home.join(".cargo/bin");
    if let Some(content) = read(&file)
        && (content.split('\n').any(|existing| existing == line) || (rustup_folder && content.contains(".cargo/env")))
    {
        return PathAction::AlreadyConfigured { file };
    }
    if shell == Shell::Fish && rustup_folder {
        let rustup = config().join("fish/conf.d/rustup.fish");
        if read(&rustup).is_some() {
            return PathAction::AlreadyConfigured { file: rustup };
        }
    }
    PathAction::Add { shell, file, line }
}

/// Reads a start-up file for [`decide`]: its content when it is a regular file or a link to one.
/// Anything unreadable counts as absent, as it does for the installer's `grep`.
#[must_use]
pub fn read_file(path: &Path) -> Option<String> {
    if !fs::metadata(path).ok()?.is_file() {
        return None;
    }
    fs::read(path).ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// Appends the line to the start-up file as the installer does: an empty line, [`COMMENT`] and
/// the line. The file is opened for appending and never rewritten, so a link stays a link and
/// only its target grows; the file and its folder are created when missing.
///
/// # Errors
///
/// Returns the path that could not be created or written, with the reason.
pub fn apply(file: &Path, line: &str) -> Result<(), ApplyError> {
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder).map_err(|error| ApplyError { path: folder.to_path_buf(), error })?;
    }
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(file)
        .and_then(|mut opened| opened.write_all(format!("\n{COMMENT}\n{line}\n").as_bytes()))
        .map_err(|error| ApplyError { path: file.to_path_buf(), error })
}

/// A start-up file that could not be written.
#[derive(Debug)]
pub struct ApplyError {
    /// The file, or the folder that could not be created for it.
    pub path: PathBuf,
    /// Why.
    pub error: io::Error,
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.error)
    }
}

impl std::error::Error for ApplyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// A path as it is shown to the user: `~` for the home folder.
#[must_use]
pub fn shown(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// A variable's value, with an empty one treated as not set, as `${VAR:-default}` does.
fn set(value: Option<&Path>) -> Option<&Path> {
    value.filter(|path| !path.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use qframe::document::{Document, Shape, Table, ValueKind};
    use std::os::unix::fs::{PermissionsExt, symlink};

    const CASES: &str = include_str!("../tests/path-cases.toml");

    fn shape() -> Shape {
        let case = Shape::new()
            .required("name", ValueKind::text())
            .required("on_path", ValueKind::flag())
            .required("outcome", ValueKind::choice(["on-path", "already", "add", "unknown"]))
            .optional("shell", ValueKind::text())
            .optional("cargo_home", ValueKind::text())
            .optional("xdg_config_home", ValueKind::text())
            .optional("zdotdir", ValueKind::text())
            .optional("file", ValueKind::text())
            .optional("before", ValueKind::text())
            .optional("link_to", ValueKind::text())
            .optional("extra_file", ValueKind::text())
            .optional("line", ValueKind::text())
            .optional("configured_by", ValueKind::text())
            .optional("shown_file", ValueKind::text())
            .optional("after", ValueKind::text());
        Shape::new().entries("case", case)
    }

    /// Runs one case in its own temporary folder and says what went wrong, if anything.
    fn run_case(case: &Table) -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|error| format!("temporary folder: {error}"))?;
        let root = temp.path();
        let home = root.join("home");
        fs::create_dir_all(&home).map_err(|error| error.to_string())?;
        let fill = |key: &str| {
            case.text(key).map(|text| {
                text.replace("{home}", &home.display().to_string()).replace("{root}", &root.display().to_string())
            })
        };
        let path_of = |key: &str| fill(key).map(PathBuf::from);
        let need = |key: &str| path_of(key).ok_or_else(|| format!("the case has no {key}"));
        let write = |path: &Path, content: &str| {
            fs::create_dir_all(path.parent().unwrap_or(root)).and_then(|()| fs::write(path, content))
        };

        if let Some(target) = path_of("link_to") {
            let file = need("file")?;
            fs::create_dir_all(target.parent().unwrap_or(root)).map_err(|error| error.to_string())?;
            fs::create_dir_all(file.parent().unwrap_or(root)).map_err(|error| error.to_string())?;
            symlink(&target, &file).map_err(|error| error.to_string())?;
            if let Some(before) = fill("before") {
                write(&target, &before).map_err(|error| error.to_string())?;
            }
        } else if let Some(before) = fill("before") {
            write(&need("file")?, &before).map_err(|error| error.to_string())?;
        }
        if let Some(extra) = path_of("extra_file") {
            write(&extra, "").map_err(|error| error.to_string())?;
        }

        let cargo_home = path_of("cargo_home");
        let xdg_config_home = path_of("xdg_config_home");
        let zdotdir = path_of("zdotdir");
        let shell = fill("shell");
        let mut env = Environment {
            home: &home,
            cargo_home: cargo_home.as_deref(),
            path: &[],
            shell: shell.as_deref(),
            xdg_config_home: xdg_config_home.as_deref(),
            zdotdir: zdotdir.as_deref(),
        };
        let mut path = vec![PathBuf::from("/usr/local/bin"), PathBuf::from("/usr/bin")];
        if case.flag("on_path") == Some(true) {
            path.insert(1, env.bin_dir());
        }
        env.path = &path;

        let line = || fill("line").ok_or("the case has no line");
        let expected = match case.text("outcome") {
            Some("on-path") => PathAction::OnPath,
            Some("already") => PathAction::AlreadyConfigured { file: need("configured_by")? },
            Some("unknown") => PathAction::Unknown { line: line()? },
            Some("add") => {
                let file = need("file")?;
                let shell = match file.file_name().and_then(|name| name.to_str()) {
                    Some(".bashrc") => Shell::Bash,
                    Some(".zshrc") => Shell::Zsh,
                    _ => Shell::Fish,
                };
                PathAction::Add { shell, file, line: line()? }
            }
            other => return Err(format!("unknown outcome {other:?}")),
        };
        let action = decide(&env, read_file);
        if action != expected {
            return Err(format!("decided {action:?}, the table expects {expected:?}"));
        }

        let PathAction::Add { file: added, line, .. } = action else { return Ok(()) };
        let shown_file = fill("shown_file").ok_or("the case has no shown_file")?;
        if shown(&added, &home) != shown_file {
            return Err(format!("shown as {}, the table expects {shown_file}", shown(&added, &home)));
        }
        apply(&added, &line).map_err(|error| format!("apply failed: {error}"))?;
        let after = fs::read_to_string(&added).map_err(|error| error.to_string())?;
        if Some(&after) != fill("after").as_ref() {
            return Err(format!("the file holds {after:?}, the table expects {:?}", fill("after")));
        }
        if path_of("link_to").is_some() {
            let kept = fs::symlink_metadata(&added).is_ok_and(|meta| meta.file_type().is_symlink());
            if !kept {
                return Err("the link was replaced by a file".to_owned());
            }
        }
        // Once added, the same rules find the line and add nothing more.
        match decide(&env, read_file) {
            PathAction::AlreadyConfigured { file } if file == added => Ok(()),
            again => Err(format!("after adding, decided {again:?}")),
        }
    }

    #[test]
    fn every_case_of_the_shared_table_holds() {
        let document = Document::parse("path-cases.toml", CASES, &shape());
        let problems: Vec<String> = document.diagnostics().iter().map(ToString::to_string).collect();
        assert!(problems.is_empty(), "the table does not read cleanly:\n{}", problems.join("\n"));
        let cases = document.root().entries("case");
        assert!(cases.len() > 30, "the table lost its cases: {}", cases.len());
        let failures: Vec<String> = cases
            .iter()
            .filter_map(|case| run_case(case).err().map(|why| format!("{}: {why}", case.text("name").unwrap_or("?"))))
            .collect();
        assert!(failures.is_empty(), "{} of {} cases failed:\n{}", failures.len(), cases.len(), failures.join("\n"));
    }

    #[test]
    fn a_read_only_file_is_an_error_naming_it() {
        let temp = tempfile::tempdir().expect("temporary folder");
        let file = temp.path().join(".bashrc");
        fs::write(&file, "alias ll='ls -l'\n").expect("write the file");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o444)).expect("make it read-only");

        let error = apply(&file, "export PATH=\"$HOME/.cargo/bin:$PATH\"").expect_err("a read-only file is refused");
        assert_eq!(error.path, file);
        assert_eq!(error.error.kind(), io::ErrorKind::PermissionDenied);
        assert!(error.to_string().starts_with(&file.display().to_string()));
        assert_eq!(fs::read_to_string(&file).expect("read it back"), "alias ll='ls -l'\n");
    }

    #[test]
    fn a_folder_that_cannot_be_created_is_an_error_naming_it() {
        let temp = tempfile::tempdir().expect("temporary folder");
        // A file where the folder should be: creating the folder fails for any user.
        let blocker = temp.path().join(".config");
        fs::write(&blocker, "").expect("write the blocking file");
        let file = blocker.join("fish/config.fish");

        let error = apply(&file, "fish_add_path $HOME/.cargo/bin").expect_err("the folder cannot be created");
        assert_eq!(error.path, blocker.join("fish"));
        assert_eq!(fs::read_to_string(&blocker).expect("read it back"), "");
    }

    #[test]
    fn a_folder_in_place_of_the_file_is_an_error_naming_it() {
        let temp = tempfile::tempdir().expect("temporary folder");
        let file = temp.path().join(".zshrc");
        fs::create_dir(&file).expect("make a folder");

        let error = apply(&file, "export PATH=\"$HOME/.cargo/bin:$PATH\"").expect_err("a folder is not a file");
        assert_eq!(error.path, file);
    }

    #[test]
    fn paths_under_home_are_shown_with_a_tilde() {
        let home = Path::new("/home/ada");
        assert_eq!(shown(Path::new("/home/ada/.config/fish/config.fish"), home), "~/.config/fish/config.fish");
        assert_eq!(shown(home, home), "~");
        assert_eq!(shown(Path::new("/home/adam/.bashrc"), home), "/home/adam/.bashrc");
        assert_eq!(shown(Path::new("/opt/cargo/bin"), home), "/opt/cargo/bin");
    }
}
