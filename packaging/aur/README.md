# AUR drafts

These are draft `PKGBUILD` files for putting the Quvyta apps on the [AUR](https://aur.archlinux.org), Arch Linux's user repository. **Nothing here is published.** No AUR package exists, no account has been made, and no upload will happen without the owner's explicit approval. Until then this folder is only a prepared answer to "could we ship this on Arch?".

Today the way to install the Quvyta apps on Arch is `cargo install`, or the `quvyta` program itself, which does that for you.

There is one folder per released member:

| Folder | Programs it installs | Source |
|---|---|---|
| `quvyta/` | `quvyta` | <https://github.com/quvyta/quvyta> |
| `quvyta-code/` | `qcode`, `quvyta-code` | <https://github.com/quvyta/code> |
| `quvyta-focus/` | `qfocus`, `quvyta-focus` | <https://github.com/quvyta/focus> |
| `quvyta-packages/` | `qpac`, `quvyta-packages` | <https://github.com/quvyta/packages> |
| `quvyta-tools/` | `qtools`, `quvyta-tools` | <https://github.com/quvyta/tools> |
| `quvyta-framework-showcase/` | `qframe`, `quvyta-framework-showcase` | <https://github.com/quvyta/framework> |

Two Quvyta apps are deliberately missing:

- **`quvyta-desktop` (`qdesk`)** is not released. It is listed among the Quvyta apps as coming soon and there is nothing on crates.io to build from, so there is nothing to package. It gets a folder when it is published.
- **`quvyta-framework`** is a Rust library, not a program. Arch does not package Rust libraries; people who build against it get it from crates.io. The showcase, which is a program, stands for it here.

## What a PKGBUILD here does

Each one builds the published crate from its crates.io tarball, the way the [Rust package guidelines](https://wiki.archlinux.org/title/Rust_package_guidelines) describe:

- `prepare()` runs `cargo fetch --locked` once, so `build()` can run with `--frozen` and no network.
- `build()` runs `cargo build --frozen --release --all-features`.
- `package()` installs each program with `install -Dm0755`, plus the licence and the README.
- `CARGO_HOME` is only set when the packager has not set one, so the build never writes into a home folder it was not given. `RUSTUP_TOOLCHAIN=stable` keeps a rustup user off whatever their default toolchain is. `options=('!lto')` leaves link-time optimisation to cargo; makepkg's `CFLAGS` and `LDFLAGS` still reach any C a dependency compiles.

There is no `check()`, and that is on purpose. The crate on crates.io does not carry its tests: each `Cargo.toml` has an `include` list that keeps the `tests/` folder and its fixtures out of the tarball, so `cargo test` there fails to compile before it runs a single test. The tests run in the source repository instead, where the commit gate makes them pass before there is a release to package at all. If a member's `include` list ever grows to carry its tests, a `check()` with `cargo test --frozen --all-features` belongs back in that PKGBUILD.

## Filling in the checksum

`sha256sums` is a row of zeros on purpose: it is a placeholder, not a value, so nobody builds an unverified tarball by accident. Whoever prepares a release sets `pkgver` to the version that is on crates.io and then runs, in that package's folder:

```sh
updpkgsums
```

It downloads the tarball and writes the real checksum into the `PKGBUILD`. `updpkgsums` comes from the `pacman-contrib` package.

## Trying one without touching your machine

Never run `makepkg` or `pacman` on the machine you are working on while these are drafts. Build them in a container that you throw away afterwards. `makepkg` refuses to run as root, so the container makes a normal user first:

```sh
podman run --rm -it \
  -v "$PWD/packaging/aur/quvyta:/pkg:ro" \
  docker.io/library/archlinux bash -c '
    pacman -Syu --noconfirm --needed base-devel rust pacman-contrib &&
    useradd -m builder &&
    cp /pkg/PKGBUILD /home/builder/ &&
    chown -R builder /home/builder &&
    su builder -c "cd ~ && updpkgsums && makepkg -s --noconfirm"
  '
```

Run it from the top of the repository, and change `quvyta` in the volume path for another member. It downloads the whole Rust toolchain into the container each time, so give it a while. `updpkgsums` fills in the placeholder checksum inside the container, so the drafts here stay as they are. To check only that the file parses, which needs no network beyond the base image:

```sh
podman run --rm -v "$PWD/packaging/aur/quvyta:/pkg:ro" docker.io/library/archlinux \
  bash -c 'pacman -Syu --noconfirm --needed base-devel >/dev/null &&
           useradd -m builder && cp /pkg/PKGBUILD /home/builder/ &&
           chown -R builder /home/builder &&
           su builder -c "cd ~ && makepkg --printsrcinfo"'
```

## What a real AUR release would still need

All of it waits for the owner's approval. When that comes:

1. An AUR account with an SSH public key uploaded to it. The AUR has no web upload; git over SSH is the only way in.
2. For each package, a clone of its AUR repository over SSH. Putting the account's user name in `~/.ssh/config` keeps it out of every command:

   ```
   Host aur.archlinux.org
       User aur
       IdentityFile ~/.ssh/aur
   ```

   then `git clone aur.archlinux.org:<pkgname>.git`. The repository is empty until the first push, and the name must be free.
3. `.SRCINFO` generated beside the `PKGBUILD` and committed with it, in the same commit. The AUR reads the package's metadata from that file, not from the `PKGBUILD`:

   ```sh
   makepkg --printsrcinfo > .SRCINFO
   ```

4. Only the `PKGBUILD`, the `.SRCINFO` and any files the build needs go in that repository; nothing else.
5. Every new version repeats it: bump `pkgver`, reset `pkgrel` to `1`, `updpkgsums`, regenerate `.SRCINFO`, build it once in a clean container, then push.

The order the Quvyta apps publish in is GitHub releases and crates.io first, then the AUR, then Debian.
