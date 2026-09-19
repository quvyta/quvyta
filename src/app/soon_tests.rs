//! A member that is not released yet: listed with what it will be, never installed or updated
//! from quvyta, and opened like any other when a build of it is already on the machine.

use std::path::Path;

use qframe::icons::GlyphMode;

use super::install_tests::{DIALOG_IN, calls, has, run, settle};
use super::tests::{LIST, TOAST_IN, harness, index, line_with, machine};
use super::*;
use crate::install::tests::packages;
use crate::inventory::tests::installed_command;
use crate::updates::tests::search_scenario;

fn desk() -> usize {
    index("desk")
}

/// What crates.io answers when something named like the member is there after all: a newer
/// version than the local build, and qcode's update beside it.
const SEARCH: &str = "\
quvyta-code = \"0.1.2\"
quvyta-desktop = \"9.9.9\"
";

/// The machine of [`machine`] in `root` where cargo also lists a local build of qdesk, installed
/// with `cargo install --path`, and crates.io answers [`SEARCH`].
fn with_local_build(root: &Path) -> Harness<Quvyta> {
    let app = Quvyta::new(machine(root));
    let list = format!("{LIST}quvyta-desktop v0.1.0 ({}):\n    qdesk\n", root.join("desk").display());
    std::fs::write(root.join("bin/list.out"), list).expect("scenario");
    installed_command(&app.machine, "qdesk");
    search_scenario(root, SEARCH, 0);
    packages(root);
    let mut h = run(app, 100, 30);
    settle(&mut h);
    h
}

fn installs(root: &Path) -> usize {
    calls(root).iter().filter(|call| call.starts_with("install --locked")).count()
}

#[test]
fn its_row_says_it_is_coming_between_the_programs_and_the_framework() {
    let (_root, mut h) = harness(100, 24);
    let screen = h.screen();
    assert!(line_with(&screen, "qdesk ").contains("coming soon"), "{screen}");
    assert!(!line_with(&screen, "qdesk ").contains("not installed"), "{screen}");
    let rows: Vec<&str> = screen.lines().skip(1).collect();
    let at = |command: &str| rows.iter().position(|row| row.contains(command)).expect("listed");
    assert!(at("qpac ") < at("qdesk ") && at("qdesk ") < at("qframe "), "{screen}");
    h.set_locale("tr");
    assert!(line_with(&h.screen(), "qdesk ").contains("yakında"), "{}", h.screen());
}

#[test]
fn its_details_say_what_it_will_be_and_that_it_is_not_out() {
    let (_root, mut h) = harness(100, 30);
    h.send(Msg::Select(desk()));
    let screen = h.screen();
    for text in [
        "Quvyta Desktop",
        "Coming soon",
        "quvyta-desktop",
        "qdesk",
        "a launcher and a file manager",
        "https://github.com/quvyta/desktop",
        "Not released yet",
    ] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    for text in ["Install", "cargo install", "A beta", "enter"] {
        assert!(!screen.contains(text), "`{text}` is offered:\n{screen}");
    }
    h.set_locale("tr");
    let screen = h.screen();
    for text in ["Yakında", "Henüz yayımlanmadı", "SSH üzerinden"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("Kur "), "{screen}");
}

#[test]
fn nothing_on_the_screen_installs_it() {
    let root = tempfile::tempdir().expect("temp");
    let app = Quvyta::new(machine(root.path()));
    packages(root.path());
    let mut h = run(app, 100, 30);
    h.send(Msg::Select(desk())).press("enter").advance(DIALOG_IN);
    assert!(h.app().installs.dialog.is_none(), "enter asks nothing:\n{}", h.screen());
    h.send(Msg::Install(InstallMsg::Ask(desk()))).advance(DIALOG_IN);
    assert!(h.app().installs.dialog.is_none(), "neither does a message:\n{}", h.screen());
    h.send(Msg::Install(InstallMsg::Retry(desk())));
    h.press("u").send(Msg::Updates(UpdateMsg::InstallAll));
    settle(&mut h);
    assert!(!h.app().installs.has(desk()), "nothing queued it");
    assert_eq!(installs(root.path()), 0);
    assert!(h.handoffs().is_empty(), "and nothing opened");
}

#[test]
fn a_narrow_screen_shows_its_page_without_a_way_to_install() {
    let (_root, mut h) = harness(48, 24);
    h.send(Msg::Select(desk())).press("enter");
    let screen = h.screen();
    assert!(h.app().detail_page, "{screen}");
    assert!(screen.contains("Coming soon") && screen.contains("Not released yet"), "{screen}");
    h.press("enter").advance(DIALOG_IN);
    assert!(h.app().installs.dialog.is_none(), "{}", h.screen());
}

#[test]
fn the_command_line_never_opens_on_its_install_dialog() {
    let root = tempfile::tempdir().expect("temp");
    let app = Quvyta::new(machine(root.path())).asking(vec![desk()]);
    let mut h = run(app, 100, 30);
    h.advance(DIALOG_IN).advance(TOAST_IN);
    assert!(h.app().installs.dialog.is_none(), "{}", h.screen());
    assert!(!has(&h, "Already installed"), "it is not installed either:\n{}", h.screen());
}

#[test]
fn a_local_build_is_installed_elsewhere_opens_and_is_never_updated() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = with_local_build(root.path());
    let screen = h.screen();
    let row = line_with(&screen, "qdesk ");
    assert!(row.contains("unknown version") && !row.contains("new"), "{screen}");
    assert!(screen.contains("1 update"), "only qcode's counts:\n{screen}");

    h.send(Msg::Select(desk()));
    let screen = h.screen();
    assert!(screen.contains("Installed without cargo") && screen.contains("Open"), "{screen}");
    for text in ["Update", "Remove", "9.9.9"] {
        assert!(!screen.contains(text), "`{text}` is offered:\n{screen}");
    }
    h.press("u").send(Msg::Updates(UpdateMsg::InstallAll));
    settle(&mut h);
    let updates: Vec<String> =
        calls(root.path()).into_iter().filter(|call| call.starts_with("install --locked")).collect();
    assert_eq!(updates, ["install --locked quvyta-code --version 0.1.2"], "Install updates leaves qdesk alone");

    h.send(Msg::Select(desk())).press("enter");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, h.app().machine.cargo_bin().join("qdesk").as_os_str());
}

#[test]
fn ascii_screens_of_it_keep_the_rules() {
    for (width, height) in [(40, 16), (60, 20), (100, 24)] {
        let (_root, mut h) = harness(width, height);
        h.set_glyph_mode(GlyphMode::Ascii);
        h.send(Msg::ShowDetail(desk()));
        let screen = h.screen();
        assert!(screen.contains("Quvyta Desktop"), "{width}x{height}:\n{screen}");
        for forbidden in ['[', ']', '{', '}', '|', '▌'] {
            assert!(!screen.contains(forbidden), "`{forbidden}` at {width}x{height}:\n{screen}");
        }
    }
}
