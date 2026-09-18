# quvyta

**quvyta** introduces the Quvyta family of terminal applications. It shows each member, what it
does, where its source lives and the command that installs it, and copies either with one key or
click. Every member is built on [quvyta-framework](https://github.com/quvyta/framework), so they
look and behave alike: the same themes, icons, languages, keys and mouse behaviour. quvyta is open
source under the MIT licence.

> **Beta.** quvyta is new, and so is the family it introduces. Today it only shows the members
> and how to install them; installing, opening and shared settings are planned. The interface may
> still change between releases. Please report anything that looks wrong at
> <https://github.com/quvyta/quvyta/issues>.

## The family

| Member | Crate | Program | Install | Status |
|---|---|---|---|---|
| [Framework](https://github.com/quvyta/framework) | `quvyta-framework` | — | `cargo add quvyta-framework` | Released |
| [Code](https://github.com/quvyta/code) | `quvyta-code` | `qcode` | `cargo install quvyta-code` | Beta |
| [Focus](https://github.com/quvyta/focus) | `quvyta-focus` | `qfocus` | `cargo install quvyta-focus` | Beta |
| [Tools](https://github.com/quvyta/tools) | `quvyta-tools` | `qtools` | `cargo install quvyta-tools` | Beta |
| [Packages](https://github.com/quvyta/packages) | `quvyta-packages` | `qpackages` | `cargo install quvyta-packages` | Beta |

- **qcode** runs coding agent harnesses inside Podman or Docker containers.
- **qfocus** tracks what you focus on and shows where your time went in charts.
- **qtools** applies the settings Arch Linux users usually set up by hand, from one list, with a
  preview, a backup and an undo.
- **qpackages** is a package manager for Arch Linux that shows exactly what will change before
  anything does.

Each application is also installed under its long name (`quvyta-code`, `quvyta-focus` and so on).

## Install

```sh
cargo install quvyta
quvyta
```

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
