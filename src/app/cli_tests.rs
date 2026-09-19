//! Starting from the command line: `quvyta install <name>...` opens on the install dialogs of
//! the members named, one after another, and never installs without them.

use std::path::Path;

use qframe::icons::GlyphMode;

use super::install_tests::{DIALOG_IN, calls, click_beside, has, settle};
use super::tests::{TOAST_IN, env, index, machine};
use super::*;
use crate::cli::{self, Parsed, Start};
use crate::install::tests::{packages, scenario};

/// quvyta started as `quvyta <args>` on the machine of the list's tests, its first frames drawn.
fn started(root: &Path, args: &[&str], width: u16) -> Harness<Quvyta> {
    let Parsed::Open(start) = cli::parse(args.iter().map(std::ffi::OsString::from)) else {
        panic!("{args:?} opens the screen")
    };
    let asked = match start {
        Start::Install(members) => members,
        Start::List => Vec::new(),
    };
    let app = Quvyta::new(machine(root)).asking(asked);
    let mut h = Harness::with_env(app, env(), width, 30);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    // The first frame asked cargo what is installed; this one hears it.
    h.render();
    h.advance(DIALOG_IN);
    h
}

fn installs_ran(root: &Path) -> usize {
    calls(root).iter().filter(|call| call.starts_with("install --locked")).count()
}

#[test]
fn install_opens_on_the_dialog_and_installs_nothing_unasked() {
    for name in ["tools", "qtools", "quvyta-tools"] {
        let root = tempfile::tempdir().expect("temp");
        let mut h = started(root.path(), &["install", name], 100);
        assert!(has(&h, "Install qtools?"), "{name}:\n{}", h.screen());
        assert_eq!(h.app().selected, index("tools"));
        h.press("esc");
        settle(&mut h);
        assert!(!has(&h, "Install qtools?"), "{}", h.screen());
        assert_eq!(installs_ran(root.path()), 0, "closing the dialog installs nothing");
    }
}

#[test]
fn several_names_ask_one_after_another_and_queue_what_is_agreed_to() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = started(root.path(), &["install", "qtools", "qpac", "qtools"], 100);
    packages(root.path());
    scenario(root.path(), "  Installing /x/.cargo/bin/qtools\n", 0);
    assert!(has(&h, "Install qtools?"), "{}", h.screen());
    click_beside(&mut h, "Cancel", "Install");
    h.advance(DIALOG_IN);
    assert!(has(&h, "Install qpac?"), "the next dialog follows:\n{}", h.screen());
    h.press("esc").advance(DIALOG_IN);
    settle(&mut h);
    assert!(!has(&h, "Install q"), "each name asks once:\n{}", h.screen());
    assert_eq!(calls(root.path()).iter().filter(|call| *call == "install --locked quvyta-tools").count(), 1);
    assert_eq!(installs_ran(root.path()), 1, "qpac was not agreed to");
}

#[test]
fn a_member_that_is_there_is_shown_with_a_note_instead_of_installed_again() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = started(root.path(), &["install", "qcode"], 100);
    h.advance(TOAST_IN);
    assert!(h.app().installs.dialog.is_none());
    assert_eq!(h.app().selected, index("code"));
    assert!(has(&h, "Already installed: qcode"), "{}", h.screen());
    assert!(has(&h, "Open"), "its details are beside the list:\n{}", h.screen());

    let narrow = started(root.path(), &["install", "code"], 48);
    assert!(narrow.app().detail_page, "on a narrow screen its details have the screen");
}

#[test]
fn installed_names_are_skipped_on_the_way_to_the_ones_to_install() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = started(root.path(), &["install", "qcode", "quvyta", "qtools"], 100);
    h.advance(TOAST_IN);
    assert!(has(&h, "Install qtools?"), "{}", h.screen());
    assert!(has(&h, "Already installed: qcode, quvyta"), "{}", h.screen());
}

#[test]
fn without_arguments_the_list_opens_as_before() {
    let root = tempfile::tempdir().expect("temp");
    let h = started(root.path(), &[], 100);
    assert!(h.app().installs.dialog.is_none());
    assert_eq!(h.app().selected, 0);
    assert!(!has(&h, "Already installed"));
}
