# quvyta

![The family in quvyta: what is installed, which version, and one update waiting](https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/family.png)

**quvyta** installs the Quvyta family of terminal applications, opens the ones on your machine,
and keeps them up to date. It lists every member with what it does, which version you have and
whether a newer one is out; it installs, updates and removes members with cargo, after showing
exactly what it will run, and offers to put cargo's folder on your PATH when a new terminal could
not find them. Every member is built on [quvyta-framework](https://github.com/quvyta/framework), so
they look and behave alike: the same themes, icons, languages, keys and mouse behaviour. quvyta is
open source under the MIT licence.

> **Beta.** quvyta is new, and so is the family it introduces. Shared settings for the whole
> family are planned. The interface may still change between releases. Please report anything
> that looks wrong at <https://github.com/quvyta/quvyta/issues>.

<p>
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/install-confirm.png" alt="Before an install: the source, the version, where it goes and the exact command" width="49%">
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/installing.png" alt="An install in progress, with cargo's own output under Details and another member queued" width="49%">
</p>
<p>
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/path-notice.png" alt="After an install: the offer to put cargo's folder on PATH" width="49%">
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/family-tr.png" alt="The same list in Turkish, in the Nordic theme" width="49%">
</p>

## The family

| Member | Crate | Program | Install | Status |
|---|---|---|---|---|
| [Code](https://github.com/quvyta/code) | `quvyta-code` | `qcode` | `cargo install quvyta-code` | Beta |
| [Focus](https://github.com/quvyta/focus) | `quvyta-focus` | `qfocus` | `cargo install quvyta-focus` | Beta |
| [Tools](https://github.com/quvyta/tools) | `quvyta-tools` | `qtools` | `cargo install quvyta-tools` | Beta |
| [Packages](https://github.com/quvyta/packages) | `quvyta-packages` | `qpac` | `cargo install quvyta-packages` | Beta |
| [Framework](https://github.com/quvyta/framework) showcase | `quvyta-framework-showcase` | `qframe` | `cargo install quvyta-framework-showcase` | Released |
| [quvyta](https://github.com/quvyta/quvyta) | `quvyta` | `quvyta` | `cargo install quvyta` | Beta |

- **qcode** runs coding agent harnesses inside Podman or Docker containers.
- **qfocus** tracks what you focus on and shows where your time went in charts.
- **qtools** applies the settings Arch Linux users usually set up by hand, from one list, with a
  preview, a backup and an undo.
- **qpackages** is a package manager for Arch Linux that shows exactly what will change before
  anything does.
- **qframe** shows off the framework every member is built on; the framework itself is a Rust
  library you add to a project with `cargo add quvyta-framework`.

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
are included). `↑` `↓` move through the list, `enter` opens the selected member or installs one
you do not have, `u` updates it, `r` looks for updates again, `tab` moves between controls and
`ctrl+q` quits. On a narrow terminal the first `enter` shows a member's details and `esc` goes
back to the list.

## Installing, updating and removing

Before anything happens quvyta shows where a member comes from, which version it gets, where the
program goes and the exact cargo command, which you can copy. Members are built on your machine
with `cargo install --locked`, one at a time; while one builds you can keep using the list, and
more members wait their turn. Nothing needs sudo: everything is written under `~/.cargo`. If a
build fails, quvyta says why in a plain sentence, shows cargo's last lines and keeps the whole
log. If `~/.cargo/bin` is not on your PATH, it shows the line for your shell's start-up file and
adds it if you agree.

When quvyta starts it asks crates.io, through `cargo search`, for the newest versions, at most
once every six hours. A member with a newer version says so in the list, and **Install updates**
updates them all. Members installed some other way than cargo are left to whatever installed
them. quvyta can update itself; the new version runs the next time you start it.

**Remove** deletes a member's program with `cargo uninstall`. Its settings stay where they are, so
installing it again brings them back. quvyta does not remove itself.

An opened member gets the whole terminal and starts in your home folder; when it closes, quvyta
comes back. To go straight back to your shell instead, put this in quvyta's settings file,
`~/.config/quvyta/launcher.conf` on Linux:

```toml
after_close = "shell"
```

To stop quvyta from asking crates.io for updates when it starts (`r` still asks), add:

```toml
check_updates = false
```

quvyta learns what is installed from `cargo install --list`, cargo's own record. A member's command
found elsewhere on your PATH counts as installed too, with an unknown version; quvyta only looks at
the file and never runs a member to find out.

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
