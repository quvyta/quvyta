//! Updates on the screen: the newer version in the row, the count and Install updates under
//! the list, the update dialog, asking again with `r`, a failed check, turning the check off,
//! and quvyta updating itself. crates.io is the stand-in cargo's `search.out`.

use std::path::Path;

use qframe::icons::GlyphMode;
use tempfile::TempDir;

use super::install_tests::{DIALOG_IN, calls, click_beside, has, run, settle};
use super::installs::Action;
use super::tests::{LANGUAGES, LIST, TOAST_IN, harness_with_settings, index, line_with, machine};
use super::*;
use crate::install::tests::packages;
use crate::inventory::tests::installed_command;
use crate::updates::tests::search_scenario;

/// What crates.io answers: qcode and qframe have newer versions than the machine of
/// [`machine`] has, qfocus (installed elsewhere) too, and quvyta has the running version.
fn search() -> String {
    format!(
        "\
quvyta-code = \"0.1.2\"                  # Runs containers
quvyta-focus = \"0.1.2\"                 # Focus tracking in the terminal
quvyta-tools = \"0.1.2\"                 # Applies the recommended Arch Linux settings
quvyta-framework-showcase = \"0.1.5\"    # Every component, live
quvyta-framework = \"0.1.5\"             # A Rust framework
quvyta = \"{}\"                          # This application
",
        env!("CARGO_PKG_VERSION")
    )
}

/// The application on the machine of [`machine`] in `root`, crates.io answering `search`.
fn start_with(root: &Path, search: &str, code: i32, width: u16, height: u16) -> Harness<Quvyta> {
    let app = Quvyta::new(machine(root));
    search_scenario(root, search, code);
    packages(root);
    let mut h = run(app, width, height);
    settle(&mut h);
    h
}

fn updating() -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    let h = start_with(root.path(), &search(), 0, 100, 30);
    (root, h)
}

fn searches(root: &Path) -> usize {
    calls(root).iter().filter(|call| call.as_str() == "search quvyta --limit 20").count()
}

/// The list row of `command`: one of its first words is the command, which the lines of the
/// details beside the list never start with.
fn row<'a>(screen: &'a str, command: &str) -> &'a str {
    screen
        .lines()
        .skip(1)
        .find(|line| line.split_whitespace().take(3).any(|word| word == command))
        .unwrap_or_else(|| panic!("no row of `{command}`:\n{screen}"))
}

fn installs(root: &Path) -> Vec<String> {
    calls(root).into_iter().filter(|call| call.starts_with("install --locked")).collect()
}

#[test]
fn a_row_with_an_update_names_the_new_version_and_the_list_counts_them() {
    let (_root, h) = updating();
    let screen = h.screen();
    assert!(row(&screen, "qcode").contains("0.1.1  new 0.1.2"), "{screen}");
    assert!(row(&screen, "qframe").contains("0.1.4  new 0.1.5"), "{screen}");
    assert!(!row(&screen, "qfocus").contains("new"), "installed elsewhere, left to that:\n{screen}");
    assert!(!row(&screen, "qtools").contains("new"), "{screen}");
    assert!(!row(&screen, "quvyta").contains("new"), "{screen}");
    assert!(screen.contains("2 updates") && screen.contains("Install updates"), "{screen}");
    assert!(!screen.contains("Could not check"), "{screen}");
}

#[test]
fn the_details_show_the_new_version_in_the_accent_colour_and_an_update_button() {
    let (_root, mut h) = updating();
    // The monochrome accent is the colour of text; nordic's tells them apart.
    h.set_theme("nordic");
    let screen = h.screen();
    assert!(screen.contains("Installed  0.1.1, ~/.cargo/bin/qcode  new 0.1.2"), "{screen}");
    assert!(line_with(&screen, "Open").contains("Update"), "Update sits beside Open:\n{screen}");
    let row = screen.lines().position(|line| line.contains("Installed  0.1.1")).expect("the installed line");
    let line = screen.lines().nth(row).expect("line");
    let column = line[..line.rfind("new 0.1.2").expect("in the details")].chars().count();
    let accent = h.env().theme().color("accent");
    let at = (u16::try_from(column).expect("x"), u16::try_from(row).expect("y"));
    assert_eq!(h.fg(at.0, at.1), accent, "{screen}");
    assert_ne!(h.fg(at.0 - 3, at.1), accent, "only the new version is in the accent colour");
}

#[test]
fn up_to_date_says_nothing() {
    let root = tempfile::tempdir().expect("temp");
    let current = "quvyta-code = \"0.1.1\"\nquvyta-framework-showcase = \"0.1.4\"\nquvyta-tools = \"0.1.2\"\n";
    let h = start_with(root.path(), current, 0, 100, 30);
    let screen = h.screen();
    for text in ["new ", "1 update", "2 updates", "Install updates", "Update", "Could not check"] {
        assert!(!screen.contains(text), "`{text}` while everything is up to date:\n{screen}");
    }
    assert_eq!(searches(root.path()), 1);
}

#[test]
fn a_failed_check_says_so_faintly_and_keeps_the_last_answer() {
    let root = tempfile::tempdir().expect("temp");
    let h = start_with(root.path(), "", 101, 100, 30);
    let screen = h.screen();
    assert!(screen.contains("Could not check for updates"), "{screen}");
    assert!(!screen.contains("new "), "{screen}");
    let (x, y) = h.find("Could not check").expect("the line");
    let faint = h.fg(u16::try_from(x).expect("x"), u16::try_from(y).expect("y"));
    let (qx, qy) = h.find("qtools").expect("a row");
    assert_ne!(faint, h.fg(u16::try_from(qx).expect("x"), u16::try_from(qy).expect("y")), "faint, not like a row");

    // An answer from yesterday stands in when crates.io cannot be reached.
    let root = tempfile::tempdir().expect("temp");
    let yesterday = crate::updates::Latest::from_search(&search(), crate::updates::now() - 24 * 60 * 60);
    yesterday.write(&root.path().join("data/latest.toml")).expect("kept");
    let h = start_with(root.path(), "", 101, 100, 30);
    let screen = h.screen();
    assert!(screen.contains("Could not check for updates"), "{screen}");
    assert!(row(&screen, "qcode").contains("new 0.1.2"), "the last answer is used:\n{screen}");
}

#[test]
fn a_fresh_answer_is_not_asked_again_at_start() {
    let root = tempfile::tempdir().expect("temp");
    let fresh = crate::updates::Latest::from_search(&search(), crate::updates::now());
    fresh.write(&root.path().join("data/latest.toml")).expect("kept");
    let h = start_with(root.path(), "", 101, 100, 30);
    assert_eq!(searches(root.path()), 0, "younger than six hours");
    assert!(row(&h.screen(), "qcode").contains("new 0.1.2"), "{}", h.screen());
    assert!(!has(&h, "Could not check"), "{}", h.screen());
}

#[test]
fn r_and_a_click_on_the_count_ask_again_at_once() {
    let (root, mut h) = updating();
    assert_eq!(searches(root.path()), 1);
    search_scenario(root.path(), &search().replace("quvyta-tools = \"0.1.2\"", "quvyta-tools = \"0.1.3\""), 0);
    h.press("r");
    settle(&mut h);
    assert_eq!(searches(root.path()), 2, "asked although the answer was fresh");
    h.click_text("2 updates");
    settle(&mut h);
    assert_eq!(searches(root.path()), 3);
    search_scenario(root.path(), "", 101);
    h.press("r");
    settle(&mut h);
    let screen = h.screen();
    assert!(screen.contains("Could not check for updates"), "{screen}");
    assert!(row(&screen, "qcode").contains("new 0.1.2"), "the answer before stands:\n{screen}");
}

#[test]
fn with_check_updates_off_nothing_is_asked_until_r() {
    let (root, mut h) = harness_with_settings("check_updates = false\n");
    search_scenario(root.path(), &search(), 0);
    settle(&mut h);
    assert_eq!(searches(root.path()), 0, "not at start");
    assert!(!has(&h, "new 0.1.2"), "{}", h.screen());
    h.press("r");
    settle(&mut h);
    assert_eq!(searches(root.path()), 1, "r is someone asking");
    assert!(row(&h.screen(), "qcode").contains("new 0.1.2"), "{}", h.screen());
}

#[test]
fn install_updates_queues_every_update_in_the_order_of_the_list() {
    let (root, mut h) = updating();
    h.click_text("Install updates");
    settle(&mut h);
    assert_eq!(
        installs(root.path()),
        ["install --locked quvyta-code --version 0.1.2", "install --locked quvyta-framework-showcase --version 0.1.5"]
    );
    let screen = h.screen();
    assert!(row(&screen, "qcode").contains("0.1.2") && !row(&screen, "qcode").contains("new"), "{screen}");
    assert!(row(&screen, "qframe").contains("0.1.5"), "{screen}");
    assert!(!screen.contains("Install updates") && !screen.contains("1 update"), "nothing left to update:\n{screen}");
}

#[test]
fn queued_updates_leave_the_count() {
    let root = tempfile::tempdir().expect("temp");
    let mut app = Quvyta::new(machine(root.path()));
    app.inventory = Some(Inventory::read(&app.machine));
    let latest = crate::updates::Latest::from_search(&search(), crate::updates::now());
    let _ = app.update(Msg::Updates(UpdateMsg::Checked(crate::updates::Check::Known(latest))));
    let _ = app.update(Msg::Updates(UpdateMsg::InstallAll));
    let queued: Vec<usize> = app.installs.running.iter().map(|running| running.index).collect();
    assert_eq!(queued, [index("code")], "the first runs");
    let waiting: Vec<(usize, Action)> = app.installs.queue.iter().cloned().collect();
    assert_eq!(waiting, [(index("framework"), Action::Install(Some("0.1.5".to_owned())))]);
    let h = run(app, 100, 30);
    let screen = h.screen();
    assert!(row(&screen, "qframe").contains("queued"), "{screen}");
    assert!(!screen.contains("Install updates"), "{screen}");
}

#[test]
fn an_install_pins_the_version_the_dialog_shows() {
    let (root, mut h) = updating();
    h.send(Msg::Select(index("tools"))).press("enter");
    let screen = h.screen();
    assert!(screen.contains("Version  0.1.2"), "{screen}");
    // The command is cut short on screen; the copy is whole.
    h.click_text("install --locked quvyta-tools");
    let cargo = root.path().join("bin/cargo").display().to_string();
    assert_eq!(h.copied(), [format!("{cargo} install --locked quvyta-tools --version 0.1.2")]);
    click_beside(&mut h, "Cancel", "Install");
    settle(&mut h);
    assert_eq!(installs(root.path()), ["install --locked quvyta-tools --version 0.1.2"]);
}

#[test]
fn the_update_dialog_names_both_versions_and_updates() {
    let (root, mut h) = updating();
    h.click_text("Update").advance(DIALOG_IN);
    let screen = h.screen();
    assert!(screen.contains("Update qcode from 0.1.1 to 0.1.2?"), "{screen}");
    assert!(screen.contains("Version  0.1.2"), "{screen}");
    h.click_text("install --locked quvyta-code");
    let cargo = root.path().join("bin/cargo").display().to_string();
    assert_eq!(h.copied(), [format!("{cargo} install --locked quvyta-code --version 0.1.2")]);
    click_beside(&mut h, "Cancel", "Update");
    settle(&mut h);
    assert_eq!(installs(root.path()), ["install --locked quvyta-code --version 0.1.2"]);
    let screen = h.advance(TOAST_IN).screen();
    assert!(screen.contains("qcode 0.1.2 installed"), "{screen}");
    assert!(!row(&screen, "qcode").contains("new"), "the list was read again:\n{screen}");
    assert!(screen.contains("1 update"), "qframe is still out of date:\n{screen}");
}

#[test]
fn u_asks_to_update_the_selected_member() {
    let (_root, mut h) = updating();
    assert!(has(&h, "u  update"), "{}", h.screen());
    h.press("u");
    assert!(has(&h, "Update qcode from 0.1.1 to 0.1.2?"), "{}", h.screen());
    h.press("esc").send(Msg::Select(index("tools"))).press("u");
    assert!(h.app().installs.dialog.is_none(), "qtools has nothing to update");
}

#[test]
fn a_failed_update_is_tried_again_at_the_new_version() {
    let (root, mut h) = updating();
    std::fs::write(root.path().join("bin/install.out"), "error: the build broke\n").expect("scenario");
    std::fs::write(root.path().join("bin/install.code"), "101").expect("scenario");
    h.press("u");
    click_beside(&mut h, "Cancel", "Update");
    settle(&mut h);
    let screen = h.screen();
    assert!(screen.contains("qcode could not be installed") && screen.contains("Retry"), "{screen}");
    std::fs::write(root.path().join("bin/install.code"), "0").expect("scenario");
    h.click_text("Retry");
    settle(&mut h);
    let calls = installs(root.path());
    assert_eq!(calls.last().map(String::as_str), Some("install --locked quvyta-code --version 0.1.2"), "{calls:?}");
    assert!(!row(&h.screen(), "qcode").contains("new"), "{}", h.screen());
}

#[test]
fn members_installed_elsewhere_get_no_update() {
    let (_root, mut h) = updating();
    h.send(Msg::Select(index("focus")));
    let screen = h.screen();
    assert!(screen.contains("Open") && !screen.contains("  Update"), "{screen}");
    assert!(!line_with(&screen, "Installed").contains("new"), "{screen}");
    h.send(Msg::Install(InstallMsg::Ask(index("focus"))));
    assert!(h.app().installs.dialog.is_none());
}

/// quvyta installed by cargo at 0.1.1, with `latest` on crates.io.
fn self_update(latest: &str) -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    let app = Quvyta::new(machine(root.path()));
    std::fs::write(root.path().join("bin/list.out"), format!("{LIST}quvyta v0.1.1:\n    quvyta\n")).expect("list");
    installed_command(&app.machine, "quvyta");
    search_scenario(root.path(), &format!("quvyta = \"{latest}\"\n"), 0);
    packages(root.path());
    let mut h = run(app, 100, 30);
    settle(&mut h);
    h.send(Msg::Select(index("quvyta")));
    (root, h)
}

#[test]
fn quvyta_updates_itself_and_the_new_version_runs_next_time() {
    let (root, mut h) = self_update("9.9.9");
    let running = env!("CARGO_PKG_VERSION");
    let screen = h.screen();
    assert!(row(&screen, "quvyta").contains(&format!("{running}  new 9.9.9")), "{screen}");
    assert!(screen.contains("enter  update") && !screen.contains("u  update"), "enter says it once:\n{screen}");
    h.press("enter");
    assert!(has(&h, "Update quvyta from 0.1.1 to 9.9.9?"), "{}", h.screen());
    click_beside(&mut h, "Cancel", "Update");
    settle(&mut h);
    assert_eq!(installs(root.path()), ["install --locked quvyta --version 9.9.9"]);
    let screen = h.advance(TOAST_IN).screen();
    assert!(
        screen.contains("quvyta 9.9.9 installed") && screen.contains("The new version runs next time."),
        "{screen}"
    );
    assert!(!row(&screen, "quvyta").contains("new"), "cargo's record has the new one:\n{screen}");
    assert!(!screen.contains("Update"), "{screen}");
}

#[test]
fn quvyta_installed_without_cargo_is_not_updated_by_it() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = start_with(root.path(), "quvyta = \"9.9.9\"\n", 0, 100, 30);
    h.send(Msg::Select(index("quvyta")));
    let screen = h.screen();
    assert!(!screen.contains("new 9.9.9") && !screen.contains("Update"), "{screen}");
}

#[test]
fn turkish_update_screens_read_naturally() {
    let (_root, mut h) = updating();
    h.set_locale("tr");
    let screen = h.screen();
    for text in ["0.1.1  yeni 0.1.2", "2 güncelleme", "Güncellemeleri kur", "Güncelle"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    h.press("u");
    assert!(has(&h, "qcode 0.1.1 sürümünden 0.1.2 sürümüne güncellensin mi?"), "{}", h.screen());
    let root = tempfile::tempdir().expect("temp");
    let mut h = start_with(root.path(), "", 101, 100, 30);
    h.set_locale("tr");
    assert!(has(&h, "Güncellemeler denetlenemedi"), "{}", h.screen());
}

#[test]
fn update_screens_keep_the_rules_in_ascii_and_on_narrow_screens() {
    for (width, height) in [(40, 30), (48, 30), (60, 30), (100, 30)] {
        for locale in LANGUAGES {
            let root = tempfile::tempdir().expect("temp");
            let mut h = start_with(root.path(), &search(), 0, width, height);
            h.set_glyph_mode(GlyphMode::Ascii).set_locale(locale);
            let mut screens = vec![h.screen()];
            h.send(Msg::ShowDetail(index("code")));
            screens.push(h.screen());
            h.send(Msg::Install(InstallMsg::Ask(index("code"))));
            screens.push(h.screen());
            for screen in screens {
                for forbidden in ['[', ']', '{', '}', '|', '▌', '⟦'] {
                    assert!(!screen.contains(forbidden), "`{forbidden}` at {width}x{height}:\n{screen}");
                }
            }
        }
    }
}

#[test]
fn on_a_narrow_list_the_count_and_its_button_fit() {
    let root = tempfile::tempdir().expect("temp");
    let h = start_with(root.path(), &search(), 0, 40, 24);
    let screen = h.screen();
    assert!(screen.contains("2 updates") && screen.contains("Install updates"), "{screen}");
}

/// With `QUVYTA_REVIEW=1`, writes the screens of updates in both languages, wide and narrow, to
/// `target/quvyta-update-review.html` in colour for a visual review.
#[test]
fn visual_review_updates() {
    if std::env::var_os("QUVYTA_REVIEW").is_none() {
        return;
    }
    let mut fragments = Vec::new();
    let mut shot = |h: &Harness<Quvyta>, caption: String| {
        println!("{caption}\n{}", h.screen());
        fragments.push(h.html(&caption));
    };
    for (width, height) in [(100, 26), (48, 26)] {
        for locale in ["en", "tr"] {
            let size = format!("{locale} {width}x{height}");
            let root = tempfile::tempdir().expect("temp");
            let mut h = start_with(root.path(), &search(), 0, width, height);
            h.set_locale(locale);
            shot(&h, format!("updates {size}"));
            h.send(Msg::ShowDetail(index("code")));
            shot(&h, format!("a member with an update {size}"));
            h.send(Msg::Install(InstallMsg::Ask(index("code")))).advance(DIALOG_IN);
            shot(&h, format!("update dialog {size}"));
            let root = tempfile::tempdir().expect("temp");
            let mut h = start_with(root.path(), "", 101, width, height);
            h.set_locale(locale);
            shot(&h, format!("check failed {size}"));
        }
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/quvyta-update-review.html");
    std::fs::write(path, qframe::runtime::html_page(&fragments)).expect("review page written");
}
