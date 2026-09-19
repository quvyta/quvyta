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
for tool in grep sed basename dirname mkdir cat uname; do
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

# Sets up a fresh home, in the given folder or one named after the case. stub_dir holds the stand-ins; a case removes the ones it wants missing.
fresh() {
    case_name=$1
    home=${2:-"$work/home-$case_name"}
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
        mkdir -p "${CARGO_HOME:-$HOME/.cargo}/bin"
        : >"${CARGO_HOME:-$HOME/.cargo}/bin/$bin"
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
#
# These cases come from tests/path-cases.toml, the table quvyta's own PATH rules are tested
# against too, so the two cannot drift apart.

cases="$here/../path-cases.toml"

# Prints one value of case number $case_number exactly as the table holds it, with {root} and
# {home} filled in, and without a newline of its own; fails when the case has no such key.
# It reads only the part of TOML the table is written in: see the comment at its top.
value() {
    awk -v want="$case_number" -v key="$1" -v root="$case_root" -v home="$case_root/home" '
        function fill(text,    out, at) {
            out = ""
            while ((at = index(text, "{")) > 0) {
                if (substr(text, at, 6) == "{home}") {
                    out = out substr(text, 1, at - 1) home
                    text = substr(text, at + 6)
                } else if (substr(text, at, 6) == "{root}") {
                    out = out substr(text, 1, at - 1) root
                    text = substr(text, at + 6)
                } else {
                    out = out substr(text, 1, at)
                    text = substr(text, at + 1)
                }
            }
            return out text
        }
        multiline {
            if (length($0) >= 3 && substr($0, length($0) - 2) == "'"'''"'") {
                text = text substr($0, 1, length($0) - 3)
                multiline = 0
                if (wanted) { printf "%s", fill(text); found = 1; exit }
            } else {
                text = text $0 "\n"
            }
            next
        }
        $0 == "[[case]]" { number++; next }
        /^[ \t]*(#|$)/ { next }
        {
            at = index($0, " = ")
            wanted = number == want && substr($0, 1, at - 1) == key
            text = substr($0, at + 3)
            if (text == "'"'''"'") { multiline = 1; text = ""; next }
            if (wanted) {
                if (substr(text, 1, 1) == "'"'"'") text = substr(text, 2, length(text) - 2)
                printf "%s", fill(text); found = 1; exit
            }
        }
        END { exit !found }
    ' "$cases"
}

# The home folder as a list of names, with a checksum for every file, leaving out cargo's own
# folders and the files of the harness. Equal lists mean nothing was written.
snapshot() {
    (cd "$home" && find . \( -path ./.cargo -o -path ./cargo \) -prune -o \
        ! -name .out ! -name .stub.log ! -name .answers -print | sort | while read -r entry; do
        if [ -L "$entry" ]; then
            echo "$entry link"
        elif [ -f "$entry" ]; then
            echo "$entry $(cksum <"$entry")"
        else
            echo "$entry"
        fi
    done)
}

# Whether a file holds exactly the text, trailing newlines included.
same_content() {
    [ "$(cat "$1"; echo .)" = "$(value "$2"; echo .)" ]
}

# Runs install.sh with only the variables the case sets. case_env holds VAR=value words and is
# split on purpose.
# shellcheck disable=SC2086
run_path_case() {
    env -i HOME="$home" PATH="$path" $case_env "$shell_path" "$script" --yes code >"$home/.out" 2>&1 </dev/null
    status=$?
}

# bash fills in SHELL from the password file when it is not set, so under bash install.sh never
# sees it unset.
runner_sets_shell=0
# shellcheck disable=SC2016 # the inner shell expands it
[ "$(env -i "$shell_path" -c 'echo "${SHELL-unset}"')" = unset ] || runner_sets_shell=1

case_count=$(awk '$0 == "[[case]]" { n++ } END { print n + 0 }' "$cases")
[ "$case_count" -gt 0 ] || { echo "no cases in $cases"; failures=$((failures + 1)); }
case_number=1
while [ "$case_number" -le "$case_count" ]; do
    case_root="$work/path-$case_number"
    fresh "path-$(value name)" "$case_root/home"

    if ! value shell >/dev/null && [ "$runner_sets_shell" = 1 ]; then
        echo "skipped path case $(value name): $shell sets SHELL itself when it is not set"
        case_number=$((case_number + 1))
        continue
    fi
    case_env=
    for variable in SHELL=shell CARGO_HOME=cargo_home XDG_CONFIG_HOME=xdg_config_home ZDOTDIR=zdotdir; do
        if value "${variable#*=}" >/dev/null; then
            case_env="$case_env ${variable%%=*}=$(value "${variable#*=}")"
        fi
    done
    cargo_home=$(value cargo_home) && [ -n "$cargo_home" ] || cargo_home="$home/.cargo"
    [ "$(value on_path)" = true ] && path="$cargo_home/bin:$path"

    file=$(value file) || file=
    if link_to=$(value link_to); then
        mkdir -p "$(dirname "$file")" "$(dirname "$link_to")"
        ln -s "$link_to" "$file"
        value before >/dev/null && value before >"$link_to"
    elif value before >/dev/null; then
        mkdir -p "$(dirname "$file")"
        value before >"$file"
    fi
    if extra_file=$(value extra_file); then
        mkdir -p "$(dirname "$extra_file")"
        : >"$extra_file"
    fi
    before=$(snapshot)

    run_path_case
    expect_status 0
    outcome=$(value outcome)
    case $outcome in
        on-path)
            expect_output "Run a command above by its name"
            [ "$(snapshot)" = "$before" ] || fail "the home folder changed"
            ;;
        already)
            expect_output "$(value configured_by) already adds it"
            expect_output "Open a new terminal"
            [ "$(snapshot)" = "$before" ] || fail "the home folder changed"
            ;;
        unknown)
            expect_output "is not one this script knows"
            expect_output "  $(value line)"
            [ "$(snapshot)" = "$before" ] || fail "the home folder changed"
            ;;
        add)
            expect_output "This line would be added to $file"
            expect_output "  $(value line)"
            expect_output "Added to $file"
            same_content "$file" after || fail "$file does not hold what the table expects"
            if [ -n "$link_to" ]; then
                [ -L "$file" ] || fail "$file is no longer a link"
            fi
            # A second run finds the line it added and leaves the file alone.
            run_path_case
            expect_output "$file already adds it"
            same_content "$file" after || fail "$file changed on the second run"
            ;;
        *) fail "unknown outcome in the table: $outcome" ;;
    esac
    case_number=$((case_number + 1))
done

# --- macOS
#
# uname and xcode-select are stand-ins here: uname says Darwin, and xcode-select answers like it
# does once Apple's Command Line Tools are installed. A C compiler stand-in stays in place, as
# /usr/bin/cc is always there on macOS even without the tools.

# Sets up a fresh home that looks like macOS with the Command Line Tools installed.
fresh_mac() {
    fresh "$1"
    # shellcheck disable=SC2016 # the stand-ins expand their own arguments
    printf '#!/bin/sh\n[ "$1" = -s ] && echo Darwin\n' >"$stubs/uname"
    # shellcheck disable=SC2016
    printf '#!/bin/sh\n[ "$1" = -p ] && echo /Library/Developer/CommandLineTools\n' >"$stubs/xcode-select"
    chmod +x "$stubs/uname" "$stubs/xcode-select"
    user_shell=/bin/zsh
}

# shellcheck disable=SC2016 # rustup's line, left unexpanded as it writes it
rustup_line='. "$HOME/.cargo/env"'

fresh_mac mac-zsh
run --yes code
expect_status 0
expect_logged "cargo install --locked quvyta-code"
expect_output "This line would be added to $home/.zprofile"
expect_count "$home/.zprofile" "$bash_line" 1
[ ! -e "$home/.zshrc" ] || fail ".zshrc written on macOS"
expect_output "Open a new terminal"
run --yes code
expect_output "$home/.zprofile already adds it"
expect_count "$home/.zprofile" "$bash_line" 1

fresh_mac mac-zsh-zdotdir
mkdir -p "$home/zdot"
extra_env="ZDOTDIR=$home/zdot"
run --yes code
expect_status 0
expect_count "$home/zdot/.zprofile" "$bash_line" 1
[ ! -e "$home/.zprofile" ] || fail ".zprofile written outside ZDOTDIR"

fresh_mac mac-bash
user_shell=/bin/bash
run --yes code
expect_status 0
expect_count "$home/.bash_profile" "$bash_line" 1
[ ! -e "$home/.bashrc" ] || fail ".bashrc written on macOS"

fresh_mac mac-empty-shell
user_shell=
run --yes code
expect_status 0
expect_count "$home/.zprofile" "$bash_line" 1

fresh_mac mac-unknown-shell
user_shell=/bin/tcsh
run --yes code
expect_status 0
expect_count "$home/.zprofile" "$bash_line" 1

fresh_mac mac-fish
user_shell=/opt/homebrew/bin/fish
run --yes code
expect_status 0
expect_count "$home/.config/fish/config.fish" "fish_add_path \$HOME/.cargo/bin" 1

fresh_mac mac-rustup-line-present
printf '%s\n' "$rustup_line" >"$home/.zprofile"
before=$(cksum <"$home/.zprofile")
run --yes code
expect_status 0
expect_output "$home/.zprofile already adds it"
[ "$(cksum <"$home/.zprofile")" = "$before" ] || fail ".zprofile changed"

fresh_mac mac-no-tty-shows-zprofile
run code
expect_status 1
expect_output "cargo install --locked quvyta-code"
expect_home_untouched

fresh_mac mac-no-command-line-tools
rm "$stubs/xcode-select"
run --yes code
expect_status 1
expect_output "Command Line Tools"
expect_output "  xcode-select --install"
expect_not_logged "cargo install"
expect_home_untouched

fresh_mac mac-command-line-tools-removed
printf '#!/bin/sh\necho "xcode-select: error: unable to get active developer directory" >&2\nexit 2\n' >"$stubs/xcode-select"
run --yes code
expect_status 1
expect_output "  xcode-select --install"
expect_not_logged "cargo install"
expect_not_logged "xcode-select --install"
expect_home_untouched

fresh_mac mac-list-marks-arch-only
run
expect_status 1
expect_output "packages   qpac"
expect_output "Arch Linux only, not for macOS"
[ "$(grep -c 'Arch Linux only, not for macOS' "$home/.out")" = 2 ] || fail "not exactly two members marked"
expect_not_logged "cargo"

fresh_mac mac-arch-only-named
run --yes packages code
expect_status 0
expect_output "Skipping packages (qpac): it runs on Arch Linux only."
expect_logged "cargo install --locked quvyta-code"
expect_not_logged "quvyta-packages"

fresh_mac mac-arch-only-alone
run --yes tools
expect_status 1
expect_output "Skipping tools (qtools)"
expect_output "Nothing left to install on macOS."
expect_not_logged "cargo"
expect_home_untouched

fresh_mac mac-all
run --yes all
expect_status 0
for crate in quvyta-framework-showcase quvyta-code quvyta-focus quvyta; do
    expect_logged "cargo install --locked $crate"
done
expect_not_logged "quvyta-packages"
expect_not_logged "quvyta-tools"

fresh linux-list-has-no-mark
run
expect_status 1
if grep -qF "not for macOS" "$home/.out"; then
    fail "Linux list marks members as not for macOS"
fi

fresh linux-empty-shell-unknown
user_shell=
run --yes code
expect_status 0
expect_output "is not one this script knows"
extra=$(cd "$home" && find . -mindepth 1 -maxdepth 1 ! -name .out ! -name .stub.log ! -name .cargo)
[ -z "$extra" ] || fail "home folder changed: $extra"

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

    fresh_mac tty-mac-pick
    run_tty "1 4
y
y
"
    expect_status 0
    expect_output "Skipping packages (qpac)"
    expect_logged "cargo install --locked quvyta-framework-showcase"
    expect_not_logged "quvyta-packages"
    expect_count "$home/.zprofile" "$bash_line" 1

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
