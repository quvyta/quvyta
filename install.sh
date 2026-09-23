#!/bin/sh
# Installs Quvyta apps with cargo, on Linux and macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh -s -- code
#
# Run `sh install.sh --help` for the options. Rust comes from rustup's own address and the
# applications from crates.io. Shell start-up files are only written after you agree to the exact
# line shown. sudo is used only when a C linker is missing and you say yes, on the terminal, to
# the one command shown that installs it; --yes never says that yes for you.
#
# Everything lives in functions and runs from the last line, so a download cut short midway
# runs nothing at all.

set -u

# Every member published crates for, at least this Rust.
min_rust_major=1
min_rust_minor=95

names="framework code focus packages tools quvyta desk"

# Members that are not released yet: there is nothing on crates.io to install. Naming one says so
# and installs nothing for it; they are left out of all and out of the picker's numbers.
# The day qdesk is published, take desk out of this one line. Two more places change with it:
# Soon = $false in install.ps1's list, and Status::Soon -> Status::Beta in src/ecosystem.rs.
soon="desk"

crate_of() {
    case $1 in
        framework) echo quvyta-framework-showcase ;;
        quvyta) echo quvyta ;;
        desk) echo quvyta-desktop ;;
        *) echo "quvyta-$1" ;;
    esac
}

command_of() {
    case $1 in
        framework) echo qframe ;;
        code) echo qcode ;;
        focus) echo qfocus ;;
        packages) echo qpac ;;
        tools) echo qtools ;;
        quvyta) echo quvyta ;;
        desk) echo qdesk ;;
    esac
}

about() {
    case $1 in
        framework) echo "the showcase of the framework every member is built on" ;;
        code) echo "coding agents inside Podman or Docker containers" ;;
        focus) echo "tracks what you focus on and where your time went" ;;
        packages) echo "a package manager for Arch Linux that shows every change first" ;;
        tools) echo "the settings Arch Linux users usually set up by hand, with undo" ;;
        quvyta) echo "installs, opens, updates and removes the Quvyta apps" ;;
        desk) echo "a desktop inside the terminal: windows, a dock, a launcher and a file manager" ;;
    esac
}

# Whether a member is one of the names above that cannot be installed yet.
unreleased() {
    case " $soon " in
        *" $1 "*) return 0 ;;
    esac
    return 1
}

# The one line a person sees for a member that is not out yet, in the words quvyta itself uses.
say_soon() {
    say "$(command_of "$1") is not released yet, so it cannot be installed; quvyta's own list shows it as coming soon."
}

# Members that only run on Arch Linux; they are not built on macOS.
arch_only() {
    case $1 in
        packages | tools) return 0 ;;
    esac
    return 1
}

on_mac() {
    [ "$os" = Darwin ]
}

# Whether a member can be installed on this system.
supported() {
    ! { on_mac && arch_only "$1"; }
}

usage() {
    cat <<EOF
Installs Quvyta apps with cargo, on Linux and macOS.

Usage:
  curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh
  curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh -s -- [options] [names]
  sh install.sh [options] [names]

Names (several may be given; none lets you choose):
  framework   quvyta-framework-showcase, command qframe
  code        quvyta-code, command qcode
  focus       quvyta-focus, command qfocus
  packages    quvyta-packages, command qpac; Arch Linux only
  tools       quvyta-tools, command qtools; Arch Linux only
  quvyta      quvyta, command quvyta
  desk        quvyta-desktop, command qdesk; not released yet, so it cannot be installed
  all         every one of the above that can be installed

Options:
  -y, --yes   agree to every question: installing Rust with rustup, installing the
              chosen crates and adding cargo's folder to PATH in your shell's start-up file
  -h, --help  show this text

Questions are read from the terminal, so they work when the script comes through a pipe.
Without a terminal and without --yes nothing is installed or written; the script only
says what it would do.

When Rust's C linker is missing, the script shows the command that installs it and
offers to run it. That command uses sudo, or on macOS opens Apple's own installer, so
only a yes typed on the terminal runs it; --yes does not, and the answer is no by default.
EOF
}

say() { printf '%s\n' "$*"; }
warn() { printf '%s\n' "$*" >&2; }

# Whether a question can be asked: /dev/tty has to open, not just exist, because a process
# without a controlling terminal still sees the device node.
have_tty() {
    (: </dev/tty) 2>/dev/null
}

# ask QUESTION -> 0 yes, 1 no, 2 cannot ask. Enter means yes.
ask() {
    if [ "$assume_yes" = 1 ]; then
        return 0
    fi
    if ! have_tty; then
        return 2
    fi
    printf '%s (Y/n) ' "$1" >/dev/tty
    answer=
    read -r answer </dev/tty || answer=n
    case $answer in
        '' | y | Y | yes | Yes | YES) return 0 ;;
        *) return 1 ;;
    esac
}

# ask_system QUESTION -> 0 yes, 1 no, 2 cannot ask. For the steps that change the system itself,
# such as a command run with sudo: only a yes typed on the terminal counts, so --yes does not
# answer it and Enter means no.
ask_system() {
    if [ "$assume_yes" = 1 ] || ! have_tty; then
        return 2
    fi
    printf '%s (y/N) ' "$1" >/dev/tty
    answer=
    read -r answer </dev/tty || answer=n
    case $answer in
        y | Y | yes | Yes | YES) return 0 ;;
        *) return 1 ;;
    esac
}

is_name() {
    for known in $names; do
        [ "$1" = "$known" ] && return 0
    done
    return 1
}

# Adds a name to $chosen once.
choose() {
    case " $chosen " in
        *" $1 "*) ;;
        *) chosen="${chosen:+$chosen }$1" ;;
    esac
}

parse_args() {
    assume_yes=0
    show_help=0
    chosen=
    soon_named=
    for arg in "$@"; do
        case $arg in
            -y | --yes) assume_yes=1 ;;
            -h | --help) show_help=1 ;;
            all) for name in $names; do unreleased "$name" || choose "$name"; done ;;
            -*)
                warn "Unknown option: $arg"
                warn "Run with --help to see the options."
                return 2
                ;;
            *)
                if is_name "$arg"; then
                    if unreleased "$arg"; then
                        soon_named="${soon_named:+$soon_named }$arg"
                    else
                        choose "$arg"
                    fi
                else
                    warn "Unknown name: $arg"
                    warn "Known names: $names (or all)."
                    return 2
                fi
                ;;
        esac
    done
    return 0
}

list_apps() {
    number=1
    for name in $names; do
        text=$(about "$name")
        # A member still to come has no number: the picker installs, and this cannot be installed.
        if unreleased "$name"; then
            printf '     %-10s %-8s %s\n' "$name" "$(command_of "$name")" "$text; coming soon, not released yet"
            continue
        fi
        supported "$name" || text="$text; Arch Linux only, not for macOS"
        printf '  %s  %-10s %-8s %s\n' "$number" "$name" "$(command_of "$name")" "$text"
        number=$((number + 1))
    done
}

# Turns the reply of the picker (numbers, names or "all") into $chosen.
pick_from() {
    for word in $1; do
        case $word in
            all) for name in $names; do unreleased "$name" || choose "$name"; done ;;
            *[!0-9]*)
                if ! is_name "$word"; then
                    warn "Unknown choice: $word"
                    return 1
                fi
                if unreleased "$word"; then
                    say_soon "$word"
                else
                    choose "$word"
                fi
                ;;
            *)
                number=1
                found=
                for name in $names; do
                    unreleased "$name" && continue
                    if [ "$number" = "$word" ]; then
                        choose "$name"
                        found=1
                    fi
                    number=$((number + 1))
                done
                if [ -z "$found" ]; then
                    warn "Unknown choice: $word"
                    return 1
                fi
                ;;
        esac
    done
    return 0
}

pick() {
    say "The Quvyta apps:"
    list_apps
    if ! have_tty; then
        say ""
        say "There is no terminal to choose on. Name the members instead, for example:"
        say "  curl -fsSL https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh | sh -s -- code focus"
        return 1
    fi
    while :; do
        printf 'Which ones? Numbers or names, separated by spaces, or all: ' >/dev/tty
        reply=
        read -r reply </dev/tty || reply=
        if [ -z "$reply" ]; then
            say "Nothing chosen, nothing installed."
            return 1
        fi
        chosen=
        pick_from "$reply" && [ -n "$chosen" ] && return 0
    done
}

# Leaves out of $chosen the members this system cannot run, saying so for each.
drop_unsupported() {
    kept=
    for name in $chosen; do
        if supported "$name"; then
            kept="${kept:+$kept }$name"
        else
            say "Skipping $name ($(command_of "$name")): it runs on Arch Linux only."
        fi
    done
    chosen=$kept
    if [ -z "$chosen" ]; then
        say "Nothing left to install on macOS."
        return 1
    fi
    return 0
}

download() {
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSf "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget --https-only --secure-protocol=TLSv1_2 -qO- "$1"
    else
        warn "Neither curl nor wget is available to download rustup."
        return 1
    fi
}

ensure_cargo() {
    if command -v cargo >/dev/null 2>&1; then
        return 0
    fi
    # rustup may have been installed earlier without its folder on PATH yet.
    if [ -x "$bin_dir/cargo" ]; then
        PATH="$bin_dir:$PATH"
        export PATH
        return 0
    fi
    say ""
    say "cargo, Rust's package tool, is not installed. The Quvyta applications are built with it."
    say "rustup, the official Rust installer, can install it for you into $rustup_home and $cargo_home."
    say "It is downloaded from https://sh.rustup.rs and does not need sudo."
    ask "Install Rust with rustup now?"
    case $? in
        0) ;;
        1)
            say "Rust was not installed. Install it from https://rustup.rs and run this script again."
            return 1
            ;;
        *)
            say "There is no terminal to ask on, so nothing was installed. To install Rust yourself:"
            say "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
            say "then run this script again, or run it with --yes to let it install Rust."
            return 1
            ;;
    esac
    # Its own start-up file changes are turned off: the PATH step below asks before writing.
    if ! download https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal; then
        warn "rustup could not install Rust."
        return 1
    fi
    PATH="$bin_dir:$PATH"
    export PATH
    command -v cargo >/dev/null 2>&1 || {
        warn "rustup finished but cargo is still not found in $bin_dir."
        return 1
    }
    say ""
    say "Rust is installed. rustup's note about PATH is taken care of at the end."
}

ensure_rust_version() {
    version=$(cargo --version 2>/dev/null | sed -n 's/^cargo \([0-9][0-9]*\)\.\([0-9][0-9]*\).*/\1 \2/p')
    [ -n "$version" ] || return 0
    # shellcheck disable=SC2086 # the two numbers are split on purpose
    set -- $version
    if [ "$1" -gt "$min_rust_major" ] || { [ "$1" -eq "$min_rust_major" ] && [ "$2" -ge "$min_rust_minor" ]; }; then
        return 0
    fi
    say ""
    say "Rust $1.$2 is installed; the Quvyta applications need $min_rust_major.$min_rust_minor or later."
    if command -v rustup >/dev/null 2>&1; then
        ask "Update it with rustup update stable?"
        case $? in
            0) rustup update stable </dev/null && return 0 ;;
            2) say "There is no terminal to ask on. Run: rustup update stable" ;;
        esac
    else
        say "Update Rust with the tool you installed it with, then run this script again."
    fi
    return 1
}

# The distribution /etc/os-release names in ID or ID_LIKE: arch, debian, fedora or other.
# The same table as quvyta's own check, which a test keeps in step.
distro() {
    file=${QUVYTA_OS_RELEASE:-/etc/os-release}
    ids=
    if [ -r "$file" ]; then
        for key in ID ID_LIKE; do
            ids="$ids $(sed -n "s/^[[:space:]]*$key=//p" "$file" | sed -n "1{s/[\"']//g;p;}")"
        done
    fi
    # shellcheck disable=SC2086 # split into words on purpose, tabs and all
    set -- $ids
    case " $* " in
        *" arch "*) echo arch ;;
        *" debian "* | *" ubuntu "*) echo debian ;;
        *" fedora "* | *" rhel "*) echo fedora ;;
        *) echo other ;;
    esac
}

# The distribution's name as people write it.
distro_name() {
    case $1 in
        arch) echo "Arch Linux" ;;
        debian) echo "Debian, Ubuntu" ;;
        fedora) echo "Fedora" ;;
    esac
}

# The command that installs a C linker there.
linker_command() {
    case $1 in
        arch) echo "sudo pacman -S --needed base-devel" ;;
        debian) echo "sudo apt install build-essential" ;;
        fedora) echo "sudo dnf install gcc" ;;
    esac
}

have_linker() {
    for linker in cc gcc clang; do
        command -v "$linker" >/dev/null 2>&1 && return 0
    done
    return 1
}

# Every known command, for a distribution this script does not know or a person who did not
# want one run.
say_linker_commands() {
    say "Install one with your package manager, then run this script again, for example:"
    for known in arch debian fedora; do
        printf '  %-16s %s\n' "$(distro_name "$known"):" "$(linker_command "$known")"
    done
}

ensure_linker() {
    if on_mac; then
        # /usr/bin/cc is there even without the Command Line Tools: it is a stand-in that opens
        # an install dialog instead of linking, so only xcode-select tells whether they are there.
        xcode-select -p >/dev/null 2>&1 && return 0
        say ""
        say "Rust needs Apple's Command Line Tools to link programs, and they are not installed."
        say "This command opens Apple's installer for them:"
        say "  xcode-select --install"
        ask_system "Open it now?"
        case $? in
            0)
                if xcode-select --install </dev/null; then
                    say "Confirm in the window that opened. Once the tools are installed, run this script again."
                else
                    say "The installer did not open. Run the command above yourself, confirm in the window that opens, then run this script again."
                fi
                ;;
            *) say "Run it, confirm in the window that opens, then run this script again." ;;
        esac
        # The tools install in Apple's own window, which this script cannot wait for.
        return 1
    fi
    have_linker && return 0
    say ""
    say "Rust needs a C linker to build programs, and none was found (cc, gcc or clang)."
    found_distro=$(distro)
    linker_cmd=$(linker_command "$found_distro")
    if [ -z "$linker_cmd" ] || [ "$assume_yes" = 1 ] || ! have_tty; then
        say_linker_commands
        return 1
    fi
    say "On $(distro_name "$found_distro") this command installs one:"
    say "  $linker_cmd"
    say "sudo will ask for your password here, in this terminal."
    ask_system "Run it now?"
    if [ $? != 0 ]; then
        say "Nothing was run. Install a C linker, then run this script again."
        return 1
    fi
    # The package manager asks its own questions, and they have to come from the person, not from
    # the rest of this script when it arrives through a pipe.
    # shellcheck disable=SC2086 # the command is split into its words on purpose
    $linker_cmd </dev/tty
    if have_linker; then
        say "A C linker is installed."
        return 0
    fi
    say "There is still no C linker (cc, gcc or clang). Install one, then run this script again."
    return 1
}

install_chosen() {
    say ""
    say "These will be built from crates.io and installed into $bin_dir:"
    for name in $chosen; do
        printf '  %-27s command %s\n' "$(crate_of "$name")" "$(command_of "$name")"
    done
    say "Building takes a few minutes for each."
    ask "Install them now?"
    case $? in
        0) ;;
        1)
            say "Nothing was installed."
            return 1
            ;;
        *)
            say "There is no terminal to ask on, so nothing was installed. Run with --yes, or yourself:"
            for name in $chosen; do
                say "  cargo install --locked $(crate_of "$name")"
            done
            return 1
            ;;
    esac
    installed=
    failed=
    for name in $chosen; do
        crate=$(crate_of "$name")
        say ""
        say "Installing $crate..."
        if cargo install --locked "$crate" </dev/null; then
            installed="${installed:+$installed }$name"
        else
            failed="${failed:+$failed }$crate"
        fi
    done
    return 0
}

detect_shell() {
    shell_name=${SHELL:-}
    shell_name=${shell_name##*/}
    case $shell_name in
        fish | bash | zsh) echo "$shell_name" ;;
        # zsh is macOS's own shell, so it is the best guess there.
        *) if on_mac; then echo zsh; else echo other; fi ;;
    esac
}

# The start-up file and the line that puts cargo's folder on PATH, for the detected shell.
# Paths under the home folder are written with $HOME so the file still reads well.
path_line_for() {
    shown_dir=$bin_dir
    case $bin_dir in
        "$HOME"/*) shown_dir="\$HOME${bin_dir#"$HOME"}" ;;
    esac
    case $1 in
        fish)
            rc_file="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"
            path_line="fish_add_path $shown_dir"
            ;;
        zsh)
            # macOS terminals open login shells, which read .zprofile; Linux ones read .zshrc.
            if on_mac; then
                rc_file="${ZDOTDIR:-$HOME}/.zprofile"
            else
                rc_file="${ZDOTDIR:-$HOME}/.zshrc"
            fi
            path_line="export PATH=\"$shown_dir:\$PATH\""
            ;;
        bash)
            # A login bash, as macOS terminals start, reads .bash_profile and not .bashrc.
            if on_mac; then
                rc_file="$HOME/.bash_profile"
            else
                rc_file="$HOME/.bashrc"
            fi
            path_line="export PATH=\"$shown_dir:\$PATH\""
            ;;
        *)
            rc_file=
            path_line="export PATH=\"$shown_dir:\$PATH\""
            ;;
    esac
}

# Whether the start-up file already puts cargo's folder on PATH, either with our line or with
# the one rustup writes itself.
rc_has_path() {
    [ -f "$1" ] || return 1
    grep -qxF "$2" "$1" && return 0
    case $bin_dir in
        "$HOME"/.cargo/bin) grep -qF '.cargo/env' "$1" && return 0 ;;
    esac
    return 1
}

# rustup keeps fish's line in a file of its own, which fish reads at start-up.
fish_rustup_file() {
    echo "${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d/rustup.fish"
}

fish_has_rustup_path() {
    [ "$bin_dir" = "$HOME/.cargo/bin" ] && [ -f "$(fish_rustup_file)" ]
}

# Sets path_state: on-path (nothing to do), next-terminal (a start-up file adds the folder, so a
# new terminal finds the programs) or by-hand (the line still has to be added).
ensure_path() {
    path_state=on-path
    case ":$start_path:" in
        *":$bin_dir:"*) return 0 ;;
    esac
    path_state=by-hand
    shell=$(detect_shell)
    path_line_for "$shell"
    say ""
    say "$bin_dir, where cargo puts programs, is not on your PATH yet."
    if [ -z "$rc_file" ]; then
        say "Your shell (${SHELL:-unknown}) is not one this script knows. Add this line to its start-up file:"
        say "  $path_line"
        return 0
    fi
    if rc_has_path "$rc_file" "$path_line"; then
        say "$rc_file already adds it; a new terminal will pick it up."
        path_state=next-terminal
        return 0
    fi
    if [ "$shell" = fish ] && fish_has_rustup_path; then
        say "$(fish_rustup_file) already adds it; a new terminal will pick it up."
        path_state=next-terminal
        return 0
    fi
    say "This line would be added to $rc_file:"
    say "  $path_line"
    ask "Add it?"
    case $? in
        0)
            mkdir -p "$(dirname "$rc_file")" &&
                printf '\n# cargo programs, added by the Quvyta installer\n%s\n' "$path_line" >>"$rc_file" &&
                say "Added to $rc_file." && path_state=next-terminal && return 0
            warn "Could not write to $rc_file. Add the line yourself."
            ;;
        1) say "Not added. Add the line yourself to run the programs by name." ;;
        *) say "There is no terminal to ask on, so $rc_file was not changed. Add the line yourself." ;;
    esac
    return 0
}

summary() {
    say ""
    if [ -n "$installed" ]; then
        say "Installed:"
        for name in $installed; do
            printf '  %-8s %s\n' "$(command_of "$name")" "$(about "$name")"
        done
    fi
    if [ -n "$failed" ]; then
        say "Could not install: $failed. The messages above say why."
    fi
    if [ -n "$installed" ]; then
        case $path_state in
            on-path) say "Run a command above by its name to start it." ;;
            next-terminal) say "Open a new terminal, then run a command above by its name." ;;
            *) say "Until the line above is added, start them by their full path, such as $bin_dir/$(command_of "${installed%% *}")." ;;
        esac
    fi
}

main() {
    os=$(uname -s 2>/dev/null) || os=
    parse_args "$@" || exit 2
    if [ "$show_help" = 1 ]; then
        usage
        exit 0
    fi

    cargo_home=${CARGO_HOME:-$HOME/.cargo}
    rustup_home=${RUSTUP_HOME:-$HOME/.rustup}
    bin_dir="$cargo_home/bin"
    start_path=$PATH
    installed=
    failed=
    path_state=on-path

    if [ -n "$soon_named" ]; then
        for name in $soon_named; do say_soon "$name"; done
        # Named on its own, nothing is left to install; ending here also keeps the picker away.
        [ -n "$chosen" ] || exit 1
    fi
    if [ -z "$chosen" ]; then
        pick || exit 1
    fi
    drop_unsupported || exit 1
    ensure_cargo || exit 1
    ensure_rust_version || exit 1
    ensure_linker || exit 1
    install_chosen || exit 1
    if [ -n "$installed" ]; then
        ensure_path
    fi
    summary
    [ -z "$failed" ]
}

main "$@"
