#!/bin/sh
# Checks install.sh without a network and without touching the real home folder.
#
# Every case runs in a fresh temporary HOME with CARGO_HOME inside it and a PATH that holds only
# a few basic tools plus stand-ins for cargo, rustup's download and a C linker. The stand-ins
# write down what they were asked to do, so a case can check that nothing was installed.
#
# Usage: tests/install/run.sh [shell]   (the shell that runs install.sh; default sh)
set -u

here=$(cd "$(dirname "$0")" && pwd)
script="$here/../../install.sh"
shell=${1:-sh}
shell_path=$(command -v "$shell") || { echo "no $shell to test with" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# The basic tools install.sh relies on, and nothing else from the machine.
tools="$work/tools"
mkdir -p "$tools"
for tool in grep sed basename dirname mkdir cat; do
    ln -s "$(command -v "$tool")" "$tools/$tool"
done
ln -s "$shell_path" "$tools/sh"

# Tells whether util-linux is there to give a case a terminal or take it away.
# env -i looks programs up on the case's own PATH, so these are kept as full paths.
script_bin=$(command -v script) && have_script=1 || have_script=0
# BusyBox's setsid cannot wait for its program, so it is only used when it can.
have_setsid=0
if setsid_bin=$(command -v setsid) && "$setsid_bin" -w true 2>/dev/null; then
    have_setsid=1
fi

failures=0
case_name=

# The lines as they should appear in the start-up files, with $HOME left unexpanded.
# shellcheck disable=SC2016
bash_line='export PATH="$HOME/.cargo/bin:$PATH"'
# shellcheck disable=SC2016
fish_line='fish_add_path $HOME/.cargo/bin'
# shellcheck disable=SC2016
env_line='. "$HOME/.cargo/env"'

# Sets up a fresh home. stub_dir holds the stand-ins; a case removes the ones it wants missing.
fresh() {
    case_name=$1
    home="$work/home-$case_name"
    stubs="$work/stubs-$case_name"
    mkdir -p "$home" "$stubs"
    log="$home/.stub.log"

    cat >"$stubs/cargo" <<'EOF'
#!/bin/sh
echo "cargo $*" >>"$HOME/.stub.log"
case $1 in
    --version) echo "cargo ${FAKE_CARGO_VERSION:-1.95.0} (stub)" ;;
    install)
        crate=$3
        case $crate in
            quvyta-framework-showcase) bin=qframe ;;
            quvyta-code) bin=qcode ;;
            quvyta-focus) bin=qfocus ;;
            quvyta-packages) bin=qpac ;;
            quvyta-tools) bin=qtools ;;
            *) bin=$crate ;;
        esac
        mkdir -p "$CARGO_HOME/bin"
        : >"$CARGO_HOME/bin/$bin"
        ;;
esac
EOF
    # Stands in for the download of rustup: the "installer" it returns puts the cargo stand-in
    # where rustup would.
    cat >"$stubs/curl" <<EOF
#!/bin/sh
echo "curl \$*" >>"\$HOME/.stub.log"
cat <<'INNER'
echo "rustup-init \$*" >>"\$HOME/.stub.log"
mkdir -p "\$CARGO_HOME/bin"
$(command -v cp) "$stubs/cargo.real" "\$CARGO_HOME/bin/cargo"
INNER
EOF
    printf '#!/bin/sh\nexit 0\n' >"$stubs/cc"
    chmod +x "$stubs/cargo" "$stubs/curl" "$stubs/cc"
    cp "$stubs/cargo" "$stubs/cargo.real"

    path="$stubs:$tools"
    user_shell=/bin/bash
    extra_env=
}

# Runs install.sh with no terminal at all. extra_env holds VAR=value words and is split on purpose.
# shellcheck disable=SC2086
run() {
    if [ "$have_setsid" = 1 ]; then
        env -i HOME="$home" CARGO_HOME="$home/.cargo" PATH="$path" SHELL="$user_shell" $extra_env \
            "$setsid_bin" -w "$shell_path" "$script" "$@" >"$home/.out" 2>&1 </dev/null
    else
        env -i HOME="$home" CARGO_HOME="$home/.cargo" PATH="$path" SHELL="$user_shell" $extra_env \
            "$shell_path" "$script" "$@" >"$home/.out" 2>&1 </dev/null
    fi
    status=$?
}

# Runs install.sh on a terminal of its own and answers its questions with the given lines.
# shellcheck disable=SC2086
run_tty() {
    answers=$1
    shift
    printf '%s' "$answers" >"$home/.answers"
    # script starts the command with $SHELL, so the case's own SHELL is set inside it.
    command="SHELL=$user_shell exec $shell_path $script $*"
    env -i HOME="$home" CARGO_HOME="$home/.cargo" PATH="$path" SHELL="$shell_path" $extra_env \
        TERM=dumb "$script_bin" -qec "$command" /dev/null <"$home/.answers" >"$home/.out" 2>&1
    status=$?
}

fail() {
    echo "FAIL $case_name: $*"
    echo "--- output"
    cat "$home/.out"
    echo "--- stub log"
    cat "$log" 2>/dev/null
    echo "---"
    failures=$((failures + 1))
}

expect_status() {
    [ "$status" = "$1" ] || fail "exit status $status, expected $1"
}

expect_output() {
    grep -qF -- "$1" "$home/.out" || fail "output lacks: $1"
}

expect_logged() {
    grep -qxF -- "$1" "$log" 2>/dev/null || fail "not run: $1"
}

expect_not_logged() {
    if grep -qF -- "$1" "$log" 2>/dev/null; then
        fail "should not have run: $1"
    fi
}

# Nothing but the files the harness itself puts in the home folder.
expect_home_untouched() {
    extra=$(cd "$home" && find . -mindepth 1 ! -name .out ! -name .stub.log ! -name .answers)
    [ -z "$extra" ] || fail "home folder changed: $extra"
}

expect_count() {
    count=$(grep -cxF -- "$2" "$1" 2>/dev/null)
    [ "${count:-0}" = "$3" ] || fail "$1 has '$2' ${count:-0} times, expected $3"
}

# --- Arguments

fresh help
run --help
expect_status 0
expect_output "quvyta-packages, command qpac"
expect_output "-y, --yes"
expect_home_untouched

fresh unknown-name
run code nonsense
expect_status 2
expect_output "Unknown name: nonsense"
expect_not_logged "cargo"

fresh unknown-option
run --force code
expect_status 2
expect_output "Unknown option: --force"

fresh names-once-in-order
run --yes code focus code
expect_status 0
expect_logged "cargo install --locked quvyta-code"
expect_logged "cargo install --locked quvyta-focus"
expect_count "$log" "cargo install --locked quvyta-code" 1
[ "$(grep '^cargo install' "$log" | head -n 1)" = "cargo install --locked quvyta-code" ] || fail "order not kept"

fresh all
run -y all
expect_status 0
for crate in quvyta-framework-showcase quvyta-code quvyta-focus quvyta-packages quvyta-tools quvyta; do
    expect_logged "cargo install --locked $crate"
done
for command in qframe qcode qfocus qpac qtools quvyta; do
    expect_output "  $command "
done

# --- Without a terminal

fresh no-tty-names
run code
expect_status 1
expect_output "cargo install --locked quvyta-code"
expect_not_logged "cargo install"
expect_home_untouched

fresh no-tty-no-names
run
expect_status 1
expect_output "There is no terminal to choose on"
expect_output "packages   qpac"
expect_not_logged "cargo"
expect_home_untouched

fresh no-tty-no-cargo
rm "$stubs/cargo"
run code
expect_status 1
expect_output "https://sh.rustup.rs"
expect_not_logged "curl"
expect_home_untouched

fresh no-tty-yes-installs-and-writes
run --yes code
expect_status 0
expect_count "$home/.bashrc" "$bash_line" 1
expect_output "Open a new terminal"

# --- Rust itself

fresh rustup-with-yes
rm "$stubs/cargo"
run --yes tools
expect_status 0
expect_logged "curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs"
expect_logged "rustup-init -y --no-modify-path --profile minimal"
expect_output "Rust is installed."
expect_logged "cargo install --locked quvyta-tools"

fresh rust-too-old
extra_env="FAKE_CARGO_VERSION=1.80.2"
run --yes code
expect_status 1
expect_output "Rust 1.80 is installed"
expect_not_logged "cargo install"

fresh no-linker
rm "$stubs/cc"
run --yes code
expect_status 1
expect_output "C linker"
expect_not_logged "cargo install"

# --- PATH and the shell's start-up file

fresh already-on-path
path="$home/.cargo/bin:$path"
run --yes code
expect_status 0
expect_output "Run a command above by its name"
[ ! -e "$home/.bashrc" ] || fail ".bashrc written although the folder is on PATH"

fresh bash-twice
run --yes code
run --yes focus
expect_count "$home/.bashrc" "$bash_line" 1
expect_output "already adds it"

fresh zsh
user_shell=/usr/bin/zsh
run --yes code
expect_count "$home/.zshrc" "$bash_line" 1
[ ! -e "$home/.bashrc" ] || fail "bash file written for zsh"

fresh zsh-zdotdir
user_shell=/bin/zsh
extra_env="ZDOTDIR=$home/zdot"
mkdir -p "$home/zdot"
run --yes code
expect_count "$home/zdot/.zshrc" "$bash_line" 1

fresh fish
user_shell=/usr/bin/fish
run --yes code
run --yes focus
expect_count "$home/.config/fish/config.fish" "$fish_line" 1

fresh fish-rustup-conf
user_shell=/usr/bin/fish
mkdir -p "$home/.config/fish/conf.d"
: >"$home/.config/fish/conf.d/rustup.fish"
run --yes code
[ ! -e "$home/.config/fish/config.fish" ] || fail "config.fish written although rustup's file adds the folder"

fresh rustup-env-line
printf '%s\n' "$env_line" >"$home/.bashrc"
run --yes code
expect_count "$home/.bashrc" "$bash_line" 0

fresh unknown-shell
user_shell=/bin/tcsh
run --yes code
expect_status 0
expect_output "$bash_line"
[ ! -e "$home/.bashrc" ] && [ ! -e "$home/.profile" ] || fail "a start-up file was written for an unknown shell"

fresh cargo-home-elsewhere
elsewhere="$work/cargo-elsewhere"
env_home=$home
run_elsewhere() {
    env -i HOME="$env_home" CARGO_HOME="$elsewhere" PATH="$path" SHELL=/bin/bash \
        "$shell_path" "$script" "$@" >"$home/.out" 2>&1 </dev/null
    status=$?
}
run_elsewhere --yes code
expect_count "$home/.bashrc" "export PATH=\"$elsewhere/bin:\$PATH\"" 1

# --- On a terminal

if [ "$have_script" = 1 ]; then
    fresh tty-pick-and-agree
    run_tty "2 4
y
y
"
    expect_status 0
    expect_logged "cargo install --locked quvyta-code"
    expect_logged "cargo install --locked quvyta-packages"
    expect_not_logged "quvyta-focus"
    expect_count "$home/.bashrc" "$bash_line" 1

    fresh tty-pick-names
    run_tty "tools quvyta
y
n
"
    expect_status 0
    expect_logged "cargo install --locked quvyta-tools"
    expect_logged "cargo install --locked quvyta"
    [ ! -e "$home/.bashrc" ] || fail ".bashrc written after no"
    expect_output "Not added"
    expect_output "start them by their full path"

    fresh tty-bad-then-good
    run_tty "9
code
y
y
"
    expect_status 0
    expect_output "Unknown choice: 9"
    expect_logged "cargo install --locked quvyta-code"

    fresh tty-empty-choice
    run_tty "
"
    expect_status 1
    expect_output "Nothing chosen"
    expect_not_logged "cargo"

    fresh tty-decline-install
    run_tty "n
" code
    expect_status 1
    expect_not_logged "cargo install"
    expect_home_untouched

    fresh tty-decline-rustup
    rm "$stubs/cargo"
    run_tty "n
" code
    expect_status 1
    expect_output "Rust was not installed"
    expect_not_logged "curl"
    expect_home_untouched
else
    echo "skipped the terminal cases: util-linux script is not installed"
fi

if [ "$failures" -gt 0 ]; then
    echo "$failures failed ($shell)"
    exit 1
fi
echo "install.sh: all cases passed ($shell)"
