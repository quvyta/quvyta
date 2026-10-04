//! An app that does not run on this machine: still on the list, saying why it cannot be
//! installed from here, and offered no way to install it — from the keyboard, from the mouse and
//! from the command line. On Arch Linux, and on the platform an app is built for, nothing
//! changes.
//!
//! Every test stands in a temporary root and names the distribution in `root/etc/os-release`
//! before the machine is built, since that is when quvyta reads it. Nothing is installed, nothing
//! leaves the folder and no network is used: cargo is the stand-in in `tests/fake-cargo`.

use std::path::Path;

use qframe::icons::GlyphMode;
use qframe::runtime::HandoffOutcome;
use tempfile::TempDir;

use super::cli_tests::{installs_ran, started};
use super::install_tests::{DIALOG_IN, calls, click_beside, has, run, settle};
use super::tests::{LANGUAGES, LIST, TOAST_IN, index, line_with, machine, start};
use super::*;
use crate::ecosystem::{APPS, NotHere};
use crate::install::tests::packages;
use crate::inventory::tests::{installed_command, machine_with_cargo};
use crate::updates::tests::search_scenario;

/// The distribution `root/etc/os-release` names. It is written before the machine is built,
/// because quvyta reads the file when it starts, not afterwards.
fn distro(root: &Path, id: &str) {
    std::fs::create_dir_all(root.join("etc")).expect("folder");
    std::fs::write(root.join("etc/os-release"), format!("ID={id}\n")).expect("os-release");
}

/// The app list on a machine that is not Arch Linux, `width` by `height`, where qtools and qpac
/// run nowhere near and nothing of theirs is installed.
fn debian(width: u16, height: u16) -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    distro(root.path(), "debian");
    let h = start(root.path(), width, height);
    (root, h)
}

/// The calls the stand-in cargo was given, keeping only those that start with `first`.
fn cargo_calls(root: &Path, first: &str) -> Vec<String> {
    calls(root).into_iter().filter(|call| call.starts_with(first)).collect()
}

#[test]
fn an_app_that_only_runs_on_arch_linux_is_not_offered_elsewhere() {
    let (root, mut h) = debian(100, 30);
    let screen = h.screen();
    println!("{screen}");
    for command in ["qtools", "qpac"] {
        let row = line_with(&screen, &format!("{command} "));
        assert!(row.contains("Arch Linux only"), "`{command}` says why it is not there:\n{row}");
        assert!(!row.contains("not installed"), "`not installed` invites the install:\n{row}");
    }

    h.press("down").press("down").press("enter").advance(TOAST_IN);
    let screen = h.screen();
    println!("{screen}");
    assert!(!has(&h, "Install qtools?"), "no install question opens:\n{screen}");
    assert!(h.app().installs.dialog.is_none(), "no dialog at all:\n{screen}");
    assert!(screen.contains("qtools runs only on Arch Linux"), "the reason is told:\n{screen}");
    assert!(screen.contains("qtools") && !h.app().detail_page, "and the list is still on screen:\n{screen}");
    assert_eq!(installs_ran(root.path()), 0, "nothing ran");
}

#[test]
fn the_same_machine_offers_them_on_arch_linux() {
    let (root, _debian) = debian(100, 30);
    // The very same folder, named Arch Linux now: the distribution is the only thing that
    // changed, and the apps that cannot run anywhere else are offered again.
    distro(root.path(), "arch");
    let mut h = start(root.path(), 100, 30);
    let screen = h.screen();
    for command in ["qtools", "qpac"] {
        let row = line_with(&screen, &format!("{command} "));
        assert!(row.contains("not installed"), "`{command}` can be installed here:\n{row}");
        assert!(!row.contains("Arch Linux only"), "and nothing is held back:\n{row}");
    }
    h.press("down").press("down").press("enter").advance(DIALOG_IN);
    assert!(has(&h, "Install qtools?"), "{}", h.screen());
    assert_eq!(installs_ran(root.path()), 0, "and it installs nothing without being agreed to");
}

#[test]
fn an_app_that_cannot_run_here_has_no_install_button() {
    let (root, mut h) = debian(48, 30);
    h.press("down").press("down").press("enter");
    let screen = h.screen();
    println!("{screen}");
    assert!(h.app().detail_page, "its page has the screen:\n{screen}");
    assert!(screen.contains("Quvyta Tools"), "{}", screen);
    assert!(screen.contains("Arch Linux only"), "the reason stands where the button would be:\n{screen}");
    assert!(!screen.contains("Install"), "there is no Install on it at all:\n{screen}");

    h.press("enter").advance(TOAST_IN);
    assert!(h.app().installs.dialog.is_none(), "enter asks nothing:\n{}", h.screen());
    assert_eq!(installs_ran(root.path()), 0, "and nothing is installed");
}

#[test]
fn the_command_line_does_not_ask_to_install_it_either() {
    let root = tempfile::tempdir().expect("temp");
    distro(root.path(), "debian");
    let mut h = started(root.path(), &["install", "tools"], 100);
    h.advance(TOAST_IN);
    let screen = h.screen();
    assert!(!has(&h, "Install qtools?"), "{}", screen);
    assert!(h.app().installs.dialog.is_none(), "no question opens:\n{screen}");
    assert!(screen.contains("qtools runs only on Arch Linux"), "the reason is told, naming the app:\n{screen}");
    assert_eq!(installs_ran(root.path()), 0, "nothing is installed");
}

#[test]
fn an_app_that_is_already_installed_can_still_be_opened_and_removed() {
    let root = tempfile::tempdir().expect("temp");
    distro(root.path(), "debian");
    let app = Quvyta::new(machine(root.path()));
    // cargo put qtools on this machine, so quvyta is looking at an app it did not offer.
    std::fs::write(root.path().join("bin/list.out"), format!("{LIST}quvyta-tools v0.1.2:\n    qtools\n"))
        .expect("list");
    let qtools = installed_command(&app.machine, "qtools");
    packages(root.path());
    let mut h = run(app, 100, 30);
    h.send(Msg::Select(index("tools"))).set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).press("enter");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, qtools.as_os_str(), "it opens from where cargo put it");

    h.render().click_text("Remove").advance(DIALOG_IN);
    assert!(has(&h, "Remove qtools?"), "{}", h.screen());
    click_beside(&mut h, "Cancel", "Remove");
    settle(&mut h);
    assert_eq!(
        cargo_calls(root.path(), "uninstall"),
        ["uninstall quvyta-tools"],
        "cargo put it there, so cargo takes it away"
    );
    let screen = h.advance(TOAST_IN).screen();
    assert!(screen.contains("qtools removed"), "{screen}");
    assert!(screen.contains("Arch Linux only"), "and it is not offered once it is gone:\n{screen}");
}

#[test]
fn an_app_installed_on_this_machine_is_still_updated() {
    let root = tempfile::tempdir().expect("temp");
    distro(root.path(), "debian");
    let app = Quvyta::new(machine(root.path()));
    std::fs::write(root.path().join("bin/list.out"), format!("{LIST}quvyta-tools v0.1.1:\n    qtools\n"))
        .expect("list");
    installed_command(&app.machine, "qtools");
    search_scenario(root.path(), "quvyta-tools = \"0.1.2\"\n", 0);
    packages(root.path());
    let mut h = run(app, 100, 30);
    settle(&mut h);
    let screen = h.screen();
    let row = line_with(&screen, "qtools ");
    assert!(row.contains("0.1.1  new 0.1.2"), "{screen}");
    assert!(!row.contains("Arch Linux only"), "it is here, so the platform is not in the way:\n{row}");

    h.send(Msg::Select(index("tools")));
    let screen = h.screen();
    assert!(screen.contains("Installed  0.1.1, ~/.cargo/bin/qtools"), "the page still says where it is:\n{screen}");
    assert!(line_with(&screen, "Open").contains("Update"), "and still offers the update:\n{screen}");

    h.press("u").advance(DIALOG_IN);
    let screen = h.screen();
    assert!(has(&h, "Update qtools from 0.1.1 to 0.1.2?"), "{}", screen);
    click_beside(&mut h, "Cancel", "Update");
    settle(&mut h);
    assert_eq!(cargo_calls(root.path(), "install --locked"), ["install --locked quvyta-tools --version 0.1.2"]);
}

#[test]
fn the_words_are_the_wizard_s_words_where_the_wizard_has_them() {
    for locale in ["en", "tr"] {
        let (_root, mut h) = debian(100, 30);
        h.set_locale(locale);
        let say = |key: &str| h.env().i18n().translate(key, &[]);
        let short = say("platform.arch-only");
        let screen = h.screen();
        let row = line_with(&screen, "qtools ");
        assert!(row.contains(&short), "{locale}: the row says it in its own short words:\n{row}");
        let missing = say("row.missing");
        assert!(!row.contains(&missing), "{locale}: `{missing}` invites the install:\n{row}");

        // The same claim on the first start, where the wizard has a whole sentence: its line is the
        // app's own words with the claim after them, so the claim is these words and a period.
        let first = tempfile::tempdir().expect("temp");
        distro(first.path(), "debian");
        // A first start has no `launcher.conf`, so the machine is built without one. The screen is
        // wide enough for the claim to stay on the app's own line, so what is compared here is
        // the words and not where a language happened to wrap them.
        let mut wizard = run(Quvyta::new(machine_with_cargo(first.path(), "", 0)), 100, 30);
        wizard.set_locale(locale);
        let next = wizard.env().i18n().translate("quvyta.wizard.next", &[]);
        wizard.click_text(&next);
        let say = |key: &str| wizard.env().i18n().translate(key, &[]);
        let long = say("wizard.arch-only");
        let claim = long.rsplit("  ").next().expect("the claim after the line");
        let own = say("wizard.line-tools");
        let screen = wizard.screen();
        let line = line_with(&screen, &own).trim_start();
        assert_eq!(claim, format!("{short}."), "{locale}: the same claim, in a whole sentence");
        assert!(line.starts_with(&own) && line.trim_end().ends_with(claim), "{locale}:\n{line}");
    }
}

#[test]
fn an_unbuilt_app_on_this_platform_is_also_told_why() {
    // The case cannot happen where the tests run, so the rule is pinned against this platform's
    // own answer rather than against a machine that has one of these apps built for it.
    for arch in [false, true] {
        for key in ["explorer", "browser", "cli"] {
            let app = APPS.iter().find(|app| app.key == key).expect("an app");
            let (reason, offered) = (app.not_here(arch), app.offered(arch));
            if app.runs_here() {
                assert_eq!(reason, None, "{key} is built for this platform, so nothing is in the way");
                assert!(offered, "{key} is offered where it is built");
            } else {
                assert_eq!(reason, Some(NotHere::UnixOnly), "{key} is not built for this platform");
                assert!(!offered, "{key} is not offered where it does not build");
            }
        }
    }
}

#[test]
fn narrow_screens_and_ascii_keep_the_rules_for_these_rows() {
    for locale in LANGUAGES {
        let (_root, mut h) = debian(40, 24);
        h.set_glyph_mode(GlyphMode::Ascii).set_locale(locale);
        let short = h.env().i18n().translate("platform.arch-only", &[]);
        h.press("down").press("down");
        let screen = h.screen();
        for forbidden in ['[', ']', '{', '}', '|', '▌', '⟦'] {
            assert!(!screen.contains(forbidden), "`{forbidden}` in {locale}:\n{screen}");
        }
        let said = line_with(&screen, &short);
        assert!(!said.contains('…'), "the reason is not cut short in {locale}:\n{screen}");

        // And on the app's own page, where the Install button would be.
        h.press("enter");
        let screen = h.screen();
        let said = line_with(&screen, &short);
        assert!(!said.contains('…'), "the reason is not cut short in {locale}:\n{screen}");
        for forbidden in ['[', ']', '{', '}', '|', '▌', '⟦'] {
            assert!(!screen.contains(forbidden), "`{forbidden}` in {locale}:\n{screen}");
        }
    }
}
