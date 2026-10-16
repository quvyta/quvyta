//! An app that needs something from the machine before it can run: qbrow and the Chromium it
//! draws its pages with.
//!
//! Every test here starts from the person's hands, on the machine of the list's own tests: a
//! temporary root whose `bin` holds only what the test put there, so a browser is there only
//! where the test wrote one. Nothing is installed for real — the stand-in cargo is what runs — and
//! the line that needs sudo is recorded as a handoff, never run.

use std::path::Path;

use qframe::runtime::HandoffOutcome;
use tempfile::TempDir;

use super::install_tests::{DIALOG_IN, calls, confirm, run, settle};
use super::tests::{LANGUAGES, LIST, harness, index, line_with, machine};
use super::*;
use crate::checks::{Distro, Problem};
use crate::inventory::tests::{installed_command, write_program};
use crate::updates::tests::search_scenario;

/// The program names qbrow itself accepts, in its own order, read out of its own source. quvyta
/// must not disagree with the program it is about to start about what is missing, so this is
/// qbrow's list and not one of its own: [`crate::checks::BROWSERS`] is the same one.
const BROWSERS: [&str; 4] = ["chromium", "chromium-browser", "google-chrome-stable", "google-chrome"];

/// The one line quvyta offers on Arch Linux, which is the machine these tests are on.
const ARCH: &str = "sudo pacman -S --needed chromium";

/// The mark a surface carries down its left edge.
const PILLAR: &str = "\u{258c}";

/// The three lines it offers on the distributions it knows, for a machine that names none.
const KNOWN: [&str; 3] = ["sudo pacman -S --needed chromium", "sudo apt install chromium", "sudo dnf install chromium"];

/// What the checks answered for the install dialog, so a test says which question it is looking at.
fn problems_of(h: &Harness<Quvyta>) -> Option<Vec<Problem>> {
    h.app().installs.dialog.as_ref().and_then(|dialog| dialog.problems.clone())
}

/// How many installs have run on the machine in `root`.
fn installs_ran(root: &Path) -> usize {
    calls(root).iter().filter(|call| call.starts_with("install --locked")).count()
}

/// A machine in a temporary root that names `os_release` in its own `os-release` and has no
/// browser of any name, with the application on it and cargo's record read.
fn on(os_release: &str) -> (TempDir, Quvyta) {
    let root = tempfile::tempdir().expect("temp");
    std::fs::create_dir_all(root.path().join("etc")).expect("folder");
    std::fs::write(root.path().join("etc/os-release"), os_release).expect("os-release");
    let app = Quvyta::new(machine(root.path()));
    (root, app)
}

/// The install dialog for qbrow on such a machine: it is not installed, so `enter` asks first.
fn installing_browser(os_release: &str) -> (TempDir, Harness<Quvyta>) {
    let (root, app) = on(os_release);
    let mut h = run(app, 100, 44);
    h.send(Msg::Select(index("browser"))).press("enter").advance(DIALOG_IN);
    (root, h)
}

/// The question between **Open** and qbrow: it is installed, so `enter` would hand the terminal
/// over, and the question stands in the way of a program that is about to fail.
fn asking_to_open_on(os_release: &str) -> (TempDir, Harness<Quvyta>) {
    let (root, app) = on(os_release);
    let list = format!("{LIST}quvyta-browser v0.1.2:\n    qbrow\n");
    std::fs::write(root.path().join("bin/list.out"), list).expect("scenario");
    installed_command(&app.machine, "qbrow");
    let mut h = run(app, 100, 44);
    settle(&mut h);
    h.send(Msg::Select(index("browser"))).press("enter").advance(DIALOG_IN);
    (root, h)
}

#[test]
fn installing_the_browser_warns_that_it_needs_one_and_installs_when_agreed_to() {
    let (root, mut h) = installing_browser("ID=arch\n");
    let screen = h.screen();
    for text in
        ["Install qbrow?", "Chromium is needed to draw the pages", "To install one on Arch Linux", ARCH, "Install here"]
    {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    // qbrow builds without a browser, so the warning does not hold the install up.
    assert!(line_with(&screen, "Cancel").contains("Install"), "Install is there to press:\n{screen}");
    assert!(!screen.contains("Check again"), "nothing is in the way of the build:\n{screen}");
    assert_eq!(installs_ran(root.path()), 0, "nothing was installed yet");
    assert!(h.handoffs().is_empty(), "nothing was run");
    // The words come from the checks themselves, so the screen and the fix line cannot disagree.
    assert_eq!(problems_of(&h), Some(vec![Problem::NoBrowser(Distro::Arch)]));

    confirm(&mut h);
    settle(&mut h);
    assert_eq!(installs_ran(root.path()), 1, "agreeing installs qbrow");
    assert!(h.handoffs().is_empty(), "and the browser line was never run unasked");
}

#[test]
fn an_update_of_the_browser_already_installed_is_not_warned_about_it() {
    let (root, app) = on("ID=arch\n");
    let list = format!("{LIST}quvyta-browser v0.1.2:\n    qbrow\n");
    std::fs::write(root.path().join("bin/list.out"), list).expect("scenario");
    installed_command(&app.machine, "qbrow");
    search_scenario(root.path(), "quvyta-browser = \"0.1.3\"    # A browser\n", 0);
    let mut h = run(app, 100, 44);
    settle(&mut h);
    h.send(Msg::Select(index("browser"))).press("u").advance(DIALOG_IN);
    settle(&mut h);
    let screen = h.screen();
    assert!(screen.contains("0.1.3"), "the update question is open:\n{screen}");
    assert!(!screen.contains("Chromium is needed"), "running it is the question before opening:\n{screen}");
    assert!(line_with(&screen, "Cancel").contains("Update"), "the update is offered:\n{screen}");
}

#[test]
fn install_here_runs_the_browser_command_in_the_terminal_and_checks_again() {
    let (_root, mut h) = installing_browser("ID=arch\n");
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).click_text("Install here");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, "sh");
    assert_eq!(request.args, ["-c", ARCH], "the very line shown");
    assert!(request.pause, "what it printed stays to be read");
    assert_eq!(request.notice.as_deref(), Some(format!("Running {ARCH}").as_str()));
    settle(&mut h);
    // The line ran nowhere: the root still has no browser, and the checks say so again.
    assert_eq!(problems_of(&h), Some(vec![Problem::NoBrowser(Distro::Arch)]));
    let screen = h.screen();
    assert!(screen.contains("Chromium is needed to draw the pages"), "checked again:\n{screen}");
    assert!(line_with(&screen, "Cancel").contains("Install"), "the install is still offered:\n{screen}");
}

#[test]
fn with_a_browser_on_the_machine_nothing_is_asked() {
    // Every name qbrow looks for, one at a time: `chromium` is the first, and the other three are
    // what it falls back to.
    for name in BROWSERS {
        let (root, mut h) = harness(100, 44);
        write_program(&root.path().join(format!("bin/{name}")), "#!/bin/sh\nexit 0\n");
        h.send(Msg::Select(index("browser"))).press("enter");
        let screen = h.screen();
        assert!(screen.contains("Install qbrow?"), "{name}:\n{screen}");
        assert!(!screen.contains("Chromium is needed"), "{name} is what qbrow looks for:\n{screen}");
        assert!(line_with(&screen, "Cancel").contains("Install"), "{name}: Install is ready:\n{screen}");
        assert_eq!(problems_of(&h), Some(Vec::new()), "{name} is on the machine");
    }
}

#[test]
fn opening_the_browser_without_one_asks_first_and_starts_nothing() {
    let (root, h) = asking_to_open_on("ID=arch\n");
    let screen = h.screen();
    for text in ["Open qbrow?", "Chromium is needed to draw the pages", ARCH, "Install here", "Not now"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(h.handoffs().is_empty(), "the terminal is not handed to a program about to fail");
    assert_eq!(installs_ran(root.path()), 0, "nothing was installed");
}

#[test]
fn after_the_browser_is_installed_the_app_opens() {
    let (root, mut h) = asking_to_open_on("ID=arch\n");
    // The fix does its work: a browser is on the machine afterwards.
    write_program(&root.path().join("bin/chromium"), "#!/bin/sh\nexit 0\n");
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).click_text("Install here");
    assert_eq!(h.handoffs().len(), 1, "the line ran in the terminal");
    settle(&mut h);
    let [_, opening] = h.handoffs() else { panic!("qbrow opens itself: {:?}", h.handoffs()) };
    assert_eq!(opening.program, root.path().join("home/.cargo/bin/qbrow").as_os_str());
    assert!(opening.args.is_empty());
    assert!(!h.screen().contains("Open qbrow?"), "the question is over:\n{}", h.screen());
}

#[test]
fn not_now_leaves_the_app_alone() {
    for close in ["Not now", "esc", "×"] {
        let (root, mut h) = asking_to_open_on("ID=arch\n");
        assert!(h.screen().contains("Open qbrow?"), "{close} is asked with the question:\n{}", h.screen());
        if close == "esc" {
            h.press("esc")
        } else {
            h.click_text(close)
        };
        assert!(h.handoffs().is_empty(), "{close} starts nothing");
        let screen = h.screen();
        assert!(!screen.contains("Open qbrow?"), "{close}:\n{screen}");
        // The list is back with the app on it, still installed and still the one selected.
        assert!(line_with(&screen, "qbrow ").contains("0.1.2"), "{close}:\n{screen}");
        assert!(h.app().selected == index("browser"), "{close} keeps the selection");
        assert!(screen.contains("Open"), "{close} leaves it openable:\n{screen}");
        assert_eq!(installs_ran(root.path()), 0, "{close} installs nothing");
    }
}

#[test]
fn on_a_system_quvyta_does_not_know_the_commands_are_shown_to_copy_and_none_is_run() {
    // The install dialog.
    let (_root, h) = installing_browser("ID=alpine\n");
    let screen = h.screen();
    for command in KNOWN {
        assert!(screen.contains(command), "`{command}` is on screen to copy:\n{screen}");
    }
    assert!(!screen.contains("Install here"), "which of them is right is not known here:\n{screen}");
    assert!(screen.contains("Run the command for your system yourself"), "{screen}");
    assert!(h.handoffs().is_empty(), "none of them is run");

    // And the question that stands before the app is opened.
    let (_root, h) = asking_to_open_on("ID=alpine\n");
    let screen = h.screen();
    for command in KNOWN {
        assert!(screen.contains(command), "`{command}` is on screen to copy:\n{screen}");
    }
    assert!(!screen.contains("Install here"), "{screen}");
    assert!(screen.contains("Run the command for your system yourself"), "{screen}");
    assert!(h.handoffs().is_empty(), "none of them is run");
}

#[test]
fn the_install_view_still_reads_fully_in_every_language_on_a_narrow_screen() {
    // The same text without spacing, so a label that wrapped over two lines still reads as one:
    // what matters is that every character of it reached the screen.
    let whole = |text: &str| text.chars().filter(|ch| !ch.is_whitespace()).collect::<String>();
    for locale in LANGUAGES {
        for (own, (_, mut h)) in
            [(&[][..], installing_browser("ID=arch\n")), (&["needs.not-now"], asking_to_open_on("ID=arch\n"))]
        {
            h.set_locale(locale).resize(40, 44);
            let screen = h.screen();
            // A copyable value shortens itself on purpose; what the click copies is the whole of it.
            let copy = h.env().i18n().translate("quvyta.copy-value.copy", &[]);
            for line in screen.lines().filter(|line| !line.trim_end().ends_with(&copy)) {
                assert!(!line.contains('…'), "a line is cut in `{locale}`:\n{screen}");
            }
            // The dialog's pillar runs down its left edge, between the lines of a label that
            // wrapped, so it is left out of what is compared: it is decoration, not a letter.
            let flat = whole(&screen.replace(PILLAR, " "));
            let keys = ["needs.no-browser", "checks.install-here", "checks.sudo-here"];
            for key in keys.into_iter().chain(own.iter().copied()) {
                let label = h.env().i18n().translate(key, &[]);
                assert!(flat.contains(&whole(&label)), "`{label}` is not whole in `{locale}`:\n{screen}");
            }
        }
    }
}

#[test]
fn the_new_words_are_the_wizards_own_words_where_they_overlap() {
    for locale in ["en", "tr"] {
        // The button that runs a fix line and the note under it are the words the linker fix
        // already uses, so the two fixes speak with one voice.
        let (root, app) = on("ID=arch\n");
        std::fs::remove_file(root.path().join("bin/cc")).expect("no linker");
        let mut linker = run(app, 100, 44);
        linker.set_locale(locale).send(Msg::Select(index("tools"))).press("enter");
        let button = words(&linker, "checks.install-here");
        let note = words(&linker, "checks.sudo-here");

        let (_root, mut browser) = installing_browser("ID=arch\n");
        browser.set_locale(locale);
        let screen = whole(&browser.screen());
        assert!(screen.contains(&whole(&button)), "`{locale}` should say `{button}`");
        assert!(screen.contains(&whole(&note)), "`{locale}` should say `{note}`:\n{}", browser.screen());

        // And the question before qbrow is opened says the same words.
        let (_root, mut question) = asking_to_open_on("ID=arch\n");
        question.set_locale(locale);
        let screen = whole(&question.screen());
        assert!(screen.contains(&whole(&button)), "`{locale}` should say `{button}`");
        assert!(screen.contains(&whole(&note)), "`{locale}` should say `{note}`");
    }
}

/// The text of the language key `key`, in the harness' language, as the screen's own words.
fn words(h: &Harness<Quvyta>, key: &str) -> String {
    h.env().i18n().translate(key, &[])
}

/// The same text without spacing, so a sentence that wrapped over two lines still reads as one:
/// what matters is that every character of it reached the screen.
/// `text` without its spacing and without the dialog's pillar, which runs between the lines of a
/// sentence that wrapped.
fn whole(text: &str) -> String {
    text.replace(PILLAR, " ").chars().filter(|ch| !ch.is_whitespace()).collect()
}
