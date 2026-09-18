# quvyta

**quvyta** introduces the Quvyta family of terminal applications. It shows each member, what it
does, where its source lives and the command that installs it, and copies either with one key or
click. Every member is built on [quvyta-framework](https://github.com/quvyta/framework), so they
look and behave alike: the same themes, icons, languages, keys and mouse behaviour. quvyta is open
source under the MIT licence.

> **Beta.** quvyta is new, and so is the family it introduces. Today the application only shows
> the members and how to install them, and the install script below does the installing;
> installing from inside the application, opening and shared settings are planned. The interface may
> still change between releases. Please report anything that looks wrong at
> <https://github.com/quvyta/quvyta/issues>.

## The family

| Member | Crate | Program | Install | Status |
|---|---|---|---|---|
| [Framework](https://github.com/quvyta/framework) | `quvyta-framework` | — | `cargo add quvyta-framework` | Released |
| [Code](https://github.com/quvyta/code) | `quvyta-code` | `qcode` | `cargo install quvyta-code` | Beta |
| [Focus](https://github.com/quvyta/focus) | `quvyta-focus` | `qfocus` | `cargo install quvyta-focus` | Beta |
| [Tools](https://github.com/quvyta/tools) | `quvyta-tools` | `qtools` | `cargo install quvyta-tools` | Beta |
| [Packages](https://github.com/quvyta/packages) | `quvyta-packages` | `qpac` | `cargo install quvyta-packages` | Beta |

- **qcode** runs coding agent harnesses inside Podman or Docker containers.
- **qfocus** tracks what you focus on and shows where your time went in charts.
- **qtools** applies the settings Arch Linux users usually set up by hand, from one list, with a
  preview, a backup and an undo.
- **qpackages** is a package manager for Arch Linux that shows exactly what will change before
  anything does.

Each application is also installed under its long name (`quvyta-code`, `quvyta-focus` and so on).

## Install

One line installs any member of the family. Without a name it lists the family and lets you choose:

```sh
curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh
```

With names it installs just those, for example qcode, or quvyta itself:

```sh
curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh -s -- code
curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh -s -- quvyta
```

The names are `framework` (the framework's showcase, `qframe`), `code`, `focus`, `packages`, `tools`, `quvyta` and `all`; several can be given at once.

What the script does, asking before each step:

1. If cargo is missing, it offers to install Rust with rustup, from rustup's own address.
2. It builds the chosen members from crates.io with `cargo install`.
3. If `~/.cargo/bin` is not on your PATH, it shows the line it would add to your shell's start-up file (fish, bash or zsh) and adds it only if you agree. A line that is already there is not added again.
4. It lists the commands it installed and tells you whether a new terminal is needed.

It never uses sudo. Questions are read from the terminal even though the script arrives through a pipe; with no terminal it changes nothing and only says what it would do. `--yes` answers every question with yes, and `--help` explains the options.

To read the script before running it:

```sh
curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh -o install.sh
less install.sh
sh install.sh
```

Or install with cargo yourself:

```sh
cargo install quvyta
quvyta
```

If the shell then says the command is not found, add `~/.cargo/bin` to your PATH (fish: `fish_add_path ~/.cargo/bin`).

Rust 1.95 or later is required. The interface follows your system language (English and Turkish
are included). `←` `→` switch between members, `tab` moves between controls and `ctrl+q` quits.

## Building from source

The toolchain is pinned by `rust-toolchain.toml`.

```sh
git clone https://github.com/quvyta/quvyta
cd quvyta
cargo run
```

Before your first commit, enable the checks (formatting, clippy, tests and docs):

```sh
git config core.hooksPath .githooks
```

## Licence

MIT. See [LICENSE](LICENSE).
