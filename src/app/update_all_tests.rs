//! Updating many members at once, and the members quvyta must leave alone while it does: the
//! question before Install updates starts anything, and a member cargo built from a folder or a
//! git repository, which crates.io's version would overwrite.
//!
//! Every test stands in a temporary root with the stand-in cargo; nothing is installed and no
//! network is used.

use std::path::Path;

use tempfile::TempDir;

use super::install_tests::{DIALOG_IN, calls, click_beside, has, run, settle};
use super::tests::{LANGUAGES, LIST, index, machine};
use super::*;
use crate::install::tests::packages;
use crate::inventory::tests::installed_command;
use crate::updates::tests::search_scenario;

/// What crates.io answers: newer versions of qcode and qframe than the machine has.
fn search() -> String {
    format!(
        "\
quvyta-code = \"0.1.2\"                  # Runs containers
quvyta-framework-showcase = \"0.1.5\"    # Every component, live
quvyta = \"{}\"                          # This application
",
        env!("CARGO_PKG_VERSION")
    )
}

/// quvyta on the machine of [`machine`] in `root`, with cargo's record answering `list`.
fn start(root: &Path, list: &str) -> Harness<Quvyta> {
    let app = Quvyta::new(machine(root));
    std::fs::write(root.join("bin/list.out"), list).expect("list");
    search_scenario(root, &search(), 0);
    packages(root);
    let mut h = run(app, 100, 30);
    settle(&mut h);
    h
}

fn installs(root: &Path) -> Vec<String> {
    calls(root)
        .into_iter()
        .filter(|call| call.starts_with("install --locked") || call.starts_with("uninstall"))
        .collect()
}

/// The machine of [`machine`], with qcode built by cargo from a folder of its owner's.
fn with_local_qcode() -> (TempDir, Harness<Quvyta>) {
    let root = crate::app::tests::temp_root();
    let list = LIST.replace("quvyta-code v0.1.1:", "quvyta-code v0.1.1 (/home/ayse/src/code):");
    let h = start(root.path(), &list);
    (root, h)
}

#[test]
fn a_member_cargo_built_from_a_folder_is_left_to_that_folder() {
    let (root, mut h) = with_local_qcode();
    let screen = h.screen();
    println!("{screen}");
    assert!(screen.contains("1 update"), "only qframe, which came from crates.io, is counted:\n{screen}");
    // qcode is the first row, selected: it opens, and it offers neither an update nor a removal.
    assert!(screen.contains("Open"), "{screen}");
    assert!(!screen.contains("  Update") && !screen.contains("Remove"), "{screen}");
    h.press("u");
    assert!(h.app().installs.dialog.is_none(), "u asks nothing about qcode:\n{}", h.screen());

    h.click_text("Install updates").advance(DIALOG_IN);
    if has(&h, "Update") {
        click_beside(&mut h, "Cancel", "Update");
    }
    settle(&mut h);
    let ran = installs(root.path());
    assert!(!ran.iter().any(|call| call.contains("quvyta-code")), "qcode's own build is never replaced: {ran:?}");
}

#[test]
fn quvyta_built_from_a_folder_is_not_updated_from_crates_io_either() {
    let root = crate::app::tests::temp_root();
    let app = Quvyta::new(machine(root.path()));
    installed_command(&app.machine, "quvyta");
    std::fs::write(
        root.path().join("bin/list.out"),
        format!("{LIST}quvyta v0.0.1 (/home/ayse/src/quvyta):\n    quvyta\n"),
    )
    .expect("list");
    search_scenario(root.path(), &search(), 0);
    packages(root.path());
    let mut h = run(app, 100, 30);
    settle(&mut h);
    assert!(!h.app().updatable(index("quvyta")), "quvyta's own build is left to its folder:\n{}", h.screen());
}

/// The machine of [`machine`] as it is, with qcode and qframe both out of date.
fn outdated() -> (TempDir, Harness<Quvyta>) {
    let root = crate::app::tests::temp_root();
    let h = start(root.path(), LIST);
    (root, h)
}

#[test]
fn install_updates_asks_first_naming_every_command() {
    let (root, mut h) = outdated();
    assert!(h.screen().contains("2 updates"), "{}", h.screen());
    h.click_text("Install updates").advance(DIALOG_IN);
    let screen = h.screen();
    println!("{screen}");
    assert!(has(&h, "Update 2 apps?"), "a question comes first:\n{screen}");
    assert!(installs(root.path()).is_empty(), "and nothing runs before it is answered");
    for line in ["qcode from 0.1.1 to 0.1.2", "qframe from 0.1.4 to 0.1.5"] {
        assert!(screen.contains(line), "`{line}` is named:\n{screen}");
    }
    for command in ["install --locked quvyta-code --version 0.1.2", "install --locked quvyta-framework-showcase"] {
        assert!(screen.contains(command), "the command that runs is shown: `{command}`\n{screen}");
    }
    assert!(screen.contains("No sudo needed"), "{screen}");

    click_beside(&mut h, "Cancel", "Update");
    settle(&mut h);
    assert_eq!(
        installs(root.path()),
        ["install --locked quvyta-code --version 0.1.2", "install --locked quvyta-framework-showcase --version 0.1.5"],
        "agreed to, each runs as it was shown, in the order of the list"
    );
    assert!(!h.screen().contains("Install updates"), "{}", h.screen());
}

#[test]
fn cancelling_the_question_leaves_every_update_where_it_was() {
    let (root, mut h) = outdated();
    h.click_text("Install updates").advance(DIALOG_IN);
    assert!(has(&h, "Update 2 apps?"), "{}", h.screen());
    h.click_text("Cancel");
    settle(&mut h);
    assert!(h.app().installs.dialog.is_none(), "{}", h.screen());
    assert!(installs(root.path()).is_empty(), "nothing ran");
    assert!(h.screen().contains("2 updates"), "both still wait:\n{}", h.screen());
}

#[test]
fn install_updates_is_held_by_what_the_checks_find() {
    let (root, mut h) = outdated();
    // No C linker any more: every build would fail minutes in, so nothing may start.
    std::fs::remove_file(root.path().join("bin/cc")).expect("no linker");
    h.click_text("Install updates").advance(DIALOG_IN);
    let screen = h.screen();
    println!("{screen}");
    assert!(screen.contains("none was found"), "what is in the way is said:\n{screen}");
    assert!(
        !has(&h, "Cancel  Update") && screen.contains("Check again"),
        "and there is nothing to agree to:\n{screen}"
    );
    h.press("enter");
    settle(&mut h);
    assert!(installs(root.path()).is_empty(), "nothing ran: {:?}", installs(root.path()));
}

#[test]
fn one_update_is_asked_about_like_any_single_update() {
    let root = crate::app::tests::temp_root();
    let list = LIST.replace("quvyta-framework-showcase v0.1.4:", "quvyta-framework-showcase v0.1.5:");
    let mut h = start(root.path(), &list);
    assert!(h.screen().contains("1 update"), "{}", h.screen());
    h.click_text("Install updates").advance(DIALOG_IN);
    assert!(has(&h, "Update qcode from 0.1.1 to 0.1.2?"), "{}", h.screen());
    assert!(installs(root.path()).is_empty());
}

#[test]
fn the_question_reads_whole_in_every_language_and_on_narrow_screens() {
    for locale in LANGUAGES {
        for width in [40, 60, 100] {
            let root = crate::app::tests::temp_root();
            let app = Quvyta::new(machine(root.path()));
            search_scenario(root.path(), &search(), 0);
            packages(root.path());
            let mut h = run(app, width, 40);
            h.set_locale(locale);
            settle(&mut h);
            let install = h.env().i18n().translate("updates.install-all", &[]);
            h.click_text(&install).advance(DIALOG_IN);
            let say = |key: &str, args: &[(&str, &str)]| {
                let args: Vec<(&str, qframe::i18n::Arg)> =
                    args.iter().map(|(name, value)| (*name, qframe::i18n::Arg::from(*value))).collect();
                h.env().i18n().translate(key, &args)
            };
            let title = h.env().i18n().translate("confirm.update-all-title", &[("n", qframe::i18n::Arg::from(2))]);
            let lines = [
                say("confirm.update-line", &[("command", "qcode"), ("from", "0.1.1"), ("to", "0.1.2")]),
                say("confirm.update-line", &[("command", "qframe"), ("from", "0.1.4"), ("to", "0.1.5")]),
            ];
            let screen = h.screen();
            // The dialog's pillar stands at the start of every line a wrapped text goes on to.
            let squeezed: String = screen.replace('\u{258c}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
            for text in std::iter::once(&title).chain(lines.iter()) {
                let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
                assert!(squeezed.contains(&text), "`{text}` whole in {locale} at {width}:\n{screen}");
            }
            assert!(installs(root.path()).is_empty(), "{locale} at {width}: asking installs nothing");
        }
    }
}
