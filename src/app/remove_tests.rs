//! Removing an app: the question naming what goes and what stays, `cargo uninstall` through
//! the stand-in cargo, the queue it shares with installs, failures, and the apps that have
//! no Remove.

use std::path::Path;

use qframe::icons::GlyphMode;
use tempfile::TempDir;

use super::install_tests::{DIALOG_IN, ask_remove, calls, click_beside, has, run, said, settle};
use super::installs::InstallMsg;
use super::tests::{LANGUAGES, TOAST_IN, harness, index, line_with, machine};
use super::*;
use crate::install::tests::packages;

fn install(msg: InstallMsg) -> Msg {
    Msg::Install(msg)
}

/// The application on the machine of [`machine`], with qcode selected and the stand-in cargo
/// knowing every package, so a removal does what cargo would.
fn removing() -> (TempDir, Harness<Quvyta>) {
    let (root, mut h) = harness(100, 30);
    packages(root.path());
    h.send(Msg::Select(index("code")));
    (root, h)
}

fn uninstalls(root: &Path) -> Vec<String> {
    calls(root).into_iter().filter(|call| call.starts_with("uninstall")).collect()
}

/// Presses Remove in the question, in the language the screen speaks.
fn confirm(h: &mut Harness<Quvyta>) {
    let cancel = said(h, "confirm.cancel");
    let remove = said(h, "remove.confirm");
    click_beside(h, &cancel, &remove);
}

#[test]
fn remove_asks_first_saying_what_goes_and_that_the_settings_stay() {
    let (root, mut h) = removing();
    assert!(line_with(&h.screen(), "Open").contains("Remove"), "{}", h.screen());
    h.click_text("Remove").advance(DIALOG_IN);
    let screen = h.screen();
    let cargo = root.path().join("bin/cargo").display().to_string();
    let settings = root.path().join("config").display().to_string();
    for text in [
        "Remove qcode?",
        "The program ~/.cargo/bin/qcode is deleted",
        "Its settings in",
        "Command",
        &format!("{cargo} uninstall quvyta-code"),
        "Cancel",
    ] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    // The folder may wrap; its last part is enough to see it named.
    let tail = settings.rsplit('/').next().expect("a name");
    assert!(screen.contains(tail), "the settings folder is named:\n{screen}");
    assert!(uninstalls(root.path()).is_empty(), "nothing ran");
    for close in ["esc", "Cancel", "×"] {
        if close == "esc" {
            h.press("esc");
        } else {
            h.click_text("Remove").advance(DIALOG_IN).click_text(close);
        }
        assert!(!has(&h, "Remove qcode?"), "{close}:\n{}", h.screen());
    }
    assert!(uninstalls(root.path()).is_empty(), "and nothing runs");
}

#[test]
fn the_question_is_drawn_as_a_danger() {
    let (_root, mut h) = removing();
    h.set_theme("nordic").click_text("Remove").advance(DIALOG_IN);
    let (x, y) = h.find("Remove qcode?").expect("the title");
    let pillar = h.fg(u16::try_from(x).expect("x").saturating_sub(3), u16::try_from(y).expect("y"));
    assert_eq!(pillar, h.env().theme().color("danger"), "{}", h.screen());
}

#[test]
fn a_removal_runs_cargo_uninstall_and_keeps_the_settings() {
    let (root, mut h) = removing();
    let settings = root.path().join("config/code.conf");
    std::fs::create_dir_all(settings.parent().expect("folder")).expect("folder");
    std::fs::write(&settings, "theme = \"nordic\"\n").expect("settings");
    h.click_text("Remove");
    confirm(&mut h);
    settle(&mut h);
    assert_eq!(uninstalls(root.path()), ["uninstall quvyta-code"]);
    let screen = h.advance(TOAST_IN).screen();
    assert!(screen.contains("qcode removed"), "{screen}");
    assert!(line_with(&screen, "qcode ").contains("not installed"), "the list was read again:\n{screen}");
    assert!(screen.contains("Install"), "the main button is Install again:\n{screen}");
    assert!(!h.app().machine.cargo_bin().join("qcode").exists());
    assert_eq!(std::fs::read_to_string(&settings).expect("kept"), "theme = \"nordic\"\n", "settings are the user's");
    assert!(!root.path().join("data/build").exists(), "no build folder is left");
}

#[test]
fn a_failed_removal_shows_cargo_s_lines_and_can_be_tried_again() {
    let (root, mut h) = removing();
    std::fs::write(root.path().join("bin/uninstall.err"), "error: failed to remove file `qcode`: Permission denied\n")
        .expect("scenario");
    std::fs::write(root.path().join("bin/uninstall.code"), "101").expect("scenario");
    h.click_text("Remove");
    confirm(&mut h);
    settle(&mut h);
    let screen = h.screen();
    for text in [
        "qcode could not be removed",
        "cargo could not remove it; its last lines say why.",
        "Last lines",
        "error: failed to remove file `qcode`: Permission denied",
        "Copy log",
        "Close",
        "Retry",
    ] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    for text in ["Nothing was installed", "The whole log is in"] {
        assert!(!screen.contains(text), "`{text}` belongs to installs:\n{screen}");
    }
    assert!(line_with(&screen, "qcode ").contains("not removed"), "{screen}");
    std::fs::remove_file(root.path().join("bin/uninstall.code")).expect("scenario");
    std::fs::remove_file(root.path().join("bin/uninstall.err")).expect("scenario");
    h.click_text("Retry");
    settle(&mut h);
    assert_eq!(uninstalls(root.path()), ["uninstall quvyta-code", "uninstall quvyta-code"]);
    assert!(line_with(&h.screen(), "qcode ").contains("not installed"), "{}", h.screen());
}

#[test]
fn only_apps_cargo_installed_can_be_removed_and_never_quvyta() {
    let (root, mut h) = removing();
    for key in ["focus", "tools", "quvyta"] {
        h.send(Msg::Select(index(key)));
        assert!(!h.screen().contains("Remove"), "{key}:\n{}", h.screen());
        // The Remove button is the only way a person can ask, and for these three it is not drawn,
        // so this message stands in for the click nobody can make: it is refused all the same.
        h.send(install(InstallMsg::AskRemove(index(key))));
        assert_eq!(h.app().installs.remove, None, "{key}");
    }
    assert!(uninstalls(root.path()).is_empty());
}

#[test]
fn quvyta_installed_by_cargo_still_does_not_remove_itself() {
    let root = tempfile::tempdir().expect("temp");
    let app = Quvyta::new(machine(root.path()));
    let list = format!("{}quvyta v0.1.1:\n    quvyta\n", super::tests::LIST);
    std::fs::write(root.path().join("bin/list.out"), list).expect("list");
    crate::inventory::tests::installed_command(&app.machine, "quvyta");
    let mut h = run(app, 100, 30);
    h.send(Msg::Select(index("quvyta")));
    assert!(!has(&h, "Remove"), "{}", h.screen());
    // There is no Remove button for quvyta to click, so this message stands in for the click
    // nobody can make.
    h.send(install(InstallMsg::AskRemove(index("quvyta"))));
    assert!(!has(&h, "Remove"), "{}", h.screen());
}

/// qtools installing, its task never run, and qcode's removal asked for behind it.
fn behind_an_install() -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    let mut app = Quvyta::new(machine(root.path()));
    app.inventory = Some(Inventory::read(&app.machine));
    let tools = index("tools");
    // The install and the removal behind it, each opened and agreed to on its button, sent here
    // because the app is built before any harness runs it. What the answers return is dropped on
    // purpose: one cargo at a time is the screen, and a harness would run both to their end.
    let _ = app.update(install(InstallMsg::Ask(tools)));
    // The checks' own answer, which is no person's to give: a background task's.
    let _ = app.update(install(InstallMsg::Checked {
        index: tools,
        problems: Vec::new(),
        other_window: false,
        named: None,
    }));
    let _ = app.update(install(InstallMsg::Confirm));
    let _ = app.update(install(InstallMsg::AskRemove(index("code"))));
    let _ = app.update(install(InstallMsg::ConfirmRemove));
    let h = run(app, 100, 30);
    (root, h)
}

#[test]
fn a_removal_waits_for_the_install_before_it() {
    let (root, mut h) = behind_an_install();
    let screen = h.screen();
    assert!(line_with(&screen, "qtools ").contains("installing"), "{screen}");
    assert!(line_with(&screen, "qcode ").contains("queued"), "{screen}");
    h.send(Msg::Select(index("code")));
    assert!(has(&h, "It starts when qtools is done."), "{}", h.screen());
    assert!(uninstalls(root.path()).is_empty(), "one cargo job at a time");
}

#[test]
fn a_running_removal_says_so_in_the_row_and_the_details() {
    let root = tempfile::tempdir().expect("temp");
    let mut app = Quvyta::new(machine(root.path()));
    app.inventory = Some(Inventory::read(&app.machine));
    // The removal opened and agreed to on its two buttons, sent here because the app is built
    // before any harness runs it; what the answer returns is dropped, so cargo is still working.
    let _ = app.update(install(InstallMsg::AskRemove(index("code"))));
    let _ = app.update(install(InstallMsg::ConfirmRemove));
    let h = run(app, 100, 30);
    let screen = h.screen();
    assert!(line_with(&screen, "qcode ").contains("removing"), "{screen}");
    assert!(screen.contains("qcode removing") && !screen.contains("Stop"), "{screen}");
}

#[test]
fn turkish_removal_reads_naturally() {
    let (root, mut h) = removing();
    h.set_locale("tr").click_text("Kaldır").advance(DIALOG_IN);
    let screen = h.screen();
    for text in ["qcode kaldırılsın mı?", "programı ve cargo'nun ona dair kaydı silinir", "Ayarları", "Vazgeç"]
    {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    confirm(&mut h);
    settle(&mut h);
    assert!(h.advance(TOAST_IN).screen().contains("qcode kaldırıldı"), "{}", h.screen());
    assert_eq!(uninstalls(root.path()), ["uninstall quvyta-code"]);
}

#[test]
fn removal_screens_keep_the_rules_in_ascii_and_on_narrow_screens() {
    for (width, height) in [(40, 30), (48, 30), (60, 30), (100, 30)] {
        for locale in LANGUAGES {
            let (root, mut h) = harness(width, height);
            h.set_glyph_mode(GlyphMode::Ascii).set_locale(locale);
            h.send(Msg::ShowDetail(index("code")));
            let mut screens = vec![h.screen()];
            ask_remove(&mut h, index("code"));
            screens.push(h.screen());
            std::fs::write(root.path().join("bin/uninstall.code"), "101").expect("scenario");
            confirm(&mut h);
            settle(&mut h);
            h.send(Msg::ShowDetail(index("code")));
            screens.push(h.screen());
            for screen in screens {
                for forbidden in ['[', ']', '{', '}', '|', '▌', '⟦'] {
                    assert!(!screen.contains(forbidden), "`{forbidden}` at {width}x{height}:\n{screen}");
                }
            }
        }
    }
}

/// With `QUVYTA_REVIEW=1`, writes the screens of removing in both languages, wide and narrow,
/// to `target/quvyta-remove-review.html` in colour for a visual review.
///
/// Ignored like [`visual_review`](super::tests::visual_review): a review writes a file for a
/// person to look at, and a run of the gate must read it as skipped and not as passed.
#[test]
#[ignore = "visual review: QUVYTA_REVIEW=1 cargo test visual_review -- --ignored"]
fn visual_review_removal() {
    if std::env::var_os("QUVYTA_REVIEW").is_none() {
        return;
    }
    let mut fragments = Vec::new();
    let mut shot = |h: &Harness<Quvyta>, caption: String| {
        println!("{caption}\n{}", h.screen());
        fragments.push(h.html(&caption));
    };
    for (width, height) in [(100, 26), (48, 30)] {
        for locale in ["en", "tr"] {
            let size = format!("{locale} {width}x{height}");
            let (root, mut h) = harness(width, height);
            h.set_locale(locale).send(Msg::ShowDetail(index("code")));
            shot(&h, format!("an app cargo installed {size}"));
            ask_remove(&mut h, index("code"));
            shot(&h, format!("remove {size}"));
            std::fs::write(root.path().join("bin/uninstall.err"), "error: failed to remove file `qcode`\n")
                .expect("scenario");
            std::fs::write(root.path().join("bin/uninstall.code"), "101").expect("scenario");
            confirm(&mut h);
            settle(&mut h);
            h.send(Msg::ShowDetail(index("code")));
            shot(&h, format!("removal failed {size}"));
        }
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/quvyta-remove-review.html");
    std::fs::write(path, qframe::runtime::html_page(&fragments)).expect("review page written");
}
