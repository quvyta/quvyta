# quvyta

**quvyta** introduces the Quvyta family of terminal applications and will install them. Every
member is built on [quvyta-framework](https://github.com/quvyta/framework), so they look and
behave alike: the same themes, icons, languages, keys and mouse behaviour.

Today it shows each member of the family, what it does and whether it is released, and lets you
copy its source address or install command with one key or click.

| Member | Package | Command | Status |
|---|---|---|---|
| Framework | `quvyta-framework` | — | Released |
| Code | `quvyta-code` | `qcode` | In the works |
| Focus | `quvyta-focus` | `qfocus` | In the works |
| Tools | `quvyta-tools` | `qtools` | In the works |
| Packages | `quvyta-packages` | `qpackages` | In the works |

## Install

```sh
cargo install quvyta
quvyta
```

The interface follows your system language (English and Turkish are included). `←`/`→` switch
between members, `tab` moves between controls and `ctrl+q` quits.

## Building from source

The toolchain is pinned by `rust-toolchain.toml`. Before your first commit, enable the checks
(formatting, clippy, tests and docs):

```sh
git config core.hooksPath .githooks
cargo run
```

## Licence

MIT. See [LICENSE](LICENSE).
