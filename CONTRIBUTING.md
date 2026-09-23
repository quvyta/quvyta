# Contributing to quvyta

Thank you for taking the time. Bug reports, ideas and pull requests are all welcome. quvyta is small and still a beta, so an issue that says "this confused me" is as useful as a patch.

## Reporting a problem

Open an issue at <https://github.com/quvyta/quvyta/issues> and pick **Bug report**. The template asks for what helps most: the quvyta version (`quvyta --version`), your operating system, your terminal and its size, whether icons look right, and what you did and saw. For a failed install, the log quvyta kept is the most useful thing you can attach; the error screen says where it is.

For something quvyta should do but does not, pick **Idea**.

## Building and testing

The toolchain is pinned by `rust-toolchain.toml`, so rustup picks the right Rust by itself.

```sh
git clone https://github.com/quvyta/quvyta
cd quvyta
git config core.hooksPath .githooks
cargo run
```

The hook runs on every commit and checks formatting, clippy with warnings as errors, the tests and the docs. Please do not skip it; if it fails, the message says which step.

The tests never install anything for real. They build a machine in a temporary folder and put a stand-in for cargo there (`tests/fake-cargo/cargo`), so your own `~/.cargo`, shell files and settings are never touched. Please keep it that way in new tests.

To look at every screen at once, in both languages and at several sizes:

```sh
QUVYTA_REVIEW=1 cargo test visual_review
```

This writes `target/quvyta-review.html`, which you can open in a browser.

### The install scripts

`install.sh` is checked by `cargo test` through `tests/install/run.sh`, which runs it in temporary home folders with stand-ins for cargo and rustup. To try another shell, pass it: `sh tests/install/run.sh dash`.

`install.ps1` is checked in a PowerShell container with podman, without Windows and without a network:

```sh
sh tests/install-ps1/run.sh
```

When a member is added or renamed, both scripts, `src/ecosystem.rs`, the language files and the README's table of apps change together; a test fails if the two scripts disagree.

## Writing code

- Text people read lives in the language files, `assets/locales/en.toml` and `assets/locales/tr.toml`, never in the code. A new sentence needs both; if you do not speak Turkish, write the English one and say so in the pull request.
- quvyta is built on [quvyta-framework](https://github.com/quvyta/framework). Widgets, themes, icons, keys and mouse behaviour come from there; if something is missing, it is usually better added to the framework than written here.
- A file quvyta reads never makes it panic: a broken setting falls back to its default and quvyta says where the problem is.
- Comments say why the code is the way it is.
- A bug fix comes with a test that fails without it.

## Pull requests

Keep a pull request to one change, describe what it does and why, and include a picture or the output of the visual review if it changes a screen. By contributing you agree that your work is released under the MIT licence of this repository.
