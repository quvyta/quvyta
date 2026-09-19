//! Reading what cargo prints. Pure functions over its text, tested with recorded output.
//!
//! On a terminal cargo colours its words and redraws its progress line in place; [`plain`]
//! takes those escapes out, and the rest reads the plain lines.

/// One package in the output of `cargo install --list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// The package name, such as `quvyta-code`.
    pub package: String,
    /// Its version without the leading `v`, such as `0.1.1`.
    pub version: String,
    /// The commands it put in cargo's `bin` folder.
    pub commands: Vec<String>,
}

/// The packages in the output of `cargo install --list`.
///
/// Each package is a line such as `quvyta-code v0.1.1:`, or `quvyta v0.1.2 (/home/x/quvyta):`
/// for one installed from a folder or a git repository, followed by its commands, indented.
/// Lines that fit neither shape are skipped, so a warning cargo prints first does no harm.
pub fn parse_install_list(text: &str) -> Vec<Installed> {
    let mut packages: Vec<Installed> = Vec::new();
    // Commands belong to the package line above them; after a line that is not one, they
    // belong to nothing.
    let mut current = false;
    for line in text.lines() {
        if line.starts_with(char::is_whitespace) {
            let command = line.trim();
            if current
                && !command.is_empty()
                && let Some(package) = packages.last_mut()
            {
                package.commands.push(command.to_owned());
            }
            continue;
        }
        current = match package_line(line) {
            Some(package) => {
                packages.push(package);
                true
            }
            None => false,
        };
    }
    packages
}

/// `name vX.Y.Z:` or `name vX.Y.Z (source):`.
fn package_line(line: &str) -> Option<Installed> {
    let line = line.trim_end().strip_suffix(':')?;
    let mut words = line.splitn(3, ' ');
    let package = words.next()?;
    let version = words.next()?.strip_prefix('v')?;
    let valid = !package.is_empty()
        && version.starts_with(|c: char| c.is_ascii_digit())
        && words.next().is_none_or(|source| source.starts_with('(') && source.ends_with(')'));
    valid.then(|| Installed { package: package.to_owned(), version: version.to_owned(), commands: Vec::new() })
}

/// One crate in the output of `cargo search`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The package name, such as `quvyta-code`.
    pub package: String,
    /// Its newest version on crates.io, such as `0.1.2`.
    pub version: String,
}

/// The crates in the output of `cargo search`: lines such as
/// `quvyta-tools = "0.1.2"    # Applies the recommended Arch Linux settings…`. The note cargo
/// adds after them and any other line are skipped. A version is kept only when it is made of the
/// characters a version may have, since it goes into a file and onto cargo's command line.
pub fn parse_search(text: &str) -> Vec<Found> {
    text.lines().filter_map(search_line).collect()
}

fn search_line(line: &str) -> Option<Found> {
    let (package, rest) = line.split_once(" = \"")?;
    let (version, _) = rest.split_once('"')?;
    let name = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_');
    let versionish = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+');
    let valid = !package.is_empty()
        && package.chars().all(name)
        && version.starts_with(|c: char| c.is_ascii_digit())
        && version.chars().all(versionish);
    valid.then(|| Found { package: package.to_owned(), version: version.to_owned() })
}

/// `line` without the terminal escapes cargo writes on a terminal: colours, erasing the line and
/// the links around words (`ESC ] 8 ;; url ESC \\`).
pub fn plain(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.next() {
            // A control sequence ends with its first byte in `@`..`~`.
            Some('[') => {
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            // An operating system command ends with BEL or with ESC `\`.
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\u{7}' {
                        break;
                    }
                    if c == '\u{1b}' {
                        chars.next_if_eq(&'\\');
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Where an install is, from one plain line of `cargo install`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Updating the index or fetching sources: `Updating`, `Downloading`, `Downloaded`, and the
    /// early `Installing <package> v<version>` that names what is about to be built.
    Downloading,
    /// A `Compiling <crate> v<version>` line: the crate being built, with no count.
    Compiling {
        /// The crate's name.
        krate: String,
    },
    /// cargo's progress line, `Building [==>   ] 142/231: ratatui, serde`.
    Counted {
        /// Units built.
        done: u32,
        /// Units in the build.
        total: u32,
        /// The first crate being built.
        krate: String,
    },
    /// Putting the program in place: `Installing <path>` or `Replacing <path>`.
    Placing,
}

/// The step a plain line of `cargo install` shows, or `None` for a line that says nothing about
/// where the install is.
pub fn step(line: &str) -> Option<Step> {
    let line = line.trim();
    let (word, rest) = line.split_once(' ').map_or((line, ""), |(word, rest)| (word, rest.trim()));
    match word {
        "Updating" | "Downloading" | "Downloaded" => Some(Step::Downloading),
        // The first `Installing` names the package before anything is built; the last one names
        // the file it puts in place.
        "Installing" | "Replacing" if rest.starts_with('/') => Some(Step::Placing),
        "Installing" => Some(Step::Downloading),
        "Compiling" => rest
            .split(' ')
            .next()
            .filter(|name| !name.is_empty())
            .map(|name| Step::Compiling { krate: name.to_owned() }),
        "Building" => counted(rest),
        _ => None,
    }
}

/// `[==>   ] 142/231: ratatui, serde` after the word `Building`.
fn counted(rest: &str) -> Option<Step> {
    let (_, after) = rest.split_once(']')?;
    let (counts, names) = after.trim().split_once(':').unwrap_or((after.trim(), ""));
    let (done, total) = counts.trim().split_once('/')?;
    let (done, total) = (done.parse().ok()?, total.parse().ok()?);
    // A build script shows as `serde(build)`; the crate is what the user knows.
    let first = names.split(',').next().unwrap_or("").trim();
    let krate = first.split('(').next().unwrap_or(first).trim().to_owned();
    (total > 0 && done <= total).then_some(Step::Counted { done, total, krate })
}

/// The version of `package` an install put in place, from cargo's plain output: the last
/// `` `package vX.Y.Z` `` or `Installing package vX.Y.Z` it names.
pub fn installed_version(package: &str, lines: &[String]) -> Option<String> {
    let quoted = format!("`{package} v");
    let named = format!("Installing {package} v");
    lines.iter().rev().find_map(|line| {
        let start = line.rfind(&quoted).map(|at| at + quoted.len()).or_else(|| {
            line.trim_start().strip_prefix(&named).map(|_| line.len() - line.trim_start().len() + named.len())
        })?;
        let version: String = line[start..].chars().take_while(|c| !matches!(c, '`' | ' ')).collect();
        version.starts_with(|c: char| c.is_ascii_digit()).then_some(version)
    })
}

/// Why an install failed, as far as cargo's output tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// There is no C linker to finish the build with.
    NoLinker,
    /// The package needs a newer Rust than the one installed.
    OldRust,
    /// crates.io could not be reached.
    Network,
    /// The disk filled up.
    DiskFull,
    /// crates.io has no such package or version.
    NotFound,
    /// Anything else: the build itself failed.
    Build,
}

/// What went wrong, from the plain lines of a failed `cargo install`. The most specific cause
/// wins: a full disk explains the errors it causes, and network retries that succeeded still
/// leave their warnings behind.
pub fn failure(lines: &[String]) -> Failure {
    let any = |needles: &[&str]| lines.iter().any(|line| needles.iter().any(|needle| line.contains(needle)));
    if any(&["No space left on device", "os error 28"]) {
        Failure::DiskFull
    } else if lines.iter().any(|line| line.contains("linker `") && line.contains("` not found")) {
        Failure::NoLinker
    } else if any(&["requires rustc", "not stabilized in this version of Cargo"]) {
        Failure::OldRust
    } else if lines.iter().any(|line| line.contains("could not find `") && line.contains("in registry")) {
        Failure::NotFound
    } else if network(lines) {
        Failure::Network
    } else {
        Failure::Build
    }
}

/// Whether the error, or what caused it, is a network one. Warnings about retries do not count
/// on their own: cargo retries and often gets through.
fn network(lines: &[String]) -> bool {
    let Some(first_error) = lines.iter().position(|line| line.starts_with("error")) else { return false };
    lines[first_error..]
        .iter()
        .filter(|line| !line.starts_with("warning"))
        .any(|line| NETWORK.iter().any(|needle| line.contains(needle)))
}

/// What cargo and curl say when crates.io cannot be reached.
const NETWORK: [&str; 8] = [
    "download of config.json failed",
    "failed to query replaced source registry",
    "Couldn't resolve host",
    "Could not resolve host",
    "failed to download",
    "Couldn't connect to server",
    "failed to connect",
    "network failure",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(package: &str, version: &str, commands: &[&str]) -> Installed {
        Installed {
            package: package.to_owned(),
            version: version.to_owned(),
            commands: commands.iter().map(|command| (*command).to_owned()).collect(),
        }
    }

    #[test]
    fn packages_and_their_commands_are_read() {
        let text = "\
bat v0.25.0:
    bat
quvyta-code v0.1.1:
    qcode
    quvyta-code
quvyta-framework-showcase v0.1.4:
    qframe
    quvyta-framework-showcase
";
        assert_eq!(
            parse_install_list(text),
            [
                installed("bat", "0.25.0", &["bat"]),
                installed("quvyta-code", "0.1.1", &["qcode", "quvyta-code"]),
                installed("quvyta-framework-showcase", "0.1.4", &["qframe", "quvyta-framework-showcase"]),
            ]
        );
    }

    #[test]
    fn packages_from_a_folder_or_git_keep_their_version() {
        let text = "\
quvyta v0.1.2 (/home/ayse/quvyta):
    quvyta
ripgrep v14.1.1 (https://github.com/BurntSushi/ripgrep#4649aa99):
    rg
";
        assert_eq!(
            parse_install_list(text),
            [installed("quvyta", "0.1.2", &["quvyta"]), installed("ripgrep", "14.1.1", &["rg"])]
        );
    }

    #[test]
    fn an_empty_list_and_stray_lines_give_nothing_wrong() {
        assert_eq!(parse_install_list(""), []);
        let text = "\
warning: be sure to add `/home/ayse/.cargo/bin` to your PATH
    qcode
quvyta-focus v0.1.1:
    qfocus
not a package line:
    stray
quvyta-tools 0.1.1:
    qtools
";
        assert_eq!(parse_install_list(text), [installed("quvyta-focus", "0.1.1", &["qfocus"])]);
    }

    #[test]
    fn windows_line_endings_are_read_too() {
        let text = "quvyta-code v0.1.1:\r\n    qcode\r\n";
        assert_eq!(parse_install_list(text), [installed("quvyta-code", "0.1.1", &["qcode"])]);
    }

    /// What `cargo search quvyta --limit 20` printed, recorded.
    const SEARCH: &str = include_str!("../tests/cargo-output/search.txt");

    fn found(package: &str, version: &str) -> Found {
        Found { package: package.to_owned(), version: version.to_owned() }
    }

    #[test]
    fn a_recorded_search_gives_every_crate_and_its_version() {
        let crates = parse_search(SEARCH);
        assert_eq!(crates.len(), 8, "{crates:?}");
        assert!(crates.contains(&found("quvyta-code", "0.1.1")), "{crates:?}");
        assert!(crates.contains(&found("quvyta-tools", "0.1.2")), "{crates:?}");
        assert!(crates.iter().any(|entry| entry.package == "quvyta-packages-core"), "every crate is read: {crates:?}");
    }

    #[test]
    fn search_lines_in_every_shape() {
        let text = "\
quvyta-focus = \"0.1.2\"                 # Focus tracking in the terminal
quvyta-next = \"0.2.0-beta.1\"
... and 3 crates more (use --limit N to see more)
note: to learn more about a package, run `cargo info <name>`
bad name = \"0.1.0\"
quvyta-odd = \"v1\"
quvyta-quote = \"0.1.0\\\" # x\"
quvyta-inject = \"0.1.0\n\"
";
        assert_eq!(parse_search(text), [found("quvyta-focus", "0.1.2"), found("quvyta-next", "0.2.0-beta.1")]);
        assert_eq!(parse_search(""), []);
    }

    /// An install of `quvyta-tools`, written in the byte format of the recorded one below:
    /// colours, `\r` progress lines erased with `ESC [K`, `\r\n` line ends.
    const INSTALL: &str = include_str!("../tests/cargo-output/install-ok.txt");

    /// Every line a terminal shows over time, the ones overwritten in place included.
    fn shown(text: &str) -> Vec<String> {
        text.split(['\r', '\n']).map(plain).filter(|line| !line.trim().is_empty()).collect()
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(ToOwned::to_owned).collect()
    }

    #[test]
    fn escapes_are_taken_out_and_text_is_kept() {
        assert_eq!(plain("\u{1b}[1m\u{1b}[92m   Compiling\u{1b}[0m serde v1.0.219"), "   Compiling serde v1.0.219");
        assert_eq!(plain("\u{1b}[K    Finished"), "    Finished");
        let link = "\u{1b}]8;;https://doc.rust-lang.org/cargo/reference/profiles.html\u{1b}\\`release` profile\u{1b}]8;;\u{1b}\\ done";
        assert_eq!(plain(link), "`release` profile done");
        assert_eq!(plain("bell \u{1b}]0;title\u{7}after"), "bell after");
        assert_eq!(plain("çalışıyor ✓"), "çalışıyor ✓");
        assert_eq!(plain("cut \u{1b}["), "cut ");
    }

    #[test]
    fn a_recorded_install_goes_through_the_steps_in_order() {
        let steps: Vec<Step> = shown(INSTALL).iter().filter_map(|line| step(line)).collect();
        assert_eq!(steps.first(), Some(&Step::Downloading));
        assert_eq!(steps.last(), Some(&Step::Placing));
        assert!(steps.contains(&Step::Compiling { krate: "proc-macro2".to_owned() }), "{steps:?}");
        assert!(
            steps.contains(&Step::Counted { done: 142, total: 231, krate: "ratatui".to_owned() }),
            "the progress line gives counts and the first crate: {steps:?}"
        );
        let placing = steps.iter().position(|step| *step == Step::Placing).expect("placing");
        let compiling = steps.iter().rposition(|step| matches!(step, Step::Compiling { .. })).expect("compiling");
        assert!(compiling < placing, "the early `Installing quvyta-tools v0.1.2` is not the placing: {steps:?}");
    }

    /// What `cargo install --locked hexyl` wrote on a 120 by 30 pseudo-terminal, byte for byte,
    /// recorded in a disposable container: progress frames that end in `\r` and are followed
    /// either by the next frame or by `ESC [K` and the line that replaces them.
    const RECORDED: &str = include_str!("../tests/cargo-output/install-pty.txt");

    #[test]
    fn every_progress_frame_of_a_recorded_install_gives_its_counts() {
        let counted: Vec<(u32, u32, String)> = shown(RECORDED)
            .iter()
            .filter_map(|line| match step(line) {
                Some(Step::Counted { done, total, krate }) => Some((done, total, krate)),
                _ => None,
            })
            .collect();
        let frames = RECORDED.matches("Building\u{1b}[0m [").count();
        assert_eq!(counted.len(), frames, "{counted:?}");
        assert_eq!(counted.first(), Some(&(0, 46, "anstyle".to_owned())), "names cut with `...` still give the first");
        assert_eq!(counted.last(), Some(&(45, 46, "hexyl".to_owned())), "`hexyl(bin)` is the crate `hexyl`");
        assert!(counted.iter().all(|(_, total, _)| *total == 46), "{counted:?}");
        let steps: Vec<Step> = shown(RECORDED).iter().filter_map(|line| step(line)).collect();
        assert_eq!(steps.first(), Some(&Step::Downloading));
        assert_eq!(steps.last(), Some(&Step::Placing));
        assert_eq!(installed_version("hexyl", &shown(RECORDED)).as_deref(), Some("0.17.0"));
    }

    #[test]
    fn progress_lines_in_every_shape() {
        let counted = |done, total, krate: &str| Some(Step::Counted { done, total, krate: krate.to_owned() });
        assert_eq!(step("    Building [=====>      ] 142/231: ratatui, serde"), counted(142, 231, "ratatui"));
        assert_eq!(step("    Building [>  ] 3/231: serde(build), libc(build.rs)"), counted(3, 231, "serde"));
        assert_eq!(step("    Building [===] 12/12"), counted(12, 12, ""));
        assert_eq!(step("    Building [==>] 13/12: nonsense"), None);
        assert_eq!(step("    Building [==>] x/12: nonsense"), None);
        assert_eq!(step("    Building"), None);
        assert_eq!(step("   Replacing /home/ayse/.cargo/bin/qtools"), Some(Step::Placing));
        assert_eq!(step("    Finished `release` profile [optimized] target(s) in 1m 32s"), None);
        assert_eq!(step("warning: unused variable"), None);
        assert_eq!(step(""), None);
    }

    #[test]
    fn the_installed_version_comes_from_the_last_line_that_names_it() {
        assert_eq!(installed_version("quvyta-tools", &shown(INSTALL)).as_deref(), Some("0.1.2"));
        let replaced = lines(
            "  Installing quvyta-tools v0.1.2\n   Replacing /home/ayse/.cargo/bin/qtools\n    Replaced package `quvyta-tools v0.1.1` with `quvyta-tools v0.1.2` (executable `qtools`)",
        );
        assert_eq!(installed_version("quvyta-tools", &replaced).as_deref(), Some("0.1.2"));
        let ignored = lines("     Ignored package `quvyta-tools v0.1.2` is already installed, use --force to override");
        assert_eq!(installed_version("quvyta-tools", &ignored).as_deref(), Some("0.1.2"));
        assert_eq!(installed_version("quvyta", &shown(INSTALL)), None, "another package's name is not a match");
        assert_eq!(installed_version("quvyta-tools", &[]), None);
    }

    #[test]
    fn failures_are_told_apart() {
        let linker = "\
   Compiling proc-macro2 v1.0.95
error: linker `cc` not found
  |
  = note: No such file or directory (os error 2)

error: could not compile `proc-macro2` (build script) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
error: failed to compile `quvyta-tools v0.1.2`, intermediate artifacts can be found at `/home/ayse/.local/share/quvyta/build`.";
        assert_eq!(failure(&lines(linker)), Failure::NoLinker);

        let old = "\
    Updating crates.io index
error: cannot install package `quvyta-tools 0.1.2`, it requires rustc 1.95 or newer, while the currently active rustc version is 1.80.1";
        assert_eq!(failure(&lines(old)), Failure::OldRust);
        let old_cargo = "\
error: failed to parse manifest at `/home/ayse/.cargo/registry/src/index.crates.io-6f17d22bba15001f/quvyta-tools-0.1.2/Cargo.toml`

Caused by:
  feature `edition2024` is required

  The package requires the Cargo feature called `edition2024`, but that feature is not stabilized in this version of Cargo (1.80.1).";
        assert_eq!(failure(&lines(old_cargo)), Failure::OldRust);

        let offline = "\
    Updating crates.io index
warning: spurious network error (3 tries remaining): [6] Couldn't resolve host name (Could not resolve host: index.crates.io)
warning: spurious network error (2 tries remaining): [6] Couldn't resolve host name (Could not resolve host: index.crates.io)
error: download of config.json failed

Caused by:
  failed to download from `https://index.crates.io/config.json`

Caused by:
  [6] Couldn't resolve host name (Could not resolve host: index.crates.io)";
        assert_eq!(failure(&lines(offline)), Failure::Network);

        let disk = "\
   Compiling ratatui v0.29.0
LLVM ERROR: IO failure on output stream: No space left on device
error: could not compile `ratatui` (lib)";
        assert_eq!(failure(&lines(disk)), Failure::DiskFull);

        let missing = "\
    Updating crates.io index
error: could not find `quvyta-toolz` in registry `crates-io` with version `*`";
        assert_eq!(failure(&lines(missing)), Failure::NotFound);

        let build = "\
   Compiling quvyta-tools v0.1.2
warning: spurious network error (2 tries remaining): [7] Couldn't connect to server
error[E0425]: cannot find value `x` in this scope
error: could not compile `quvyta-tools` (bin \"qtools\") due to 1 previous error";
        assert_eq!(failure(&lines(build)), Failure::Build, "a retry that got through is not the cause");
        assert_eq!(failure(&[]), Failure::Build);
    }
}
