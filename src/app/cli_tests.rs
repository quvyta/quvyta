//! Starting from the command line: `quvyta install <name>...` opens on the install dialogs of
//! the members named, one after another, and never installs without them; `quvyta show <name>`
//! opens on one member's page.

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
    let app = Quvyta::new(machine(root));
    let app = match start {
        Start::Install(members) => app.asking(members),
        Start::Show(member) => app.showing(member),
        Start::List => app,
    };
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

#[test]
fn a_dialog_the_command_line_asked_for_opens_over_the_list_even_from_the_settings() {
    let root = tempfile::tempdir().expect("temp");
    let mut app = Quvyta::new(machine(root.path())).asking(vec![index("tools")]);
    // The Settings tab is opened while cargo is still answering what is installed.
    let _ = app.update(Msg::Tab(Tab::Settings));
    let _ = app.update(Msg::Inventory(crate::inventory::Inventory::read(&app.machine)));
    assert_eq!(app.tab, Tab::Apps, "the dialog belongs to the list, which shows what it starts");
    assert_eq!(app.selected, index("tools"));
}

#[test]
fn show_opens_on_the_member_s_details_beside_the_list() {
    let root = tempfile::tempdir().expect("temp");
    let h = started(root.path(), &["show", "qfocus"], 100);
    let screen = h.screen();
    assert_eq!(h.app().selected, index("focus"));
    assert_eq!(h.app().tab, Tab::Apps);
    assert!(screen.contains("Quvyta Focus") && screen.contains("qtools"), "{screen}");
    assert!(super::tests::line_with(&screen, "qfocus").contains('▌'), "its row is the one selected:\n{screen}");
    assert!(h.app().installs.dialog.is_none() && h.handoffs().is_empty(), "showing starts nothing");
    assert!(h.is_focused("family"), "the list keeps the keys:\n{screen}");
}

#[test]
fn show_on_a_narrow_screen_opens_the_member_s_page_and_esc_goes_back_to_the_list() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = started(root.path(), &["show", "qfocus"], 48);
    let screen = h.screen();
    assert!(h.app().detail_page, "{screen}");
    assert!(screen.contains("Quvyta Focus") && screen.contains("Back") && !screen.contains("qtools"), "{screen}");
    h.press("esc");
    let screen = h.screen();
    assert!(!h.app().detail_page && screen.contains("qtools"), "{screen}");
    assert!(super::tests::line_with(&screen, "qfocus").contains('▌'), "the member stays selected:\n{screen}");
}

#[test]
fn show_opens_any_member_even_one_still_to_come_and_quvyta_itself() {
    let root = tempfile::tempdir().expect("temp");
    for (name, title) in [("qdesk", "Quvyta Desktop"), ("quvyta", "Running"), ("QTOOLS", "Quvyta Tools")] {
        for width in [100, 48] {
            let h = started(root.path(), &["show", name], width);
            assert!(h.screen().contains(title), "{name} at {width}:\n{}", h.screen());
            assert!(h.app().installs.dialog.is_none(), "{name} at {width}:\n{}", h.screen());
        }
    }
}
