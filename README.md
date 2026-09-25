# quvyta

One terminal app to find, install, open and update everything in the Quvyta ecosystem.

[![crates.io](https://img.shields.io/crates/v/quvyta.svg)](https://crates.io/crates/quvyta)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![A visit in quvyta: choosing qtools in the list, agreeing to the install after seeing the exact command, cargo building it, and Open on it at the end](https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/quvyta.gif)

![The Quvyta apps in quvyta: what is installed, which version, and one update waiting](https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/apps.png)

**quvyta** installs the terminal applications of the Quvyta ecosystem, opens the ones on your
machine, and keeps them up to date. It lists every app with what it does, which version you have and
whether a newer one is out; it installs, updates and removes members with cargo, after showing
exactly what it will run, and offers to put cargo's folder on your PATH when a new terminal could
not find them. Every member is built on [quvyta-framework](https://github.com/quvyta/framework), so
they look and behave alike: the same themes, icons, languages, keys and mouse behaviour. quvyta is
open source under the MIT licence.

> **Beta.** quvyta is new, and so is the ecosystem it introduces. The interface may still change
> between releases. Please report anything
> that looks wrong at <https://github.com/quvyta/quvyta/issues>.

The first start asks what every Quvyta app should look like — the language, the colour theme and the icons, with **Start with the defaults** for anyone who would rather not choose — and then which of the apps you would like. Nothing is written until you finish it, and nothing is ticked for you.

<p>
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/wizard.png" alt="The first start: the language, theme and icons every Quvyta app shares, each with a box saying where the choice holds, and Start with the defaults" width="49%">
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/wizard-apps.png" alt="The first start asking which Quvyta apps you would like, with a box beside each and nothing chosen" width="49%">
</p>

<p>
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/install-confirm.png" alt="Before an install: the source, the version, where it goes and the exact command" width="49%">
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/installing.png" alt="An install in progress, with cargo's own output under Details and another member queued" width="49%">
</p>
<p>
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/path-notice.png" alt="After an install: the offer to put cargo's folder on PATH" width="49%">
<img src="https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/apps-tr.png" alt="The same list in Turkish, in the Nordic theme" width="49%">
</p>

## The apps

| App | Crate | Program | Install | Status |
|---|---|---|---|---|
| [Code](https://github.com/quvyta/code) | `quvyta-code` | `qcode` | `cargo install quvyta-code` | Beta |
| [Focus](https://github.com/quvyta/focus) | `quvyta-focus` | `qfocus` | `cargo install quvyta-focus` | Beta |
| [Tools](https://github.com/quvyta/tools) | `quvyta-tools` | `qtools` | `cargo install quvyta-tools` | Beta |
| [Packages](https://github.com/quvyta/packages) | `quvyta-packages` | `qpac` | `cargo install quvyta-packages` | Beta |
| [Desktop](https://github.com/quvyta/desktop) | `quvyta-desktop` | `qdesk` | `cargo install quvyta-desktop` | Beta |
| [Framework](https://github.com/quvyta/framework) showcase | `quvyta-framework-showcase` | `qframe` | `cargo install quvyta-framework-showcase` | Released |
| [quvyta](https://github.com/quvyta/quvyta) | `quvyta` | `quvyta` | `cargo install quvyta` | Beta |

- **qcode** runs coding agents inside Podman or Docker containers.
- **qfocus** tracks what you focus on and shows where your time went in charts.
- **qtools** applies the settings Arch Linux users usually set up by hand, from one list, with a
  preview, a backup and an undo.
- **qpackages** is a package manager for Arch Linux that shows exactly what will change before
  anything does.
- **qdesk** is a desktop inside the terminal, with windows, icons, a dock and a launcher, made
  for working over SSH and on small machines.
- **qframe** shows off the framework every member is built on; the framework itself is a Rust
  library you add to a project with `cargo add quvyta-framework`.

Each application is also installed under its long name (`quvyta-code`, `quvyta-focus` and so on).

## Install

One line installs any Quvyta app. Without a name it lists them all and lets you choose:

```sh
curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh
```

With names it installs just those, for example qcode, or quvyta itself:

```sh
curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh -s -- code
curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh -s -- quvyta
```

The names are `framework` (the framework's showcase, `qframe`), `code`, `focus`, `packages`, `tools`, `desk` (qdesk), `quvyta` and `all`; several can be given at once.

What the script does, asking before each step:

1. If cargo is missing, it offers to install Rust with rustup, from rustup's own address.
2. It builds the chosen members from crates.io with `cargo install`.
3. If `~/.cargo/bin` is not on your PATH, it shows the line it would add to your shell's start-up file (fish, bash or zsh) and adds it only if you agree. A line that is already there is not added again.
4. It lists the commands it installed and tells you whether a new terminal is needed.

It uses sudo only for one thing: when Rust's C linker is missing it shows the command that installs it on your system (pacman, apt or dnf) and offers to run it, and only a yes typed on the terminal runs it; `--yes` never does, and the answer is no by default. Questions are read from the terminal even though the script arrives through a pipe; with no terminal it changes nothing and only says what it would do. `--yes` answers every question with yes, and `--help` explains the options.

On macOS the same line works. The PATH line goes to `.zprofile` for zsh or `.bash_profile` for bash, the Command Line Tools are checked because Rust links with them (if they are missing the script shows `xcode-select --install` and offers to open Apple's installer), and qpackages and qtools, which run on Arch Linux only, are skipped.

On Windows, in PowerShell:

```powershell
irm https://raw.githubusercontent.com/quvyta/quvyta/main/install.ps1 | iex
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/quvyta/quvyta/main/install.ps1))) code -Yes
```

The second form passes names and options. It needs no administrator: Rust comes from rustup, the chosen members from crates.io, and the only setting it writes is your own user PATH, after you agree to the folder it shows. If the Visual Studio C++ Build Tools that Rust links with are missing, it shows the winget command that installs them and offers to run it (Windows asks for permission itself; only a yes typed in the console runs it, never `-Yes`), then asks you to open a new terminal and run it again. `-Help` explains the options.

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

Rust 1.95 or later is required. The interface follows your system language (English, Turkish,
German, Spanish, French, Brazilian Portuguese, Russian, Simplified Chinese and Japanese are
included). `↑` `↓` move through the list, `enter` opens the selected member or installs one
you do not have, `u` updates it, `r` looks for updates again, `tab` moves between controls and
`ctrl+q` quits. On a narrow terminal the first `enter` shows a member's details and `esc` goes
back to the list. The header has two tabs, Apps and Settings: click one, or reach them with `tab`
and move with `←` `→`. Settings opens with the appearance every Quvyta app
shares: the language, the colour theme and the icon set, each with a box saying whether the
choice holds in every Quvyta application or in quvyta alone, and under them Reduce motion and
the pillar. Below them are quvyta's own choices (checking for updates at start, what happens
when a program closes, and whether `~/.cargo/bin` is on your PATH). `↑` `↓` pick a row, `enter`
or `space` changes it, every change is saved at once and takes effect straight away, and `esc`
goes back to Apps. An app that follows the shared settings opens with what you chose here.

`quvyta show NAME` opens straight on one member's page, for example `quvyta show qfocus`; on a
narrow terminal `esc` goes back to the list. `quvyta install NAME...` opens on the question
before installing each member named, one after another. A name is a member's short name, its
command or its package, and `quvyta --help` lists every form.

![The Settings tab: the language, theme and icons every Quvyta app shares, each with the box for changing it everywhere, and under them checking for updates at start, what happens when an app closes, and whether cargo's folder is on PATH](https://raw.githubusercontent.com/quvyta/quvyta/main/docs/screenshots/settings.png)

## Installing, updating and removing

Before anything happens quvyta shows where a member comes from, which version it gets, where the
program goes and the exact cargo command, which you can copy. Members are built on your machine
with `cargo install --locked`, one at a time; while one builds you can keep using the list, and
more members wait their turn. Nothing needs sudo: everything is written under `~/.cargo`. If
Rust or a C linker is missing, the question says so with the command that puts it right, and
**Install here** runs that command in the terminal you are looking at; sudo, when the command
needs it, asks for your password there itself. If a
build fails, quvyta says why in a plain sentence, shows cargo's last lines and keeps the whole
log. If `~/.cargo/bin` is not on your PATH, it shows the line for your shell's start-up file and
adds it if you agree.

When quvyta starts it asks crates.io for the newest versions of the Quvyta apps, at most once a day
(see [What quvyta sends over the network](#what-quvyta-sends-over-the-network)). A member with a
newer version says so in the list, and **Install updates** updates them all. Members installed some other way than cargo are left to whatever installed
them. quvyta can update itself; the new version runs the next time you start it.

**Remove** deletes a member's program with `cargo uninstall`. Its settings stay where they are, so
installing it again brings them back. quvyta does not remove itself.

An opened member gets the whole terminal and starts in your home folder; when it closes, quvyta
comes back. To go straight back to your shell instead, choose it on the Settings tab, or put this
in quvyta's settings file, `~/.config/quvyta/launcher.conf` on Linux:

```toml
after_close = "shell"
```

quvyta learns what is installed from `cargo install --list`, cargo's own record. A member's command
found elsewhere on your PATH counts as installed too, with an unknown version; quvyta only looks at
the file and never runs a member to find out.

## What quvyta sends over the network

quvyta itself makes one kind of request: when it starts, at most once a day, it runs `cargo search quvyta`, which asks crates.io for the newest versions of the packages whose name starts with `quvyta`. Only that word goes into the question; nothing about you, your machine or what you have installed. The answer is kept in quvyta's data folder (`latest.toml`), so the other starts that day ask nothing. Without a network the question fails quietly and the last answer stands.

This is the shared update notice of the Quvyta ecosystem, and it is on by default. One switch turns it off for every Quvyta application at once: **Say when an update is out** on the Settings tab, or this line in the shared file, `~/.config/quvyta/quvyta.conf` on Linux:

```toml
update-notice = false
```

With it off, quvyta asks nothing when it starts; pressing `r` still asks, since that is you asking. If you turned off quvyta's own `check_updates` in an earlier version, that choice is kept: the first start of this version turns the shared switch off and takes the old line out of `launcher.conf`.

Everything else that reaches the network is a command you asked for and saw first: `cargo install`, which downloads a member from crates.io, and **Install here**, which runs the line shown in the question.

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

## Contributing

Bug reports, ideas and pull requests are welcome; [CONTRIBUTING.md](CONTRIBUTING.md) explains how.
What changed in each release is in [CHANGELOG.md](CHANGELOG.md).

## Licence

MIT. See [LICENSE](LICENSE).
