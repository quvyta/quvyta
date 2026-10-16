//! The line under the list: the Quvyta apps this machine does not have, counted, and offered with
//! a button and a key that ask for them one after another.
//!
//! Every test stands in a temporary root with the stand-in cargo of `tests/fake-cargo`: nothing is
//! installed, nothing reaches a desktop and nothing is asked of the real machine.

use std::path::Path;

use tempfile::TempDir;

use super::cli_tests::installs_ran;
use super::install_tests::{DIALOG_IN, calls, click_beside, confirm, has, run, settle};
use super::tests::{LANGUAGES, harness, index, line_with, machine, temp_root};
use super::*;
use crate::install::tests::packages;
use crate::inventory::tests::installed_command;
use crate::updates::tests::search_scenario;

/// The machine of the list's tests in `root`, with `keys` in cargo's record as well, as
/// `cargo install` would leave them: their rows then say a version, and they are not missing.
fn machine_with(root: &Path, keys: &[&str]) -> Machine {
    let machine = machine(root);
    let mut list = std::fs::read_to_string(root.join("bin/list.out")).expect("the machine's own list");
    for key in keys {
        let app = &APPS[index(key)];
        installed_command(&machine, app.command);
        list.push_str(&format!("{} v0.1.0:\n    {}\n    {}\n", app.package, app.command, app.package));
    }
    std::fs::write(root.join("bin/list.out"), list).expect("scenario");
    machine
}

/// [`machine_with`] with `keys` installed, `width` by `height`. The folder lives as long as the
/// returned guard.
fn with_installed(keys: &[&str], width: u16, height: u16) -> (TempDir, Harness<Quvyta>) {
    let root = temp_root();
    let machine = machine_with(root.path(), keys);
    (root, run(Quvyta::new(machine), width, height))
}

/// [`with_installed`] with crates.io answering [`search`], so the update line has something to say
/// as well, and the stand-in cargo knowing what each package installs.
fn updating(keys: &[&str], width: u16, height: u16) -> (TempDir, Harness<Quvyta>) {
    let root = temp_root();
    let machine = machine_with(root.path(), keys);
    search_scenario(root.path(), &search(), 0);
    packages(root.path());
    let mut h = run(Quvyta::new(machine), width, height);
    settle(&mut h);
    (root, h)
}

/// The application on a machine whose `os-release` names `distro`, which is what tells the apps
/// that run on Arch Linux only from the rest. It is written before the machine is built, since
/// that is when quvyta reads it.
fn on(distro: &str, width: u16, height: u16) -> (TempDir, Harness<Quvyta>) {
    let root = temp_root();
    std::fs::create_dir_all(root.path().join("etc")).expect("folder");
    std::fs::write(root.path().join("etc/os-release"), distro).expect("os-release");
    let machine = machine(root.path());
    (root, run(Quvyta::new(machine), width, height))
}

/// What crates.io answers: qcode has a newer version than the machine of the list's tests has,
/// and nothing else is out of date.
fn search() -> String {
    format!("quvyta-code = \"0.1.2\"\nquvyta = \"{}\"\n", env!("CARGO_PKG_VERSION"))
}

/// The screen row `text` is on, counting from the top.
fn row_of(screen: &str, text: &str) -> usize {
    screen.lines().position(|row| row.contains(text)).unwrap_or_else(|| panic!("`{text}` is not on screen:\n{screen}"))
}

/// How many apps the list has on the screen, which is how much of the list's height is left.
fn listed(screen: &str) -> usize {
    APPS.iter()
        .filter(|app| screen.lines().any(|row| row.split_whitespace().take(3).any(|word| word == app.command)))
        .count()
}

#[test]
fn the_line_under_the_list_counts_what_is_not_installed_and_offers_to_install_it() {
    let (_root, h) = harness(100, 24);
    let screen = h.screen();
    // The machine of the list's tests has qcode and qframe from cargo and qfocus from elsewhere,
    // on Arch Linux: the six apps the rest of the list names are the ones missing.
    assert!(screen.contains("6 apps are not installed"), "{screen}");
    assert!(screen.contains("Install them"), "the button is on the screen:\n{screen}");
    assert!(
        row_of(&screen, "6 apps are not installed") > row_of(&screen, "qframe"),
        "the line is under the list:\n{screen}"
    );
    // The count is said quietly, so what the eye lands on is the button.
    let (x, y) = h.find("6 apps").expect("the count");
    let count = (u16::try_from(x).expect("x"), u16::try_from(y).expect("y"));
    let (qx, qy) = h.find("qcode").expect("a row");
    assert_ne!(
        h.fg(count.0, count.1),
        h.fg(u16::try_from(qx).expect("x"), u16::try_from(qy).expect("y")),
        "the count is faint, not like a row:\n{screen}"
    );
}

#[test]
fn installing_them_asks_for_each_one_in_turn_and_installs_nothing_unasked() {
    let (root, mut h) = harness(100, 24);
    h.click_text("Install them").advance(DIALOG_IN);
    let screen = h.screen();
    // The first app of the list that is missing, which is what the run of questions is filled with.
    assert!(has(&h, "Install qtools?"), "{screen}");
    let cargo = root.path().join("bin/cargo").display().to_string();
    assert!(
        screen.contains(&format!("{cargo} install --locked quvyta-tools")),
        "the exact cargo command is on it:\n{screen}"
    );
    assert_eq!(installs_ran(root.path()), 0, "the question installs nothing");

    confirm(&mut h);
    h.advance(DIALOG_IN);
    assert!(has(&h, "Install qpac?"), "the next app's question follows:\n{}", h.screen());
    confirm(&mut h);
    h.advance(DIALOG_IN);
    assert!(has(&h, "Install qexp?"), "and the next, in the order of the list:\n{}", h.screen());
    // Cancelling one question moves on to the next, the way the wizard's own queue does.
    for asked in ["Install qbrow?", "Install qcli?", "Install qdesk?"] {
        h.press("esc").advance(DIALOG_IN);
        assert!(has(&h, asked), "`{asked}` follows the one before it:\n{}", h.screen());
    }
    h.press("esc").advance(DIALOG_IN);
    settle(&mut h);
    assert!(!has(&h, "Install q"), "and then the run of questions is over:\n{}", h.screen());
    // The order the first-run wizard leaves its checked apps in, and nothing installed unasked.
    let asked: Vec<String> =
        calls(root.path()).into_iter().filter(|call| call.starts_with("install --locked")).collect();
    assert_eq!(asked, ["install --locked quvyta-tools", "install --locked quvyta-packages"], "{asked:?}");
}

#[test]
fn the_button_and_the_key_do_the_same_thing() {
    let (_clicked, mut by_click) = harness(100, 24);
    by_click.click_text("Install them").advance(DIALOG_IN);
    let (_pressed, mut by_key) = harness(100, 24);
    by_key.press("i").advance(DIALOG_IN);
    assert!(has(&by_click, "Install qtools?"), "the button:\n{}", by_click.screen());
    assert!(has(&by_key, "Install qtools?"), "the key:\n{}", by_key.screen());
    assert_eq!(by_click.app().selected, by_key.app().selected, "both open the same app's question");
    assert_eq!(by_click.app().asked, by_key.app().asked, "and leave the same run of questions behind it");
    assert_eq!(by_click.app().installs.dialog, by_key.app().installs.dialog, "and install nothing by either");
    assert!(has(&by_key, "i  missing"), "the key is in the hint bar:\n{}", by_key.screen());
}

#[test]
fn with_everything_installed_the_line_is_not_drawn_and_the_list_keeps_its_room() {
    // A screen just tall enough for the ten apps of the list and the update line, so the list is
    // what the body is short of: every app that can be installed here in cargo's record, and an
    // update waiting.
    let (root, mut h) = updating(&["tools", "packages", "explorer", "browser", "cli", "desk"], 100, 15);
    let screen = h.screen();
    for text in ["apps are not installed", "app is not installed", "Install them"] {
        assert!(!screen.contains(text), "`{text}` is drawn although nothing is missing:\n{screen}");
    }
    // The list kept its height: a row left empty for the line would have cut the tenth app off.
    assert_eq!(listed(&screen), APPS.len(), "every app is on the screen:\n{screen}");
    assert!(
        row_of(&screen, "1 update") == screen.lines().count() - 2,
        "the update line is on the last row of the body:\n{screen}"
    );

    // The same machine with two apps still missing, where the line does take its row: the list is
    // that much shorter, which is what the line above the update line costs.
    let (_root, missing) = updating(&["explorer", "browser"], 100, 15);
    let screen = missing.screen();
    assert!(
        row_of(&screen, "4 apps are not installed") < row_of(&screen, "1 update"),
        "the line is above the update line:\n{screen}"
    );
    assert!(listed(&screen) < APPS.len(), "and the list is that much shorter:\n{screen}");

    // An app this line would have counted is installed now, so its row opens it and asks for
    // nothing: there is no app left to count.
    h.send(Msg::Select(index("explorer"))).press("enter");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(
        request.program,
        h.app().machine.cargo_bin().join("qexp").as_os_str(),
        "the row opens the app the line no longer counts:\n{}",
        h.screen()
    );
    assert!(h.app().installs.dialog.is_none(), "and nothing is asked:\n{}", h.screen());
    assert_eq!(installs_ran(root.path()), 0, "nothing was installed");
}

#[test]
fn on_a_very_short_screen_the_line_leaves_its_rows_to_the_list() {
    // Six apps missing and two updates waiting, eight rows: with the line drawn, the list had no
    // row left at all.
    let (_root, short) = updating(&[], 40, 8);
    let screen = short.screen();
    assert!(!screen.contains("Install them"), "the line gives way:\n{screen}");
    assert!(listed(&screen) > 0, "and the list has rows again:\n{screen}");
    assert!(screen.contains("Install updates"), "the update line is still there:\n{screen}");
    // At twelve rows the line is back.
    let (_root, tall) = updating(&[], 40, 12);
    assert!(tall.screen().contains("Install them"), "{}", tall.screen());
}

#[test]
fn an_app_that_cannot_run_here_is_not_counted() {
    // qtools and qpac run on Arch Linux only. The list says so where they cannot run, and the
    // count leaves them out; the very same folder named Arch Linux counts them.
    let (_root, h) = on("ID=debian\n", 100, 24);
    let screen = h.screen();
    for command in ["qtools", "qpac", "qexp", "qbrow", "qcli", "qdesk"] {
        assert!(
            line_with(&screen, &format!("{command} ")).contains(if command == "qtools" || command == "qpac" {
                "Arch Linux only"
            } else {
                "not installed"
            }),
            "the row says what is true of `{command}`:\n{screen}"
        );
    }
    assert!(screen.contains("4 apps are not installed"), "four of the six can be installed here:\n{screen}");

    let (_arch, arch) = on("ID=arch\n", 100, 24);
    let screen = arch.screen();
    for command in ["qtools", "qpac", "qexp", "qbrow", "qcli", "qdesk"] {
        assert!(
            line_with(&screen, &format!("{command} ")).contains("not installed"),
            "`{command}` is missing:\n{screen}"
        );
    }
    assert!(screen.contains("6 apps are not installed"), "and on Arch two more can be installed:\n{screen}");
}

#[test]
fn one_left_reads_in_the_singular() {
    let (_root, mut h) = with_installed(&["tools", "packages", "explorer", "browser", "cli"], 100, 24);
    assert!(h.screen().contains("1 app is not installed"), "only qdesk is left:\n{}", h.screen());
    h.set_locale("tr");
    assert!(h.screen().contains("1 uygulama kurulu değil"), "{}", h.screen());
}

#[test]
fn a_narrow_terminal_puts_the_button_under_the_count() {
    // A list with the whole screen to itself: the count and the button fit beside each other, and
    // finding each of them whole is what says that neither is cut.
    let (_root, h) = harness(48, 24);
    let screen = h.screen();
    assert_eq!(
        row_of(&screen, "6 apps are not installed"),
        row_of(&screen, "Install them"),
        "beside each other:\n{screen}"
    );

    // Forty columns: they do not fit, so the button goes on a line of its own under the count.
    let (_root, narrow) = harness(40, 24);
    let screen = narrow.screen();
    let count = row_of(&screen, "6 apps are not installed");
    let button = row_of(&screen, "Install them");
    assert!(button > count, "the button under the count:\n{screen}");
    assert!(
        !screen.lines().nth(count).expect("the count's row").contains("Install them"),
        "and not beside it:\n{screen}"
    );
    for line in screen.lines().filter(|line| line.contains("Install them") || line.contains("apps are not installed")) {
        assert!(!line.contains('…'), "nothing is cut: `{}`", line.trim_end());
    }
}

#[test]
fn an_app_that_is_installed_somewhere_else_is_not_missing() {
    let (root, mut h) = harness(100, 24);
    // qfocus is on PATH from outside cargo, so its row says its version is unknown and the count
    // leaves it out: the six the line names are the ones still to be installed.
    assert!(h.screen().contains("6 apps are not installed"), "{}", h.screen());
    h.send(Msg::Select(index("focus"))).press("enter");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, root.path().join("bin/qfocus").as_os_str(), "its row still opens it");
}

#[test]
fn the_line_and_the_update_line_read_together() {
    let (root, mut h) = updating(&[], 100, 24);
    let screen = h.screen();
    assert!(
        row_of(&screen, "6 apps are not installed") < row_of(&screen, "1 update"),
        "the line of what is missing is above the update line:\n{screen}"
    );
    assert!(
        screen.contains("Install them") && screen.contains("Install updates"),
        "each has a button of its own:\n{screen}"
    );

    h.click_text("Install updates").advance(DIALOG_IN);
    click_beside(&mut h, "Cancel", "Update");
    settle(&mut h);
    let calls = calls(root.path());
    assert!(
        calls.contains(&"install --locked quvyta-code --version 0.1.2".to_owned()),
        "the update button updates: {calls:?}"
    );
    h.click_text("Install them").advance(DIALOG_IN);
    assert!(has(&h, "Install qtools?"), "and the line above asks for the missing ones:\n{}", h.screen());
    assert_eq!(installs_ran(root.path()), 1, "the question itself installed nothing: {calls:?}");
}

#[test]
fn turkish_reads_naturally() {
    let (root, mut h) = harness(100, 24);
    h.set_locale("tr");
    let screen = h.screen();
    for text in ["6 uygulama kurulu değil", "Hepsini kur", "i  eksik"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    h.press("i").advance(DIALOG_IN);
    let screen = h.screen();
    assert!(has(&h, "qtools kurulsun mu?"), "{screen}");
    let cargo = root.path().join("bin/cargo").display().to_string();
    assert!(
        screen.contains(&format!("{cargo} install --locked quvyta-tools")),
        "the same command in Turkish:\n{screen}"
    );
    assert_eq!(installs_ran(root.path()), 0, "the question installs nothing");
    h.press("esc").advance(DIALOG_IN);
    assert!(has(&h, "qpac kurulsun mu?"), "the next app asks in the same words:\n{}", h.screen());
}

/// The count and the button in every language quvyta speaks and at the widths the line is drawn
/// at, each looked for as itself with its spaces left out: what matters is that every character
/// of the two reached the screen, whether they sit beside each other or one under the other.
#[test]
fn the_line_under_the_list_reads_whole_in_every_language_and_at_every_width() {
    for locale in LANGUAGES {
        for width in [40, 48, 60, 100] {
            let (_root, mut h) = harness(width, 24);
            h.set_locale(locale);
            let screen = h.screen();
            let say = |key: &str| match key {
                "missing.count" => h.env().i18n().translate(key, &[("n", qframe::i18n::Arg::from(6))]),
                _ => h.env().i18n().translate(key, &[]),
            };
            let flat: String = screen.chars().filter(|ch| !ch.is_whitespace()).collect();
            for key in ["missing.count", "missing.install"] {
                let label: String = say(key).chars().filter(|ch| !ch.is_whitespace()).collect();
                assert!(flat.contains(&label), "`{label}` is not whole in {locale} at {width} columns:\n{screen}");
            }
        }
    }
}

/// With `QUVYTA_REVIEW=1`, writes the line under the list in both languages, wide and narrow, to
/// `target/quvyta-missing-review.html` in colour for a visual review.
///
/// Ignored like [`visual_review`](super::tests::visual_review): a review writes a file for a
/// person to look at, and a run of the gate must read it as skipped and not as passed.
#[test]
#[ignore = "visual review: QUVYTA_REVIEW=1 cargo test visual_review -- --ignored"]
fn visual_review_missing() {
    if std::env::var_os("QUVYTA_REVIEW").is_none() {
        return;
    }
    let mut fragments = Vec::new();
    for (width, height) in [(100, 26), (48, 26), (40, 26)] {
        for locale in ["en", "tr"] {
            let (_root, mut h) = harness(width, height);
            h.set_locale(locale);
            let caption = format!("missing {locale} {width}x{height}");
            println!("{caption}\n{}", h.screen());
            fragments.push(h.html(&caption));
        }
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/quvyta-missing-review.html");
    std::fs::write(path, qframe::runtime::html_page(&fragments)).expect("review page written");
}
