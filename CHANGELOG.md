# Changelog

What changed in each release of quvyta. Versions follow [Semantic Versioning](https://semver.org/); while the version starts with 0, a minor release may change how things look or where they are kept.

## 0.2.3 (2026-09-19)

- The README opens with what quvyta is for in one sentence, and shows the Settings tab.
- This changelog, a contributing guide, and templates for bug reports and ideas.

## 0.2.2 (2026-09-19)

- A Settings tab beside the family list, for quvyta's own settings: whether it looks for updates at start, what happens when a program opened from quvyta closes, and whether `~/.cargo/bin` is on your PATH, with Add when it is not. Every change is saved at once; if the file cannot be written, the setting goes back and quvyta says why.
- The tests pass when `CARGO_TARGET_DIR` is set in the shell that runs them.

## 0.2.1 (2026-09-19)

- `install.sh` works on macOS: the PATH line goes to `.zprofile` for zsh or `.bash_profile` for bash, the Command Line Tools are checked as the linker, and the members that run only on Arch Linux are skipped.
- `install.ps1` installs the family on Windows from PowerShell, without an administrator: Rust through rustup, the members from crates.io, and cargo's folder on your user PATH only after asking. If the Visual Studio C++ Build Tools are missing it prints the command that installs them.
- A command line: `quvyta --help`, `quvyta --version`, and `quvyta install NAME...`, which opens on the install question of each member named.
- The install and remove questions show cargo's whole command: the dialog widens to fit it, and on a narrow screen the command wraps.
- Requires quvyta-framework 0.1.7.

## 0.2.0 (2026-09-18)

- quvyta installs, opens, updates and removes the family's programs. Before an install it shows the source, the version, where the program goes and the exact cargo command; installs run one at a time in a queue, can be stopped, and a failure is told in a plain sentence with cargo's last lines and a kept log.
- It reads what is installed from cargo's own record, and finds members installed some other way on your PATH.
- It asks crates.io for newer versions at most once every six hours; **Install updates** updates them all, and `u` updates one.
- An installed member opens from the list; when it closes, quvyta comes back (or quits, with `after_close = "shell"`).
- It offers to put `~/.cargo/bin` on your PATH, with the same rules as `install.sh`.

## 0.1.2 (2026-09-18)

- A one-line installer, `install.sh`: it lists the family, installs Rust with rustup if cargo is missing, installs the members you pick with cargo, and adds cargo's folder to your shell's PATH only if you agree.
- qpackages' command is `qpac`.

## 0.1.1 (2026-09-18)

- The family's applications are shown as betas, each with the command that installs it.
- A beta note and the family table in the README.

## 0.1.0 (2026-09-17)

- The first release: a terminal app that introduces the Quvyta family, with what each member does, its package and its command, in English and Turkish.
