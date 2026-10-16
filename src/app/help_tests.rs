//! The two ways in to everything that has no button: `?` opens the list of every key of the
//! screen that is open, and `ctrl p` opens the palette, where a command is run by typing a few
//! letters of its name.
//!
//! Everything here goes through the keys and the pointer a person has: a key, typed letters, a
//! click on the words of the screen. Nothing installs for real and nothing leaves the folder: the
//! stand-in cargo of `tests/fake-cargo` answers every call, and its `calls.log` is what says that
//! nothing ran.

use qframe::icons::GlyphMode;

use super::cli_tests::installs_ran;
use super::install_tests::{DIALOG_IN, confirm, run, settle};
use super::tests::{LANGUAGES, env, harness, machine, machine_off_path, start, temp_root};
use super::*;
use crate::ecosystem::APPS;
use crate::install::tests::packages;
use crate::updates::tests::search_scenario;

/// Long enough for a layer to have opened, with its title or its filter.
const OPENED: std::time::Duration = std::time::Duration::from_millis(300);

/// What crates.io answers when qcode has a newer version than the machine of the list's tests has.
const NEWER: &str = "\
quvyta-code = \"0.1.2\"                  # Runs containers
quvyta-framework-showcase = \"0.1.5\"    # Every component, live
";

/// The application on the machine in `root`, crates.io answering `search` and every package cargo
/// installs known to it, as a machine somebody has used comes about. The stand-in cargo is written
/// before the scenario beside it, which is the order the machine itself comes about in.
fn with_crates_io(root: &std::path::Path, search: &str) -> Harness<Quvyta> {
    let app = Quvyta::new(machine(root));
    search_scenario(root, search, 0);
    packages(root);
    let mut h = run(app, 100, 30);
    settle(&mut h);
    h
}

/// The title the list of keys draws over the screen, in the language it is drawing in.
fn help_title() -> String {
    env().i18n().translate("quvyta.help.title", &[])
}

/// The faint words of the palette's filter while nothing is typed.
fn placeholder() -> String {
    env().i18n().translate("quvyta.palette.placeholder", &[])
}

/// The lines of the "This screen" group of the list of keys: from its own title to the next one,
/// which is the first of the keymap's groups. Both titles are the framework's own words, read
/// from its language files, so the group is found in whatever language is on the screen.
fn this_screen(screen: &str) -> String {
    let words = env();
    let start = words.i18n().translate("quvyta.help.screen", &[]);
    let after = [words.i18n().translate("quvyta.help.app", &[]), words.i18n().translate("quvyta.help.global", &[])];
    let lines: Vec<&str> = screen.lines().collect();
    let Some(from) = lines.iter().position(|line| line.contains(start.as_str())) else { return String::new() };
    let end = lines[from + 1..]
        .iter()
        .position(|line| after.iter().any(|title| line.contains(title.as_str())))
        .map_or(lines.len(), |next| from + 1 + next);
    lines[from..end].join("\n")
}

/// Every key the "This screen" group lists, in the order it lists them. The layer draws each key
/// in a chip of the key column and the words beside it, so the key is what stands before the last
/// run of blank cells on the line.
fn keys_of(group: &str) -> Vec<String> {
    group
        .lines()
        .skip(1)
        .filter_map(|line| line.trim().rsplit_once("  "))
        .map(|(key, _)| key.trim().to_owned())
        .collect()
}

/// The keys and words the hint bar under the screen draws, in the pairs it draws them: a key
/// chip, then the words that say what it does.
fn bar(screen: &str) -> Vec<(String, String)> {
    let parts: Vec<&str> =
        screen.lines().last().unwrap_or_default().split("  ").map(str::trim).filter(|part| !part.is_empty()).collect();
    parts.chunks(2).filter(|pair| pair.len() == 2).map(|pair| (pair[0].to_owned(), pair[1].to_owned())).collect()
}

#[test]
fn a_question_mark_lists_the_keys_of_the_list_and_esc_gives_them_back_to_the_list() {
    let (root, mut h) = harness(100, 30);
    let title = help_title();
    assert!(h.is_focused("apps"), "the list has the keys from the start");
    h.press("?").advance(OPENED);
    let screen = h.screen();
    assert!(screen.contains(&title), "the list of keys is over the screen:\n{screen}");
    let group = this_screen(&screen);
    for expected in ["enter", "open", "r", "check for updates"] {
        assert!(group.contains(expected), "`{expected}` is missing from the keys of the list:\n{group}");
    }
    assert!(!group.contains(" u "), "nothing has an update yet, so `u` is not a key:\n{group}");
    // The keymap's own actions are the framework's to add, under their own headings.
    let words = env();
    assert!(screen.contains(&words.i18n().translate("quvyta.help.app", &[])), "{screen}");
    assert!(screen.contains(&words.i18n().translate("quvyta.help.global", &[])), "{screen}");
    assert!(screen.contains("ctrl q"), "and the way out is among them:\n{screen}");

    h.press("esc");
    assert!(!h.screen().contains(&title), "esc closes the list of keys:\n{}", h.screen());
    assert!(h.is_focused("apps"), "and the focus went back where it was");
    // Which the keys reach again: the list acts on the app shown once more.
    h.press("enter");
    let [request] = h.handoffs() else { panic!("enter opens the app shown: {:?}", h.handoffs()) };
    assert_eq!(request.program, root.path().join("home/.cargo/bin/qcode").as_os_str());
}

#[test]
fn the_close_mark_of_the_list_of_keys_closes_it_and_gives_the_keys_back_to_the_list() {
    let (_root, mut h) = harness(100, 30);
    h.press("?").advance(OPENED);
    let title = help_title();
    assert!(h.screen().contains(&title), "the list of keys is open:\n{}", h.screen());
    h.click_text("×");
    assert!(!h.screen().contains(&title), "the close mark closes it:\n{}", h.screen());
    assert!(h.is_focused("apps"), "and the focus went back where it was");
    // Which the keys reach again: the list acts on the app shown once more.
    h.press("enter");
    assert_eq!(h.handoffs().len(), 1, "so the list takes the keys again: {:?}", h.handoffs());
}

#[test]
fn a_click_outside_the_list_of_keys_closes_it_and_a_click_on_it_does_not() {
    let (_root, mut h) = harness(100, 30);
    h.press("?").advance(OPENED);
    let title = help_title();
    assert!(h.screen().contains(&title), "the list of keys is open:\n{}", h.screen());
    let (x, y) = h.find(&title).expect("the title of the list of keys");
    h.click(x, y);
    assert!(h.screen().contains(&title), "a click on the list itself keeps it open:\n{}", h.screen());
    // The layer stands in the middle of the screen, so the top left corner is outside it.
    h.click(0, 0);
    assert!(!h.screen().contains(&title), "a click on the dimmed screen closes it:\n{}", h.screen());
    assert!(h.is_focused("apps"), "and the focus went back where it was");
    h.press("enter");
    assert_eq!(h.handoffs().len(), 1, "so the list takes the keys again: {:?}", h.handoffs());
}

#[test]
fn a_click_outside_the_palette_closes_it_and_gives_the_keys_back_to_the_list() {
    let (_root, mut h) = harness(100, 30);
    h.press("ctrl+p").advance(OPENED);
    let filter = placeholder();
    assert!(h.screen().contains(&filter), "the palette is over the screen:\n{}", h.screen());
    // The palette stands in the upper middle of the screen, so the bottom left corner is outside it.
    h.click(0, 29);
    assert!(!h.screen().contains(&filter), "a click on the dimmed screen closes it:\n{}", h.screen());
    assert!(h.is_focused("apps"), "and the focus went back where it was");
    h.press("enter");
    assert_eq!(h.handoffs().len(), 1, "so the list takes the keys again: {:?}", h.handoffs());
}

#[test]
fn the_list_of_keys_says_nothing_twice_about_one_key() {
    // The list of keys asks the focused widget for the keys it takes and adds the screen's own,
    // so a key both would answer to is named once. A narrow list takes enter itself, and a hint
    // saying a second thing about it would put two rows for one key on the screen.
    for (width, height, page) in [(100, 30, false), (48, 20, false), (48, 20, true), (100, 44, false)] {
        let (_root, mut h) = harness(width, height);
        if page {
            h.press("enter");
        }
        if width == 100 && height == 44 {
            h.click_text("Settings");
        }
        h.press("?").advance(OPENED);
        let group = this_screen(&h.screen());
        let keys = keys_of(&group);
        let mut said = keys.clone();
        said.sort();
        said.dedup();
        assert_eq!(said.len(), keys.len(), "a key is listed twice at {width}x{height}, page {page}:\n{group}");
        assert!(!keys.is_empty(), "the keys of the screen at {width}x{height} are listed:\n{group}");
    }
}

#[test]
fn the_list_of_keys_names_u_where_an_update_is_offered() {
    let root = temp_root();
    let mut h = with_crates_io(root.path(), NEWER);
    // qcode: installed, and crates.io has a newer one, so it can be both opened and updated.
    assert!(h.screen().contains("new 0.1.2"), "qcode has an update:\n{}", h.screen());
    assert!(h.screen().contains("u  update"), "the bar says what `u` does:\n{}", h.screen());
    h.press("?").advance(OPENED);
    let group = this_screen(&h.screen());
    let update =
        group.lines().find(|line| line.contains(" u ")).unwrap_or_else(|| panic!("`u` is not listed:\n{group}"));
    assert!(update.contains("update"), "and it says what it does:\n{update}");
    assert!(group.contains("enter") && group.contains("open"), "enter still opens what is there:\n{group}");
}

#[test]
fn the_list_of_keys_agrees_with_the_hint_bar_under_the_screen() {
    let (_root, mut h) = harness(100, 30);
    let hints = bar(&h.screen());
    h.press("?").advance(OPENED);
    let group = this_screen(&h.screen());
    // Every key quvyta binds and draws in the bar is in the list with the same words, so the two
    // cannot say different things about one key.
    for action in ["primary", "back", "refresh", "update"] {
        let Some(key) = h.env().keymap().label_for(Scope::App, action) else { continue };
        let Some((_, word)) = hints.iter().find(|(chip, _)| *chip == key) else { continue };
        let line = group
            .lines()
            .find(|line| line.contains(&format!(" {key} ")))
            .unwrap_or_else(|| panic!("the keys of the list say nothing about `{key}`, which the bar draws:\n{group}"));
        assert!(line.contains(word.as_str()), "the bar says `{key}` is `{word}`:\n{line}");
    }
    assert!(!hints.is_empty(), "the bar draws something: {hints:?}");
}

#[test]
fn a_question_mark_on_the_settings_tab_lists_that_tabs_keys() {
    let (_root, mut h) = harness(100, 44);
    h.click_text("Settings");
    h.press("?").advance(OPENED);
    let screen = h.screen();
    assert!(screen.contains(&help_title()), "{screen}");
    let group = this_screen(&screen);
    assert!(group.contains("esc") && group.contains("apps"), "esc goes back to the app list:\n{group}");
    // The app list's keys would act on an app nobody sees, so they are not listed here.
    assert!(!group.contains("enter"), "the app list's keys are left out:\n{group}");
    h.press("esc");
    assert!(
        h.screen().contains("Do the applications follow the shared settings"),
        "and the Settings tab is still there:\n{}",
        h.screen()
    );
}

#[test]
fn ctrl_p_opens_the_palette_and_a_command_asks_before_anything_runs() {
    let (root, mut h) = harness(100, 30);
    h.press("ctrl+p").advance(OPENED);
    let screen = h.screen();
    assert!(screen.contains(&placeholder()), "the palette is over the screen:\n{screen}");
    h.type_text("Install qtools").press("enter").advance(DIALOG_IN);
    let screen = h.screen();
    assert!(screen.contains("Install qtools?"), "the very question the button asks:\n{screen}");
    assert!(!screen.contains(&placeholder()), "the palette is closed:\n{screen}");
    assert_eq!(installs_ran(root.path()), 0, "and nothing is installed before it is agreed to");
}

#[test]
fn the_missing_apps_are_in_the_list_of_keys_and_the_palette_and_still_asked_for_one_by_one() {
    let (root, mut h) = harness(100, 30);
    h.press("?").advance(OPENED);
    let group = this_screen(&h.screen());
    let missing =
        group.lines().find(|line| line.contains(" i ")).unwrap_or_else(|| panic!("`i` is not listed:\n{group}"));
    assert!(missing.contains("missing"), "and it says what it is for, as the bar does:\n{missing}");
    h.press("esc");

    h.press("ctrl+p").advance(OPENED).type_text("Install the missing").press("enter").advance(DIALOG_IN);
    let screen = h.screen();
    assert!(screen.contains("Install qtools?"), "the first missing app's own question:\n{screen}");
    assert_eq!(installs_ran(root.path()), 0, "and nothing is installed before it is agreed to");
}

#[test]
fn an_install_asked_for_in_the_palette_asks_first_and_runs_when_it_is_agreed_to() {
    let (root, mut h) = harness(100, 30);
    h.press("ctrl+p").advance(OPENED).type_text("Install qtools").press("enter").advance(DIALOG_IN);
    assert_eq!(installs_ran(root.path()), 0, "the question is the whole of what happened so far");
    // The dialog's own button answers it, exactly as it answers the key and the button on screen.
    confirm(&mut h);
    settle(&mut h);
    assert_eq!(installs_ran(root.path()), 1, "agreeing to the question installs it");
}

#[test]
fn the_palette_offers_an_update_only_where_there_is_one() {
    // crates.io answers nothing at all, so nothing is out of date.
    let (_root, mut h) = harness(100, 30);
    h.press("ctrl+p").advance(OPENED).type_text("qcode");
    let screen = h.screen();
    assert!(screen.contains("Open qcode") && screen.contains("Show qcode"), "{screen}");
    assert!(!screen.contains("Update qcode"), "there is no update:\n{screen}");
    assert!(!screen.contains("Install qcode"), "and it is installed already:\n{screen}");

    // crates.io has a newer qcode, so the command the Update button sends is there.
    let root = temp_root();
    let mut h = with_crates_io(root.path(), NEWER);
    h.press("ctrl+p").advance(OPENED).type_text("qcode");
    let screen = h.screen();
    assert!(screen.contains("Update qcode"), "{screen}");
    assert!(!screen.contains("Install qcode"), "an installed app is not installed again:\n{screen}");
}

#[test]
fn the_palette_offers_no_install_for_an_app_that_cannot_run_here() {
    let root = temp_root();
    std::fs::create_dir_all(root.path().join("etc")).expect("folder");
    // Named before the machine is built, since quvyta reads it as it starts.
    std::fs::write(root.path().join("etc/os-release"), "ID=debian\n").expect("os-release");
    let mut h = start(root.path(), 100, 30);
    assert!(h.screen().contains("Arch Linux only"), "qtools says why: {}", h.screen());
    h.press("ctrl+p").advance(OPENED).type_text("qtools");
    let screen = h.screen();
    assert!(screen.contains("Show qtools"), "the app is still on the list:\n{screen}");
    for forbidden in ["Install qtools", "Open qtools", "Update qtools", "Remove qtools"] {
        assert!(!screen.contains(forbidden), "`{forbidden}` is offered for an app that cannot run here:\n{screen}");
    }
}

#[test]
fn a_command_of_the_palette_does_exactly_what_the_button_does() {
    let (_root, mut h) = harness(100, 30);
    h.click_text("Open");
    let [button] = h.handoffs() else { panic!("the button opened it: {:?}", h.handoffs()) };
    let button = button.clone();
    h.press("ctrl+p").advance(OPENED).type_text("Open qcode").press("enter");
    let [_, command] = h.handoffs() else { panic!("the command opened it too: {:?}", h.handoffs()) };
    assert_eq!(button, *command, "the palette's Open is the button's Open");
}

#[test]
fn a_command_of_the_palette_runs_the_two_tabs_and_the_way_out() {
    let (_root, mut h) = harness(100, 30);
    h.press("ctrl+p").advance(OPENED).type_text("Settings").press("enter");
    assert!(!h.screen().contains("qtools"), "the tab command changed tab:\n{}", h.screen());
    // The list's commands are the ones the list offers, so this tab offers none of them.
    h.press("ctrl+p").advance(OPENED).type_text("qcode");
    let settings = h.screen();
    assert!(settings.contains("No matching commands"), "no app is on this tab:\n{settings}");
    h.press("esc");
    h.press("ctrl+p").advance(OPENED).type_text("Apps").press("enter");
    assert!(h.screen().contains("qtools"), "and back to the list:\n{}", h.screen());
    h.press("ctrl+p").advance(OPENED).type_text("quit").press("enter");
    assert!(h.quit_requested(), "the framework's own way out");
}

#[test]
fn the_palette_offers_the_apps_where_the_list_is_and_nowhere_else() {
    // The page of what changed is over the list, and the keys reach nothing but its Back while it
    // is there, so a command that would act on an app is no command to run.
    let root = temp_root();
    std::fs::create_dir_all(root.path().join("data")).expect("folder");
    std::fs::write(root.path().join("data/seen.toml"), "version = \"0.0.1\"\n").expect("seen");
    let mut h = start(root.path(), 100, 30);
    assert!(h.screen().contains("What changed in"), "the page is over the list:\n{}", h.screen());
    h.press("ctrl+p").advance(OPENED).type_text("qcode");
    let screen = h.screen();
    assert!(screen.contains("No matching commands"), "no app is under the page:\n{screen}");
    h.press("esc");
    h.press("esc");
    assert!(h.screen().contains("qtools"), "and the list is back:\n{}", h.screen());

    // On a narrow screen an app's own page has taken the list's place, so the list's commands are
    // not on offer there either; the two tabs and the way out are.
    let (_root, mut h) = harness(48, 20);
    h.press("enter");
    assert!(!h.screen().contains("qtools"), "an app's page is over the list:\n{}", h.screen());
    h.press("ctrl+p").advance(OPENED).type_text("qcode");
    let screen = h.screen();
    assert!(screen.contains("No matching commands"), "no app is under the page:\n{screen}");
    assert!(screen.contains("Settings") && screen.contains("quit"), "but the ways out are:\n{screen}");
}

#[test]
fn the_palette_offers_the_path_notices_add_while_the_notice_offers_it() {
    // A machine whose cargo folder is not on PATH is asked at start, and the notice offers to add
    // the line that would put it there.
    let root = temp_root();
    let mut machine = machine_off_path(root.path());
    machine.shell = Some("/bin/bash".to_owned());
    let mut h = Harness::with_env(Quvyta::new(machine), env(), 100, 30);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    // One frame hears cargo, which starts the check; the next hears the check.
    h.render().render();
    settle(&mut h);
    assert!(
        h.screen().contains("is not on PATH") && h.screen().contains("Add"),
        "the notice offers its Add:\n{}",
        h.screen()
    );
    h.press("ctrl+p").advance(OPENED).type_text("Add");
    let screen = h.screen();
    // The palette draws its list over the app list and the notice behind it, so the row naming
    // the command is the one that is neither the filter's own text nor the notice's button.
    let named: Vec<&str> = screen
        .lines()
        .map(str::trim_end)
        .filter(|row| row.ends_with("Add") && !row.contains('⌕') && !row.contains("Not now"))
        .collect();
    assert_eq!(named.len(), 1, "and the palette names that very command:\n{screen}");
    // Which runs the very message the notice's own button sends.
    h.press("enter");
    settle(&mut h);
    let screen = h.screen();
    assert!(screen.contains("Added. Terminals you open from now on can start"), "the line is written:\n{screen}");
    assert!(!screen.contains("Not now"), "and the notice has nothing left to offer:\n{screen}");
    // Which is why the palette offers it no more.
    h.press("ctrl+p").advance(OPENED).type_text("Add");
    assert!(h.screen().contains("No matching commands"), "the command went with the offer:\n{}", h.screen());
}

/// Every word the list of keys and the palette added, which each of the nine languages has to
/// carry itself: a language that borrows the English one shows the words nobody translated.
const WORDS: [&str; 11] = [
    "keys.primary",
    "keys.back",
    "keys.refresh",
    "keys.update",
    "palette.open",
    "palette.install",
    "palette.update",
    "palette.remove",
    "palette.show",
    "palette.check",
    "palette.missing",
];

#[test]
fn every_new_word_is_written_in_every_language() {
    let english = env();
    for locale in LANGUAGES.iter().filter(|locale| **locale != "en") {
        let (_root, mut h) = harness(100, 24);
        h.set_locale(locale);
        for key in WORDS {
            assert!(h.env().i18n().has(locale, key), "`{key}` is not `{locale}`'s own word");
            let command = [("command", "qcode".into())];
            let here = h.env().i18n().translate(key, &command);
            let there = english.i18n().translate(key, &command);
            assert_ne!(here, there, "`{key}` in `{locale}` is still the English one: {here}");
        }
    }
    assert_eq!(WORDS.len(), 11, "every word the two layers added is asked for");
}

/// The apps the palette names and what it offers each of them, on the machine the list's tests
/// use: what the details of the app show now, and the page itself. Written out rather than worked
/// out from the app, so a change in what the screen offers is caught here and not on somebody's
/// screen.
#[test]
fn the_palette_offers_each_app_what_its_details_offer() {
    let verbs = ["Open", "Install", "Update", "Remove", "Show"];
    let offered: [(&str, &[&str]); 10] = [
        ("qcode", &["Open", "Remove", "Show"]),
        ("qfocus", &["Open", "Show"]),
        ("qtools", &["Install", "Show"]),
        ("qpac", &["Install", "Show"]),
        ("qexp", &["Install", "Show"]),
        ("qbrow", &["Install", "Show"]),
        ("qcli", &["Install", "Show"]),
        ("qdesk", &["Install", "Show"]),
        ("qframe", &["Open", "Remove", "Show"]),
        ("quvyta", &["Show"]),
    ];
    assert_eq!(offered.len(), APPS.len(), "every app on the list is asked about");
    let (_root, mut h) = harness(100, 30);
    for (command, expected) in offered {
        h.press("ctrl+p").advance(OPENED).type_text(command);
        let screen = h.screen();
        for verb in verbs {
            assert_eq!(
                screen.contains(&format!("{verb} {command}")),
                expected.contains(&verb),
                "`{verb} {command}` on a screen offering {expected:?}:\n{screen}"
            );
        }
        h.press("esc");
    }
}
