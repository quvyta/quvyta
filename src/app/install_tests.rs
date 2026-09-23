//! Installing from the screen: the dialog and its checks, a whole install through the stand-in
//! cargo, the queue, stopping, quitting, failures and the build folder.
//!
//! The harness runs a background task until it ends, so an install through the stand-in cargo
//! finishes within one frame. Screens of an install still running are built instead by queueing
//! members on the application before the harness starts: the task is made but never run, and
//! the test sends the lines and endings the task would.

use std::path::Path;

use qframe::icons::GlyphMode;
use qframe::runtime::{HandoffOutcome, Task, TaskEvent, TaskOutcome, Termination};
use qframe::storage::AppLock;
use tempfile::TempDir;

use super::installs::InstallMsg;
use super::tests::{
    LANGUAGES, LONGEST_LANGUAGES, TOAST_IN, env, harness, harness_with_settings, index, line_with, machine,
};
use super::*;
use crate::cargo::Failure;
use crate::checks::Problem;
use crate::install::Outcome;
use crate::install::tests::{free_soon, packages, scenario};
use crate::inventory::tests::{wait_until_runnable, write_program};

/// What cargo writes for a whole install, recorded.
const INSTALL: &str = include_str!("../../tests/cargo-output/install-ok.txt");

fn install(msg: InstallMsg) -> Msg {
    Msg::Install(msg)
}

/// The application on the machine of [`machine`] in `root`, with what is installed read, before
/// any harness runs it.
fn app_in(root: &Path) -> Quvyta {
    let mut app = Quvyta::new(machine(root));
    app.inventory = Some(Inventory::read(&app.machine));
    app
}

/// Queues `key` as the dialog would, without running anything: the first one queued becomes
/// the running install whose task never starts.
fn queue(app: &mut Quvyta, key: &str) {
    let index = index(key);
    let _ = app.update(install(InstallMsg::Ask(index)));
    let _ = app.update(install(InstallMsg::Checked { index, problems: Vec::new(), other_window: false }));
    let _ = app.update(install(InstallMsg::Confirm));
}

/// A harness over `app`, as the tests of the list start theirs.
pub(super) fn run(app: Quvyta, width: u16, height: u16) -> Harness<Quvyta> {
    let mut h = Harness::with_env(app, env(), width, height);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    h.render();
    h
}

/// qtools installing and qpac waiting, on a machine in a temporary folder.
fn installing() -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    let mut app = app_in(root.path());
    queue(&mut app, "tools");
    queue(&mut app, "packages");
    app.selected = index("tools");
    let h = run(app, 100, 30);
    (root, h)
}

/// The end the task of a stopped install reports.
fn stopped(key: &str) -> Msg {
    let id = Task::<Msg>::new("stopped", |_| Err(String::new())).id();
    install(InstallMsg::Event(index(key), TaskEvent::Finished { id, outcome: TaskOutcome::Cancelled }))
}

/// Long enough for a dialog to have popped in, with its close mark.
pub(super) const DIALOG_IN: std::time::Duration = std::time::Duration::from_millis(300);

/// Clicks `text` on the row of the screen that also holds `anchor`, such as a dialog's button
/// beside its Cancel.
pub(super) fn click_beside(h: &mut Harness<Quvyta>, anchor: &str, text: &str) {
    let screen = h.screen();
    let (y, x) = screen
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(anchor))
        .find_map(|(y, line)| {
            let at = line.rfind(text)?;
            Some((y, line[..at].chars().count()))
        })
        .unwrap_or_else(|| panic!("`{text}` beside `{anchor}` is not on screen:\n{screen}"));
    h.click(i32::try_from(x).expect("x"), i32::try_from(y).expect("y"));
}

/// Presses the Install button of the open dialog.
fn confirm(h: &mut Harness<Quvyta>) {
    click_beside(h, "Cancel", "Install");
}

pub(super) fn has(h: &Harness<Quvyta>, text: &str) -> bool {
    h.screen().contains(text)
}

/// The arguments cargo was called with, in order.
pub(super) fn calls(root: &Path) -> Vec<String> {
    std::fs::read_to_string(root.join("bin/calls.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.rsplit('\t').next().map(ToOwned::to_owned))
        .collect()
}

/// A few frames: the task's end, then the list read again and the build folder let go.
pub(super) fn settle(h: &mut Harness<Quvyta>) {
    for _ in 0..4 {
        h.render();
    }
}

#[test]
fn install_asks_first_showing_source_version_target_and_command() {
    let (root, mut h) = harness(100, 30);
    h.send(Msg::Select(index("tools")));
    assert!(h.screen().contains("enter  install"), "{}", h.screen());
    h.press("enter");
    let screen = h.screen();
    let cargo = root.path().join("bin/cargo").display().to_string();
    for text in [
        "Install qtools?",
        "Source   crates.io, quvyta-tools",
        "Version  the latest version on crates.io",
        "Target   ~/.cargo/bin/qtools",
        "Command",
        &format!("{cargo} install --locked quvyta-tools"),
        "It is built on this machine",
        "No sudo needed; it writes only under ~/.cargo.",
        "Cancel",
    ] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("--version"), "no version is known yet, so none is pinned:\n{screen}");
    assert!(calls(root.path()).iter().all(|call| !call.starts_with("install --locked")), "nothing ran");
    h.press("esc");
    assert!(!has(&h, "Install qtools?"), "{}", h.screen());
    assert!(calls(root.path()).iter().all(|call| !call.starts_with("install --locked")), "and nothing runs");
}

#[test]
fn cancel_and_the_close_mark_close_the_dialog_like_esc() {
    for close in ["Cancel", "×"] {
        let (_root, mut h) = harness(100, 30);
        h.send(Msg::Select(index("tools"))).click_text("Install").advance(DIALOG_IN);
        assert!(has(&h, "Install qtools?"));
        h.click_text(close);
        assert!(!has(&h, "Install qtools?"), "{close}:\n{}", h.screen());
        assert!(h.app().installs.running.is_none());
    }
}

#[test]
fn the_command_in_the_dialog_is_copied_whole() {
    let (root, mut h) = harness(100, 30);
    h.send(Msg::Select(index("tools"))).press("enter");
    h.click_text("install --locked quvyta-tools");
    assert_eq!(h.copied(), [format!("{} install --locked quvyta-tools", root.path().join("bin/cargo").display())]);
}

/// The machine of the list's tests under a folder with a long name, as a home folder often
/// is, so the commands in the dialogs are as long as on a real machine.
fn long_named() -> (TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().expect("temp");
    let deep = root.path().join("home-of-somebody-with-a-long-name");
    std::fs::create_dir_all(&deep).expect("folder");
    (root, deep)
}

#[test]
fn a_long_command_is_shown_whole_on_a_wide_screen() {
    let (_root, deep) = long_named();
    let mut h = super::tests::start(&deep, 120, 36);
    h.send(install(InstallMsg::Ask(index("packages")))).advance(DIALOG_IN);
    let command = format!("{} install --locked quvyta-packages", deep.join("bin/cargo").display());
    assert!(has(&h, &command), "`{command}` is cut:\n{}", h.screen());
    h.send(install(InstallMsg::Close)).send(install(InstallMsg::AskRemove(index("code")))).advance(DIALOG_IN);
    let command = format!("{} uninstall quvyta-code", deep.join("bin/cargo").display());
    assert!(has(&h, &command), "`{command}` is cut:\n{}", h.screen());
}

#[test]
fn a_long_command_wraps_on_a_narrow_screen_and_is_still_copied_whole() {
    for width in [80, 72] {
        let (_root, deep) = long_named();
        let mut h = super::tests::start(&deep, width, 36);
        h.send(install(InstallMsg::Ask(index("packages")))).advance(DIALOG_IN);
        let cargo = deep.join("bin/cargo").display().to_string();
        let screen = h.screen();
        for word in [cargo.as_str(), "install", "--locked", "quvyta-packages"] {
            let whole = screen.split_whitespace().any(|shown| shown == word);
            assert!(whole, "`{word}` is not shown whole at {width} columns:\n{screen}");
        }
        h.click_text("copy");
        assert_eq!(h.copied(), [format!("{cargo} install --locked quvyta-packages")], "at {width} columns");
    }
}

#[test]
fn quvyta_never_offers_to_install_itself() {
    let (_root, mut h) = harness(100, 30);
    h.send(Msg::Select(index("quvyta"))).send(install(InstallMsg::Ask(index("quvyta"))));
    assert!(h.app().installs.dialog.is_none());
    assert!(!has(&h, "Install"), "{}", h.screen());
}

/// The dialog for qtools on a machine with a C linker and no cargo at all.
fn without_cargo() -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    write_program(&root.path().join("bin/cc"), "#!/bin/sh\nexit 0\n");
    super::tests::set_up(root.path());
    let mut h = run(Quvyta::new(Machine::in_root(root.path())), 100, 34);
    h.send(Msg::Select(index("tools"))).press("enter");
    (root, h)
}

#[test]
fn without_cargo_the_dialog_shows_rustup_and_offers_to_run_it() {
    let (_root, mut h) = without_cargo();
    let screen = h.screen();
    for text in [
        "cargo, Rust's package tool, is not installed",
        crate::checks::RUSTUP_INSTALL,
        "https://rustup.rs",
        "Install here",
        "rustup asks its own questions and needs no sudo",
        "Check again",
    ] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("install --locked"), "there is no cargo to show a command for:\n{screen}");
    assert!(!screen.contains("install.sh"), "the Quvyta install script would build quvyta again:\n{screen}");
    click_beside(&mut h, "curl --proto", "copy");
    assert_eq!(h.copied(), [crate::checks::RUSTUP_INSTALL]);
    h.send(install(InstallMsg::Confirm));
    assert!(h.app().installs.running.is_none(), "a dialog with problems installs nothing");
    assert!(h.handoffs().is_empty(), "nothing runs before the button is pressed");
}

#[test]
fn install_here_runs_rustup_in_the_terminal_and_checks_again() {
    let (_root, mut h) = without_cargo();
    // rustup does its work: cargo is in its folder afterwards.
    write_program(&h.app().machine.cargo_bin().join("cargo"), "#!/bin/sh\nexit 0\n");
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).click_text("Install here");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, "sh");
    assert_eq!(request.args, ["-c", crate::checks::RUSTUP_INSTALL], "the very line shown");
    assert!(request.pause, "what it printed stays to be read");
    assert_eq!(request.notice.as_deref(), Some(format!("Running {}", crate::checks::RUSTUP_INSTALL).as_str()));
    settle(&mut h);
    assert_eq!(h.app().installs.dialog.as_ref().and_then(|dialog| dialog.problems.clone()), Some(Vec::new()));
    assert!(!has(&h, "cargo, Rust's package tool"), "checked again:\n{}", h.screen());
}

/// The application with a `rustc` that reports `version`, and `rustup` when `rustup`.
fn with_rust(version: &str, rustup: bool) -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    let app = Quvyta::new(machine(root.path()));
    let rustc = root.path().join("bin/rustc");
    write_program(&rustc, &format!("#!/bin/sh\necho 'rustc {version} (3f5fd8dd4 2024-08-06)'\n"));
    wait_until_runnable(&rustc);
    if rustup {
        write_program(&app.machine.cargo_bin().join("rustup"), "#!/bin/sh\nexit 0\n");
    }
    let mut h = run(app, 100, 34);
    h.send(Msg::Select(index("tools"))).press("enter");
    (root, h)
}

#[test]
fn an_old_rust_with_rustup_is_updated_in_the_terminal_and_checked_again() {
    let (root, mut h) = with_rust("1.80.1", true);
    let screen = h.screen();
    assert!(screen.contains("Rust 1.80.1 is installed; the Quvyta apps need 1.95 or later."), "{screen}");
    assert!(screen.contains("Update Rust") && !screen.contains("  Install  "), "{screen}");
    // rustup does its work; afterwards rustc answers with the new version.
    write_program(&root.path().join("bin/rustc"), "#!/bin/sh\necho 'rustc 1.95.0 (29483883e 2026-08-07)'\n");
    wait_until_runnable(&root.path().join("bin/rustc"));
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).click_text("Update Rust");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, h.app().machine.cargo_bin().join("rustup").as_os_str());
    assert_eq!(request.args, ["update", "stable"]);
    settle(&mut h);
    assert!(!has(&h, "Rust 1.80.1"), "checked again:\n{}", h.screen());
    assert_eq!(h.app().installs.dialog.as_ref().and_then(|dialog| dialog.problems.clone()), Some(Vec::new()));
}

#[test]
fn an_old_rust_without_rustup_only_says_what_to_do() {
    let (_root, h) = with_rust("1.80.1", false);
    let screen = h.screen();
    assert!(screen.contains("Update Rust with the tool you installed it with"), "{screen}");
    assert!(!screen.contains("Update Rust  "), "no button to press:\n{screen}");
    assert!(h.handoffs().is_empty());
}

/// The dialog for qtools on a machine whose `os-release` says `os_release` and that has no C
/// linker.
fn without_linker(os_release: &str) -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    let app = Quvyta::new(machine(root.path()));
    std::fs::remove_file(root.path().join("bin/cc")).expect("no linker");
    std::fs::create_dir_all(root.path().join("etc")).expect("folder");
    std::fs::write(root.path().join("etc/os-release"), os_release).expect("os-release");
    let mut h = run(app, 100, 40);
    h.send(Msg::Select(index("tools"))).press("enter");
    (root, h)
}

#[test]
fn a_missing_linker_shows_the_command_of_the_distribution() {
    for (os_release, command, others) in [
        ("ID=arch\n", "sudo pacman -S --needed base-devel", false),
        ("ID=ubuntu\nID_LIKE=debian\n", "sudo apt install build-essential", false),
        ("ID=fedora\n", "sudo dnf install gcc", false),
        ("ID=alpine\n", "sudo pacman -S --needed base-devel", true),
    ] {
        let (_root, h) = without_linker(os_release);
        let screen = h.screen();
        assert!(screen.contains("Rust needs a C linker"), "{screen}");
        assert!(screen.contains(command), "{os_release}:\n{screen}");
        assert_eq!(screen.contains("sudo dnf install gcc") && screen.contains("sudo apt"), others, "{screen}");
        // Only the one right command is offered to run; of several, none is.
        assert_eq!(screen.contains("Install here"), !others, "{screen}");
        assert_eq!(screen.contains("sudo asks for your password there"), !others, "{screen}");
        assert_eq!(screen.contains("Run the command for your system yourself"), others, "{screen}");
        assert!(h.handoffs().is_empty(), "sudo is not run until the button is pressed");
    }
}

#[test]
fn install_here_hands_the_terminal_to_the_linker_command_and_checks_again() {
    for (os_release, command) in [
        ("ID=arch\n", "sudo pacman -S --needed base-devel"),
        ("ID=ubuntu\nID_LIKE=debian\n", "sudo apt install build-essential"),
        ("ID=fedora\n", "sudo dnf install gcc"),
    ] {
        let (root, mut h) = without_linker(os_release);
        // The package manager does its work: a linker is there afterwards.
        write_program(&root.path().join("bin/cc"), "#!/bin/sh\nexit 0\n");
        h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).click_text("Install here");
        let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
        assert_eq!(request.program, "sh");
        assert_eq!(request.args, ["-c", command], "the very line shown");
        assert!(request.pause);
        settle(&mut h);
        assert_eq!(h.app().installs.dialog.as_ref().and_then(|dialog| dialog.problems.clone()), Some(Vec::new()));
        assert!(!has(&h, "Rust needs a C linker"), "checked again:\n{}", h.screen());
        let screen = h.screen();
        assert!(line_with(&screen, "Cancel").contains("Install") && !screen.contains("Check again"), "{screen}");
    }
}

#[test]
fn install_here_is_offered_in_every_language_in_its_own_words() {
    let own_words = [
        ("en", "Install here"),
        ("tr", "Burada kur"),
        ("de", "Hier installieren"),
        ("es", "Instalar aquí"),
        ("fr", "Installer ici"),
        ("pt-BR", "Instalar aqui"),
        ("ru", "Установить здесь"),
        ("zh-Hans", "在这里安装"),
        ("ja", "ここで入れる"),
    ];
    assert_eq!(own_words.len(), LANGUAGES.len(), "every language quvyta speaks is asked");
    for (locale, button) in own_words {
        for width in [60, 100] {
            let (_root, mut h) = without_linker("ID=arch\n");
            h.set_locale(locale).resize(width, 40);
            let screen = h.screen();
            assert!(screen.contains(button), "`{locale}` at {width} should say `{button}`:\n{screen}");
            assert!(screen.contains("sudo"), "`{locale}` at {width} says sudo asks:\n{screen}");
            h.click_text(button);
            assert_eq!(h.handoffs().len(), 1, "`{locale}` at {width}: the button runs the line");
        }
    }
}

#[test]
fn a_fix_that_fails_says_so_and_leaves_the_problem_on_screen() {
    for outcome in [HandoffOutcome::Finished { code: Some(1) }, HandoffOutcome::Failed("no terminal".to_owned())] {
        let (_root, mut h) = without_linker("ID=arch\n");
        h.set_handoff_outcome(outcome.clone()).click_text("Install here");
        settle(&mut h);
        let screen = h.advance(TOAST_IN).screen();
        assert!(screen.contains("The command did not finish successfully"), "{outcome:?}:\n{screen}");
        assert!(screen.contains("Rust needs a C linker"), "still in the way:\n{screen}");
        assert!(!screen.contains("  Install  "), "{screen}");
    }
}

#[test]
fn a_whole_install_ends_with_open_a_toast_a_log_and_no_build_folder() {
    let (root, mut h) = harness(100, 30);
    scenario(root.path(), INSTALL, 0);
    packages(root.path());
    h.send(Msg::Select(index("tools"))).press("enter");
    confirm(&mut h);
    settle(&mut h);
    let calls = calls(root.path());
    assert!(calls.contains(&"install --locked quvyta-tools".to_owned()), "{calls:?}");
    let screen = h.advance(TOAST_IN).screen();
    assert!(line_with(&screen, "qtools ").contains("0.1.2"), "{screen}");
    assert!(screen.contains("Open") && screen.contains("~/.cargo/bin/qtools"), "{screen}");
    assert!(screen.contains("qtools 0.1.2 installed"), "{screen}");
    assert!(!root.path().join("data/build").exists(), "the queue ended, so its build folder went");
    let log = std::fs::read_to_string(root.path().join("data/logs/qtools.log")).expect("log");
    assert!(log.contains("Installed package `quvyta-tools v0.1.2`"), "{log}");
    assert!(free_soon(&h.app().machine), "the build folder is let go");
    h.press("enter");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, h.app().machine.cargo_bin().join("qtools").as_os_str());
}

#[test]
fn an_install_into_a_folder_off_path_offers_to_put_it_there() {
    let root = tempfile::tempdir().expect("temp");
    let mut machine = super::tests::machine_off_path(root.path());
    machine.shell = Some("/bin/bash".to_owned());
    let mut h = run(Quvyta::new(machine), 100, 40);
    scenario(root.path(), INSTALL, 0);
    packages(root.path());
    // qcode, already installed by cargo, gets the general wording at start.
    assert!(!has(&h, "For qtools"), "{}", h.screen());
    h.send(Msg::Select(index("tools"))).press("enter");
    confirm(&mut h);
    settle(&mut h);
    let screen = h.screen();
    assert!(line_with(&screen, "qtools ").contains("0.1.2"), "{screen}");
    assert!(screen.contains("~/.cargo/bin is not on PATH"), "{screen}");
    assert!(screen.contains("For qtools to open when you type its name"), "the notice names the member:\n{screen}");
}

#[test]
fn an_install_with_cargo_s_folder_on_path_says_nothing_about_it() {
    let (root, mut h) = harness(100, 30);
    scenario(root.path(), INSTALL, 0);
    packages(root.path());
    h.send(Msg::Select(index("tools"))).press("enter");
    confirm(&mut h);
    settle(&mut h);
    assert!(line_with(&h.screen(), "qtools ").contains("0.1.2"), "{}", h.screen());
    assert!(!has(&h, "is not on PATH"), "{}", h.screen());
}

#[test]
fn the_progress_shows_the_phase_the_crate_and_stop() {
    let (_root, mut h) = installing();
    let screen = h.screen();
    for text in ["qtools installing", "Starting", "Details", "Stop"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(line_with(&screen, "qtools ").contains("installing"), "{screen}");
    assert!(line_with(&screen, "qpac ").contains("queued"), "{screen}");
    let tools = index("tools");
    for line in ["    Updating crates.io index", "   Compiling ratatui v0.29.0"] {
        h.send(install(InstallMsg::Line(tools, line.to_owned())));
    }
    let screen = h.screen();
    assert!(screen.contains("Compiling") && screen.contains("ratatui"), "{screen}");
    let compiling = h.find("Compiling").expect("phase");
    // Counts come from the frames of cargo's progress line; they sit beside the phase, which
    // keeps its width.
    h.send(install(InstallMsg::Frame(tools, "    Building [=====>  ] 142/231: ratatui, serde".to_owned())));
    let screen = h.screen();
    assert!(screen.contains("142 / 231"), "{screen}");
    assert!(line_with(&screen, "qtools ").contains("installing 61%"), "{screen}");
    let counts = h.find("142 / 231").expect("counts");
    h.send(install(InstallMsg::Line(tools, "  Installing /home/ayse/.cargo/bin/qtools".to_owned())));
    assert_eq!(h.find("142 / 231"), Some(counts), "the counts do not move when the phase changes");
    assert_eq!(h.find("Installing").map(|at| at.0), Some(compiling.0));
}

#[test]
fn the_counts_climb_with_cargo_s_frames_in_the_row_and_the_view() {
    let (_root, mut h) = installing();
    let tools = index("tools");
    h.send(install(InstallMsg::Line(tools, "   Compiling proc-macro2 v1.0.95".to_owned())));
    let mut seen = Vec::new();
    for (done, krate) in [(0_u32, "anstyle"), (12, "serde"), (45, "hexyl")] {
        h.send(install(InstallMsg::Frame(tools, format!("    Building [==>  ] {done}/46: {krate}, libc"))));
        let screen = h.screen();
        assert!(screen.contains(&format!("{done} / 46")), "the view counts {done} of 46:\n{screen}");
        assert!(screen.contains(krate), "the view names the crate being built:\n{screen}");
        seen.push(line_with(&screen, "qtools ").to_owned());
    }
    // The row climbs with them.
    for (row, share) in seen.iter().zip(["installing 0%", "installing 26%", "installing 97%"]) {
        assert!(row.contains(share), "the row says `{share}`: {row}");
    }
}

#[test]
fn a_frame_naming_another_crate_moves_nothing_but_the_crate() {
    let (_root, mut h) = installing();
    let (tools, packages) = (index("tools"), index("packages"));
    h.send(install(InstallMsg::Line(tools, "   Compiling serde v1.0.219".to_owned())));
    h.send(install(InstallMsg::Frame(tools, "    Building [==>  ] 12/46: serde, libc".to_owned())));
    let (phase, counts) = (h.find("Compiling").expect("phase"), h.find("12 / 46").expect("counts"));
    let rows: Vec<String> = h.screen().lines().take(6).map(ToOwned::to_owned).collect();

    h.send(install(InstallMsg::Frame(tools, "    Building [===>  ] 12/46: ratatui, libc".to_owned())));
    let screen = h.screen();
    assert!(screen.contains("ratatui"), "the crate being built is the newest one named:\n{screen}");
    assert!(!screen.contains("serde,"), "no frame text is shown as it came:\n{screen}");
    assert_eq!(h.find("Compiling").expect("phase"), phase, "the phase stays where it was:\n{screen}");
    assert_eq!(h.find("12 / 46").expect("counts"), counts, "the counts stay where they were:\n{screen}");
    assert_eq!(screen.lines().take(6).map(ToOwned::to_owned).collect::<Vec<_>>(), rows, "the list is unchanged");

    // A frame belongs to the install it came from; the one waiting in the queue is untouched.
    h.send(install(InstallMsg::Frame(packages, "    Building [=====>  ] 40/46: quvyta-packages".to_owned())));
    let screen = h.screen();
    assert!(screen.contains("12 / 46") && !screen.contains("40 / 46"), "{screen}");
    assert!(line_with(&screen, "qpac ").contains("queued"), "{screen}");
}

#[test]
fn frames_stay_out_of_the_details_and_the_copied_log() {
    let (_root, mut h) = installing();
    let tools = index("tools");
    h.send(install(InstallMsg::Line(tools, "   Compiling serde v1.0.219".to_owned())));
    h.send(install(InstallMsg::Frame(tools, "    Building [==>  ] 12/46: serde, libc".to_owned())));
    h.click_text("Details");
    let screen = h.screen();
    assert!(screen.contains("Compiling serde v1.0.219"), "cargo's own lines are there:\n{screen}");
    assert!(!screen.contains("Building ["), "a frame is no log line:\n{screen}");
    h.send(install(InstallMsg::CopyLog(tools)));
    assert_eq!(h.copied(), ["   Compiling serde v1.0.219"]);
}

#[test]
fn an_install_that_never_counts_keeps_an_unknown_bar_and_no_share() {
    let (_root, mut h) = installing();
    let tools = index("tools");
    for line in ["    Updating crates.io index", "   Compiling serde v1.0.219"] {
        h.send(install(InstallMsg::Line(tools, line.to_owned())));
    }
    let screen = h.screen();
    assert!(screen.contains("Compiling") && screen.contains("serde"), "{screen}");
    assert!(!screen.contains('%'), "nothing pretends to know the share:\n{screen}");
    assert!(!screen.contains(" / "), "no counts are made up:\n{screen}");
    assert!(line_with(&screen, "qtools ").contains("installing"), "{screen}");
    // The bar is drawn all the same, and as the bar of unknown work: a band of light sweeping
    // the whole track, not a track filled from the left to a share nobody knows.
    let (x, y) = h.find("Compiling").expect("phase");
    let (x, y) = (u16::try_from(x).expect("x"), u16::try_from(y + 1).expect("bar row"));
    h.advance(std::time::Duration::from_millis(600));
    let bar: Vec<_> = (0..20).map(|dx| h.bg(x + dx, y)).collect();
    let mut shades = bar.clone();
    shades.dedup();
    assert!(shades.len() > 1, "the band of light is on the track: {bar:?}\n{}", h.screen());
    h.advance(std::time::Duration::from_millis(300));
    let moved: Vec<_> = (0..20).map(|dx| h.bg(x + dx, y)).collect();
    assert_ne!(bar, moved, "the band sweeps, as work of unknown size does");
}

#[test]
fn details_show_cargo_s_own_output() {
    let (_root, mut h) = installing();
    h.send(install(InstallMsg::Line(index("tools"), "   Compiling serde v1.0.219".to_owned())));
    assert!(!has(&h, "Compiling serde v1.0.219"));
    h.click_text("Details");
    assert!(has(&h, "Compiling serde v1.0.219"), "{}", h.screen());
    h.click_text("Details");
    assert!(!has(&h, "Compiling serde v1.0.219"));
}

#[test]
fn the_queue_runs_in_order_and_a_failure_does_not_stop_the_next() {
    let (root, mut h) = installing();
    std::fs::write(root.path().join("bin/install.out"), INSTALL.replace("quvyta-tools", "quvyta-packages"))
        .expect("scenario");
    packages(root.path());
    let tools = index("tools");
    h.send(install(InstallMsg::Line(tools, "error: linker `cc` not found".to_owned())));
    h.send(install(InstallMsg::Finished {
        index: tools,
        outcome: Outcome::Failed(Failure::NoLinker),
        problems: Vec::new(),
    }));
    settle(&mut h);
    let screen = h.screen();
    assert!(line_with(&screen, "qtools ").contains("failed"), "{screen}");
    assert!(line_with(&screen, "qpac ").contains("0.1.2"), "the next one still ran:\n{screen}");
    assert_eq!(calls(root.path()).iter().filter(|call| call.starts_with("install --locked")).count(), 1);
    assert!(!root.path().join("data/build").exists());
}

#[test]
fn stopping_asks_and_leaves_the_queue_running() {
    let (root, mut h) = installing();
    packages(root.path());
    h.click_text("Stop");
    let screen = h.screen();
    assert!(screen.contains("Stop installing qtools?"), "{screen}");
    assert!(screen.contains("Stop the queued ones too"), "{screen}");
    h.click_text("Keep installing");
    assert!(!has(&h, "Stop installing qtools?") && h.app().installs.running.is_some());

    h.click_text("Stop");
    click_beside(&mut h, "Keep installing", "Stop");
    assert!(h.app().installs.stop.is_none());
    h.send(stopped("tools"));
    settle(&mut h);
    let screen = h.screen();
    assert!(screen.contains("Stopped, nothing was installed."), "{screen}");
    assert!(line_with(&screen, "qtools ").contains("not installed"), "{screen}");
    assert!(line_with(&screen, "qpac ").contains("0.1.2"), "the queued one went on:\n{screen}");
}

#[test]
fn stopping_the_queued_ones_too_empties_the_queue() {
    let (root, mut h) = installing();
    h.click_text("Stop");
    h.click_text("Stop the queued ones too");
    assert_eq!(h.app().installs.stop, Some(true));
    click_beside(&mut h, "Keep installing", "Stop");
    h.send(stopped("tools"));
    settle(&mut h);
    let screen = h.screen();
    assert!(line_with(&screen, "qpac ").contains("not installed"), "{screen}");
    assert!(calls(root.path()).iter().all(|call| !call.starts_with("install --locked")), "nothing else ran");
    assert!(!h.app().installs.busy());
}

#[test]
fn a_queued_member_can_be_taken_out() {
    let (_root, mut h) = installing();
    h.send(Msg::Select(index("packages")));
    let screen = h.screen();
    assert!(screen.contains("qpac queued") && screen.contains("It starts when qtools is done."), "{screen}");
    h.click_text("Take out of the queue");
    assert!(line_with(&h.screen(), "qpac ").contains("not installed"), "{}", h.screen());
    assert!(h.app().installs.running.is_some(), "the running one goes on");
}

#[test]
fn quitting_while_installing_asks_and_stops_it() {
    let (_root, mut h) = installing();
    h.press("ctrl+q");
    assert!(!h.quit_requested());
    let screen = h.screen();
    assert!(screen.contains("An install is running") && screen.contains("Quitting stops it"), "{screen}");
    h.press("esc");
    assert!(!h.quit_requested() && h.app().installs.running.is_some());

    h.press("ctrl+q");
    click_beside(&mut h, "Cancel", "Quit");
    assert!(!h.quit_requested(), "it waits for cargo to be stopped");
    assert!(h.app().installs.queue.is_empty());
    h.send(stopped("tools"));
    settle(&mut h);
    assert!(h.quit_requested());
}

#[test]
fn a_terminate_signal_asks_and_a_hangup_stops_without_asking() {
    let (_root, mut h) = installing();
    h.terminate(Termination::Terminate);
    assert!(!h.quit_requested() && has(&h, "An install is running"), "{}", h.screen());

    let (_root, mut h) = installing();
    h.terminate(Termination::Hangup);
    assert!(!h.quit_requested(), "cargo is stopped first");
    h.send(stopped("tools"));
    settle(&mut h);
    assert!(h.quit_requested());
}

#[test]
fn without_an_install_quitting_asks_nothing() {
    let (_root, mut h) = harness(100, 24);
    h.press("ctrl+q");
    assert!(h.quit_requested());
}

#[test]
fn with_after_close_shell_quvyta_stays_while_an_install_runs() {
    let root = tempfile::tempdir().expect("temp");
    std::fs::create_dir_all(root.path().join("config")).expect("folder");
    std::fs::write(root.path().join("config/launcher.conf"), "after_close = \"shell\"\n").expect("settings");
    let mut app = app_in(root.path());
    app.launcher = Launcher::load(app.machine.launcher_conf.as_deref());
    queue(&mut app, "tools");
    let mut h = run(app, 100, 30);
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).send(Msg::Open(index("code")));
    h.render();
    assert!(!h.quit_requested(), "the install would stop unseen");
    assert!(line_with(&h.screen(), "qtools ").contains("installing"), "{}", h.screen());

    let (_root, mut h) = harness_with_settings("after_close = \"shell\"\n");
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).press("enter");
    assert!(h.quit_requested(), "without an install it still leaves");
}

/// The application after `out` made cargo fail with code 101 for qtools, with the linker gone
/// after the dialog's checks passed.
fn failed(out: &str) -> (TempDir, Harness<Quvyta>) {
    let (root, mut h) = harness(100, 40);
    scenario(root.path(), out, 101);
    std::fs::create_dir_all(root.path().join("etc")).expect("folder");
    std::fs::write(root.path().join("etc/os-release"), "ID=arch\n").expect("os-release");
    h.send(Msg::Select(index("tools"))).press("enter");
    std::fs::remove_file(root.path().join("bin/cc")).expect("linker gone");
    confirm(&mut h);
    settle(&mut h);
    (root, h)
}

#[test]
fn every_failure_is_told_plainly_with_what_to_do() {
    for (out, sentence) in [
        ("error: linker `cc` not found\n", "There is no C linker on this machine"),
        (
            "error: cannot install package `quvyta-tools 0.1.2`, it requires rustc 1.95 or newer, while the currently active rustc version is 1.80.1\n",
            "It needs a newer Rust than the one installed.",
        ),
        (
            "error: download of config.json failed\n\nCaused by:\n  [6] Couldn't resolve host name\n",
            "crates.io could not be reached.",
        ),
        ("LLVM ERROR: IO failure on output stream: No space left on device\n", "The disk is full."),
        (
            "error: could not find `quvyta-tools` in registry `crates-io` with version `=9.9.9`\n",
            "crates.io has no such version of quvyta-tools.",
        ),
        ("error[E0425]: cannot find value `x` in this scope\n", "The build failed."),
    ] {
        let (_root, h) = failed(out);
        let screen = h.screen();
        for text in [
            "qtools could not be installed",
            sentence,
            "Nothing was installed; nothing on the machine changed.",
            "Last lines",
            "The whole log is in ~/data/logs/qtools.log",
            "Copy log",
            "Close",
            "Retry",
        ] {
            let text = text.replace("~/data", &h.app().machine.show(&h.app().machine.data_dir.clone().expect("data")));
            assert!(screen.contains(&text), "`{text}` is missing:\n{screen}");
        }
        let first: String = out.lines().next().expect("a line").chars().take(40).collect();
        assert!(screen.contains(&first), "the last lines:\n{screen}");
        assert!(line_with(&screen, "qtools ").contains("failed"), "{screen}");
        let linker = sentence.starts_with("There is no C linker");
        assert_eq!(
            screen.contains("sudo pacman -S --needed base-devel"),
            linker,
            "the fix, only for its failure:\n{screen}"
        );
    }
}

#[test]
fn a_failed_build_offers_to_install_the_missing_linker_here() {
    let (_root, mut h) = failed("error: linker `cc` not found\n");
    assert!(has(&h, "There is no C linker on this machine"), "{}", h.screen());
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).click_text("Install here");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.args, ["-c", "sudo pacman -S --needed base-devel"]);
}

#[test]
fn a_failure_shows_only_its_last_five_lines_and_copies_them_all() {
    let out: String = (1..=8).map(|n| format!("line {n}\n")).collect();
    let (_root, mut h) = failed(&out);
    let screen = h.screen();
    assert!(!screen.contains("line 3") && screen.contains("line 4") && screen.contains("line 8"), "{screen}");
    h.click_text("Copy log");
    assert_eq!(h.copied().last().map(String::as_str), Some(out.trim_end()));
}

#[test]
fn a_failure_can_be_closed_or_tried_again() {
    let (root, mut h) = failed("error: the build broke\n");
    h.click_text("Close");
    let screen = h.screen();
    assert!(!screen.contains("could not be installed"), "{screen}");
    assert!(line_with(&screen, "qtools ").contains("not installed"), "{screen}");

    let (root_again, mut h) = failed("error: the build broke\n");
    scenario(root_again.path(), INSTALL, 0);
    packages(root_again.path());
    h.click_text("Retry");
    settle(&mut h);
    assert!(line_with(&h.screen(), "qtools ").contains("0.1.2"), "{}", h.screen());
    drop(root);
}

#[test]
fn a_failed_row_carries_a_danger_mark() {
    let (_root, h) = failed("error: the build broke\n");
    let (x, y) = h.find("✕").expect("the mark");
    let mark = u16::try_from(x).expect("on screen");
    let row = u16::try_from(y).expect("on screen");
    let danger = h.env().theme().color("danger");
    assert_eq!(h.fg(mark, row), danger, "{}", h.screen());
}

#[test]
fn a_leftover_build_folder_is_deleted_when_quvyta_starts() {
    let root = tempfile::tempdir().expect("temp");
    std::fs::create_dir_all(root.path().join("data/build/release")).expect("leftover");
    let h = run(Quvyta::new(machine(root.path())), 100, 24);
    assert!(!root.path().join("data/build").exists());
    assert!(!h.app().installs.other_window);
}

#[test]
fn a_build_folder_another_window_holds_is_kept_and_installs_wait() {
    let root = tempfile::tempdir().expect("temp");
    std::fs::create_dir_all(root.path().join("data/build/release")).expect("in use");
    let lock = AppLock::acquire(&root.path().join("data/build.lock")).expect("lock").expect("free");
    let mut h = run(Quvyta::new(machine(root.path())), 100, 30);
    assert!(root.path().join("data/build").exists(), "the other window's build is kept");
    h.send(Msg::Select(index("tools")));
    assert!(has(&h, "Another quvyta window is installing"), "{}", h.screen());

    scenario(root.path(), INSTALL, 0);
    packages(root.path());
    h.press("enter");
    confirm(&mut h);
    settle(&mut h);
    let screen = h.screen();
    assert!(line_with(&screen, "qtools ").contains("queued"), "{screen}");
    assert!(screen.contains("Waiting for another quvyta window"), "{screen}");
    assert!(calls(root.path()).iter().all(|call| !call.starts_with("install --locked")));

    drop(lock);
    // The window looks every few seconds; a look that meets the lock still held by a program
    // another test just started waits for the next one.
    for _ in 0..20 {
        if !h.app().installs.other_window {
            break;
        }
        h.advance(std::time::Duration::from_secs(3));
        settle(&mut h);
    }
    let screen = h.screen();
    assert!(line_with(&screen, "qtools ").contains("0.1.2"), "it went ahead once the folder was free:\n{screen}");
    assert!(!screen.contains("Another quvyta window"), "{screen}");
}

#[test]
fn reduced_motion_keeps_the_unknown_progress_still() {
    let (_root, mut h) = installing();
    h.set_reduced_motion(true);
    let (x, y) = h.find("Starting").expect("phase");
    let (x, y) = (u16::try_from(x).expect("x"), u16::try_from(y + 1).expect("bar row"));
    let before: Vec<_> = (0..20).map(|dx| h.bg(x + dx, y)).collect();
    h.advance(std::time::Duration::from_millis(700));
    let after: Vec<_> = (0..20).map(|dx| h.bg(x + dx, y)).collect();
    assert_eq!(before, after);
}

#[test]
fn install_screens_keep_the_rules_in_ascii_and_on_narrow_screens() {
    for (width, height) in [(40, 30), (48, 30), (60, 30), (100, 30)] {
        for locale in LONGEST_LANGUAGES {
            let (_root, mut h) = installing();
            h.resize(width, height).set_glyph_mode(GlyphMode::Ascii).set_locale(locale);
            h.send(Msg::ShowDetail(index("tools")));
            let mut screens = vec![h.screen()];
            h.send(install(InstallMsg::Line(index("tools"), "   Compiling serde v1.0.219".to_owned())));
            h.send(install(InstallMsg::ToggleDetails));
            screens.push(h.screen());
            h.send(install(InstallMsg::AskStop));
            screens.push(h.screen());
            h.send(install(InstallMsg::KeepRunning)).send(install(InstallMsg::Finished {
                index: index("tools"),
                outcome: Outcome::Failed(Failure::NoLinker),
                problems: vec![Problem::NoLinker(crate::checks::Distro::Other)],
            }));
            h.send(Msg::ShowDetail(index("tools")));
            screens.push(h.screen());
            let (_other, mut dialog) = harness(width, height);
            dialog.set_glyph_mode(GlyphMode::Ascii).set_locale(locale);
            dialog.send(install(InstallMsg::Ask(index("tools"))));
            screens.push(dialog.screen());
            for screen in screens {
                for forbidden in ['[', ']', '{', '}', '|', '▌'] {
                    assert!(!screen.contains(forbidden), "`{forbidden}` in {locale} at {width}x{height}:\n{screen}");
                }
                assert!(!screen.contains('⟦'), "a key is missing in {locale}:\n{screen}");
            }
        }
    }
}

/// qcode being removed, with the task made but never run, as [`installing`] does for an install.
fn removing() -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    let mut app = app_in(root.path());
    let at = index("code");
    let _ = app.update(install(InstallMsg::AskRemove(at)));
    let _ = app.update(install(InstallMsg::ConfirmRemove));
    app.selected = at;
    let h = run(app, 100, 30);
    (root, h)
}

/// What `key` says in `locale`, read from the language files themselves: a sweep over nine
/// languages cannot keep a copy of their words by hand without the copy going stale.
fn says(locale: &str, key: &str, command: &str) -> String {
    let mut i18n = qframe::i18n::I18n::builtin();
    for (file, text) in crate::LOCALES {
        assert!(i18n.add_source(file, text), "`{file}` is readable");
    }
    assert!(i18n.set_active(locale), "`{locale}` is a language quvyta speaks");
    i18n.translate(key, &[("command", qframe::i18n::Arg::from(command))])
}

/// The same text without any spacing, so a label that wrapped over two lines still reads as
/// one: what matters is that every character of it reached the screen.
fn squeezed(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

/// The screen rows of the details alone. On a wide screen the list shares the row and what the
/// list cuts belongs to the list's own sweep.
fn detail_rows(h: &Harness<Quvyta>, screen: &str, slack: u16) -> Vec<String> {
    // A few cells short of the list's column, since the panel's own margin lives inside it:
    // taking a little of the list with the details costs nothing, because the list says an
    // install is running in words of its own, never in the title looked for here.
    let skip = if h.app().wide() { usize::from(h.app().list_column().saturating_sub(slack)) } else { 0 };
    screen
        .lines()
        .map(|line| {
            let mut cells = 0;
            line.chars()
                .filter(|ch| {
                    cells += usize::from(qframe::text::width(&ch.to_string()));
                    // A character straddling the edge belongs to the details: what is asked of
                    // them here is that their words are whole, so a spare cell costs nothing.
                    cells > skip
                })
                .collect()
        })
        .collect()
}

/// Every one of `labels` is on the screen whole, and when `whole_lines`, nothing of the
/// details is cut either. A failure is read for its labels alone: it quotes cargo, whose lines
/// stay as cargo wrote them, and the advice about a missing linker is the install dialog's own
/// text, shown here as well.
fn reads_fully(h: &Harness<Quvyta>, scene: &str, locale: &str, width: u16, labels: &[String], whole_lines: bool) {
    let screen = h.screen();
    let where_ = format!("{scene} in `{locale}` at {width} columns");
    // A copyable value shortens itself on purpose; the whole of it is what the click copies.
    let copy = says(locale, "quvyta.copy-value.copy", "");
    // The details' own column exactly: what the list cuts is the list's business.
    if whole_lines {
        for row in detail_rows(h, &screen, 0).iter().filter(|row| !row.trim_end().ends_with(&copy)) {
            assert!(!row.contains('…'), "{where_} is cut:\n{screen}");
        }
    }
    // A handful of the list's last cells allowed in, so a word of the details starting on the
    // edge is never lost: far too few to reach a member's name, which is what the list would
    // otherwise lend to a title looked for here.
    let flat = squeezed(&detail_rows(h, &screen, 8).concat());
    for label in labels {
        assert!(flat.contains(&squeezed(label)), "`{label}` is not whole, {where_}:\n{screen}");
    }
}

/// The narrowest screen on which the details sit beside the list in `locale`: the split is at
/// its tightest there, so that is where a row of buttons is likeliest to lose its end. The
/// screen answers it, since a member's page shows the rest of the list only beside it.
fn splits_at(locale: &str) -> u16 {
    let (_root, mut h) = harness(40, 30);
    h.set_locale(locale).send(Msg::ShowDetail(index("tools")));
    (40..=120)
        .find(|width| {
            h.resize(*width, 30);
            h.screen().contains("qfocus")
        })
        .expect("the details sit beside the list on some screen")
}

#[test]
fn the_install_details_read_fully_in_every_language_at_every_width() {
    let (tools, packages, code) = (index("tools"), index("packages"), index("code"));
    for locale in LANGUAGES {
        let (_running, mut running) = installing();
        running.set_locale(locale).send(Msg::ShowDetail(tools));
        // A short crate name: what that line names is cargo's own word, not quvyta's.
        running.send(install(InstallMsg::Line(tools, "   Compiling ratatui v0.29.0".to_owned())));
        // With a count beside the phase too: the phase is the word the person needs, so it is
        // the count that gives way when the line does not fit.
        running.send(install(InstallMsg::Line(tools, "    Building [=>  ] 142/231: ratatui".to_owned())));

        let (_removal, mut removal) = removing();
        removal.set_locale(locale);

        // The longest row of them all: four buttons under a failure.
        let (_failure, mut failure) = failed("error: linker `cc` not found\n");
        failure.set_locale(locale);

        for width in [40, 48, 56, 60, splits_at(locale), 100] {
            running.resize(width, 40).send(Msg::ShowDetail(tools));
            let labels = [says(locale, "install.installing", "qtools"), says(locale, "install.phase.compiling", "")];
            reads_fully(&running, "an install under way", locale, width, &labels, true);

            running.send(Msg::ShowDetail(packages));
            let labels = [says(locale, "install.queued-title", "qpac"), says(locale, "install.dequeue", "")];
            reads_fully(&running, "a queued install", locale, width, &labels, true);
            running.send(Msg::ShowDetail(tools));

            removal.resize(width, 40).send(Msg::ShowDetail(code));
            let removed = [says(locale, "remove.removing", "qcode")];
            reads_fully(&removal, "a removal", locale, width, &removed, true);

            // Tall enough that the buttons under a long failure are on the screen at all.
            failure.resize(width, 70).send(Msg::ShowDetail(tools));
            let labels = [
                says(locale, "install.failed", "qtools"),
                says(locale, "install.details", ""),
                says(locale, "install.copy-log", ""),
                says(locale, "install.close", ""),
                says(locale, "install.retry", ""),
            ];
            reads_fully(&failure, "a failed install", locale, width, &labels, false);
        }
    }
}

#[test]
fn turkish_install_screens_read_naturally() {
    let (_root, mut h) = installing();
    h.set_locale("tr");
    let screen = h.screen();
    for text in ["qtools kuruluyor", "Başlıyor", "Ayrıntılar", "Durdur", "sırada"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    let (_root, mut h) = harness(100, 30);
    h.set_locale("tr").send(Msg::Select(index("tools"))).press("enter");
    let screen = h.screen();
    for text in ["qtools kurulsun mu?", "Kaynak", "crates.io'daki en son sürüm", "Vazgeç", "Kur"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
}

/// With `QUVYTA_REVIEW=1`, writes the screens of installing in both languages, wide and narrow,
/// to `target/quvyta-install-review.html` in colour for a visual review: the dialog, the
/// dialog with a problem, an install under way with its details, the queue, the stop question,
/// a failure and the question before quitting.
#[test]
fn visual_review_installs() {
    if std::env::var_os("QUVYTA_REVIEW").is_none() {
        return;
    }
    let mut fragments = Vec::new();
    let mut shot = |h: &Harness<Quvyta>, caption: String| {
        println!("{caption}\n{}", h.screen());
        fragments.push(h.html(&caption));
    };
    for (width, height) in [(100, 30), (48, 30)] {
        for locale in ["en", "tr"] {
            let size = format!("{locale} {width}x{height}");
            let (_root, mut h) = harness(width, height);
            h.set_locale(locale).send(Msg::Select(index("tools")));
            h.send(install(InstallMsg::Ask(index("tools")))).advance(DIALOG_IN);
            shot(&h, format!("dialog {size}"));

            let root = tempfile::tempdir().expect("temp");
            let app = Quvyta::new(machine(root.path()));
            std::fs::remove_file(root.path().join("bin/cc")).expect("no linker");
            let mut h = run(app, width, height);
            h.set_locale(locale).send(Msg::Select(index("tools")));
            h.send(install(InstallMsg::Ask(index("tools")))).advance(DIALOG_IN);
            shot(&h, format!("dialog without a linker {size}"));

            let (_root, mut h) = installing();
            h.resize(width, height).set_locale(locale).send(Msg::ShowDetail(index("tools")));
            let tools = index("tools");
            h.send(install(InstallMsg::Line(tools, "   Compiling ratatui v0.29.0".to_owned())));
            shot(&h, format!("compiling, no count {size}"));
            h.send(install(InstallMsg::Line(tools, "    Building [=====>  ] 142/231: ratatui, serde".to_owned())));
            h.send(install(InstallMsg::ToggleDetails));
            shot(&h, format!("compiling with a count and details {size}"));
            h.send(install(InstallMsg::ToggleDetails)).send(install(InstallMsg::AskStop)).advance(DIALOG_IN);
            shot(&h, format!("stop {size}"));
            h.send(install(InstallMsg::KeepRunning)).send(Msg::ShowDetail(index("packages")));
            shot(&h, format!("queued {size}"));
            h.send(install(InstallMsg::AskQuit)).advance(DIALOG_IN);
            shot(&h, format!("quit {size}"));

            let (_root, mut h) = failed(
                "   Compiling proc-macro2 v1.0.95\nerror: linker `cc` not found\n  |\n  = note: No such file or directory (os error 2)\n\nerror: could not compile `proc-macro2` (build script) due to 1 previous error\n",
            );
            h.resize(width, height).set_locale(locale).send(Msg::ShowDetail(index("tools")));
            shot(&h, format!("failed {size}"));
        }
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/quvyta-install-review.html");
    std::fs::write(path, qframe::runtime::html_page(&fragments)).expect("review page written");
}
