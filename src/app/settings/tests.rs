use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use qframe::i18n::I18n;
use qframe::icons::GlyphMode;
use qframe::runtime::{HandoffOutcome, Harness};
use qframe::storage::{Family, Source};

use super::super::tests::{
    TOAST_IN, env, harness, harness_with_settings, harness_with_settings_at, line_with, machine, machine_off_path,
};
use super::*;
use crate::app::{PathMsg, Tab};
use crate::launcher::PathPrompt;
use crate::machine::Machine;

/// The application on `machine` at `width` by `height`, with what is installed read and the
/// checks that follow it answered, showing the Settings tab.
fn settings_on(machine: Machine, width: u16, height: u16) -> Harness<Quvyta> {
    let mut h = Harness::with_env(Quvyta::new(machine), env(), width, height);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    h.render().render();
    h.send(Msg::Tab(Tab::Settings));
    h
}

/// The last screen row: the key hints.
fn footer(screen: &str) -> &str {
    screen.lines().last().unwrap_or_default()
}

fn launcher_conf(root: &Path) -> String {
    fs::read_to_string(root.join("config/launcher.conf")).expect("written")
}

/// What the family's shared file holds now.
fn quvyta_conf(root: &Path) -> String {
    fs::read_to_string(root.join("config/quvyta.conf")).expect("written")
}

/// The appearance the family shares, as the shared file holds it before a test starts. Written by
/// hand rather than detected, so the rows start from the same values on every machine.
const SHARED: &str = "language = \"en\"\ntheme = \"monochrome\"\nicons = \"unicode\"\n";

/// The machine of [`machine`] whose family folder already holds the shared file `shared` and, when
/// `own` is not empty, quvyta's own `launcher.conf`.
fn family_files(root: &Path, shared: &str, own: &str) -> Machine {
    fs::create_dir_all(root.join("config")).expect("folder");
    fs::write(root.join("config/quvyta.conf"), shared).expect("shared file");
    if !own.is_empty() {
        fs::write(root.join("config/launcher.conf"), own).expect("launcher.conf");
    }
    machine(root)
}

/// The Settings tab on a screen tall enough for both sections, with the shared file in place.
fn shared_settings(root: &Path) -> Harness<Quvyta> {
    settings_on(family_files(root, SHARED, ""), 100, 44)
}

/// Where `text` starts on `screen`.
fn at(screen: &str, text: &str) -> usize {
    screen.find(text).unwrap_or_else(|| panic!("`{text}` is missing:\n{screen}"))
}

#[test]
fn the_appearance_the_family_shares_stands_above_quvyta_s_own_settings() {
    let root = tempfile::tempdir().expect("temp");
    let h = shared_settings(root.path());
    let screen = h.screen();
    for text in ["Appearance", "Language", "Theme", "Icons", "Reduce motion", "Pillar"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert_eq!(screen.matches("In every Quvyta application").count(), 3, "one box under each shared row:\n{screen}");
    assert!(at(&screen, "Appearance") < at(&screen, "launcher.conf"), "the shared rows come first:\n{screen}");
    assert!(at(&screen, "Pillar") < at(&screen, "Check for updates at start"), "{screen}");
    // quvyta writes no appearance row of its own: the three rows and their boxes are the only ones.
    assert_eq!(screen.matches("Language").count(), 1, "{screen}");
    let prefs = h.app().preferences();
    assert_eq!(prefs.theme().source, Source::Family, "quvyta follows the family it shows");
    assert_eq!(prefs.language().value, "en");
}

#[test]
fn a_theme_chosen_here_goes_to_the_shared_file_and_every_member_follows() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = shared_settings(root.path());
    h.click_text("Monochrome").advance(TOAST_IN);
    h.click_text("Nordic").render();
    assert_eq!(h.env().theme().id(), "nordic", "the running quvyta has it at once");
    assert!(quvyta_conf(root.path()).contains("theme = \"nordic\""), "{}", quvyta_conf(root.path()));
    assert!(quvyta_conf(root.path()).contains("language = \"en\""), "the other shared keys stay");
    assert_eq!(launcher_conf(root.path()), "theme = \"quvyta\"\n", "quvyta's own file says it follows");
    // Another member of the family reads the same theme from the shared file.
    let folder = root.path().join("config");
    let code = Family::QUVYTA.preferences_in(&folder, "code", &I18n::builtin());
    assert_eq!(code.theme().value, "nordic");
}

#[test]
fn clearing_the_box_keeps_the_choice_to_quvyta_and_leaves_the_family_alone() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = settings_on(family_files(root.path(), SHARED, "after_close = \"shell\"\n"), 100, 44);
    // The box under the theme row: the keys reach it from the row above, since all three boxes
    // read the same.
    h.click_text("Theme");
    h.press("down").press("space").render();
    assert_eq!(h.app().preferences().theme().source, Source::App, "only here now");
    h.send(Msg::Setting(SettingMsg::Appearance(AppearanceChange::Theme("iris".to_owned()))));
    assert_eq!(h.env().theme().id(), "iris");
    let own = launcher_conf(root.path());
    assert!(own.contains("theme = \"iris\"") && own.contains("after_close = \"shell\""), "{own}");
    assert!(quvyta_conf(root.path()).contains("theme = \"monochrome\""), "the family keeps its theme");

    // Checking it again hands the value to the family and quvyta follows it once more.
    h.send(Msg::Setting(SettingMsg::Appearance(AppearanceChange::Everywhere(qframe::storage::Shared::Theme, true))));
    assert!(quvyta_conf(root.path()).contains("theme = \"iris\""), "{}", quvyta_conf(root.path()));
    assert!(launcher_conf(root.path()).contains("theme = \"quvyta\""), "{}", launcher_conf(root.path()));
}

#[test]
fn quvyta_in_its_own_file_follows_the_family_and_its_own_value_does_not() {
    let root = tempfile::tempdir().expect("temp");
    let shared = "language = \"tr\"\ntheme = \"nordic\"\nicons = \"unicode\"\n";
    let following = family_files(root.path(), shared, "theme = \"quvyta\"\nafter_close = \"shell\"\n");
    let h = settings_on(following, 100, 44);
    let prefs = h.app().preferences();
    assert_eq!((prefs.theme().value.as_str(), prefs.theme().source), ("nordic", Source::Family));
    assert_eq!(prefs.language().value, "tr", "the shared language too");
    assert_eq!(h.app().launcher.after_close, AfterClose::Shell, "the rows of launcher.conf are read as before");
    assert_eq!(h.app().settings().diagnostics(), [], "`quvyta` is a value its own file may hold");
    let screen = h.screen();
    assert_eq!(screen.matches("In every Quvyta application").count(), 3, "{screen}");

    let own = family_files(root.path(), shared, "theme = \"amber\"\n");
    let h = settings_on(own, 100, 44);
    let prefs = h.app().preferences();
    assert_eq!((prefs.theme().value.as_str(), prefs.theme().source), ("amber", Source::App));
    assert_eq!(h.app().settings().diagnostics(), [], "a theme of its own is no problem either");
}

#[test]
fn the_language_row_speaks_both_languages_and_switching_takes_at_once() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = shared_settings(root.path());
    let screen = h.screen();
    for text in ["Appearance", "Language", "In every Quvyta application"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    h.click_text("English").advance(TOAST_IN);
    h.click_text("Türkçe").render();
    let screen = h.screen();
    for text in ["Görünüm", "Dil", "Tüm Quvyta uygulamalarında", "Hareketi azalt", "Ayarlar"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(quvyta_conf(root.path()).contains("language = \"tr\""), "{}", quvyta_conf(root.path()));
    assert_eq!(launcher_conf(root.path()), "language = \"quvyta\"\n");
}

#[test]
fn a_click_on_a_tab_shows_it_and_the_list_comes_back_as_it_was() {
    let (_root, mut h) = harness(100, 24);
    h.press("down");
    h.click_text("Settings");
    let screen = h.screen();
    for text in ["Check for updates at start", "When an app closes", "back to quvyta", "~/.cargo/bin on PATH"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("qtools") && !screen.contains("Quvyta Focus"), "{screen}");
    h.click_text("Apps");
    let screen = h.screen();
    assert!(line_with(&screen, "qfocus").contains('▌'), "the selection is kept:\n{screen}");
    assert!(!screen.contains("Check for updates at start"), "{screen}");
}

#[test]
fn the_keyboard_reaches_the_tabs_with_tab_and_moves_between_them_with_their_keys() {
    let (_root, mut h) = harness(100, 24);
    for _ in 0..12 {
        if h.is_focused("tabs") {
            break;
        }
        h.press("shift+tab");
    }
    assert!(h.is_focused("tabs"), "shift+tab reaches the tabs");
    h.press("right");
    assert!(h.screen().contains("Check for updates at start"), "{}", h.screen());
    assert!(h.is_focused("tabs"), "the keys stay on the tabs");
    h.press("left");
    assert!(h.screen().contains("Quvyta Code"), "{}", h.screen());

    h.press("right").press("tab");
    assert!(h.is_focused("page"), "tab goes on to the page, which scrolls");
    h.press("tab");
    assert!(h.is_focused("appearance"), "and then to the appearance the family shares");
    h.press("tab");
    assert!(h.is_focused("settings"), "and then to quvyta's own settings");
    h.press("esc");
    assert!(h.screen().contains("Quvyta Code"), "esc goes back to the family:\n{}", h.screen());
    assert!(h.is_focused("family"), "and gives the list the keys");
}

#[test]
fn the_settings_hints_leave_out_the_family_s_keys_and_those_keys_do_nothing() {
    let (_root, mut h) = harness(100, 24);
    let apps = h.screen();
    assert!(footer(&apps).contains("check for updates"), "{apps}");
    h.send(Msg::Tab(Tab::Settings));
    let screen = h.screen();
    let hints = footer(&screen);
    assert!(!hints.contains("check for updates") && !hints.contains("update") && !hints.contains("open"), "{screen}");
    assert!(hints.contains("choose") && hints.contains("esc"), "{screen}");
    h.press("enter").press("u").press("r");
    assert!(h.handoffs().is_empty(), "enter on the settings opens no member");
    assert!(!h.screen().contains("Install quvyta"), "{}", h.screen());
}

#[test]
fn turning_updates_off_writes_it_at_once_and_keeps_the_other_settings() {
    let (root, mut h) = harness_with_settings("after_close = \"shell\"\npath_prompt = \"dismissed\"\n");
    h.send(Msg::Tab(Tab::Settings));
    h.click_text("Check for updates at start");
    assert!(h.is_focused("settings"), "a click on a label gives the list the keys");
    h.press("space").render();
    assert!(!h.app().launcher.check_updates, "the running quvyta has it at once");
    let text = launcher_conf(root.path());
    assert!(text.contains("check_updates = false"), "{text}");
    assert!(text.contains("after_close = \"shell\"") && text.contains("path_prompt = \"dismissed\""), "{text}");
    let launcher = Launcher::load(Some(&root.path().join("config/launcher.conf")));
    assert_eq!((launcher.check_updates, launcher.path_prompt), (false, PathPrompt::Dismissed));
}

#[test]
fn choosing_quit_to_shell_writes_it_and_the_next_close_leaves_with_the_member() {
    let (root, mut h) = harness(100, 24);
    h.send(Msg::Tab(Tab::Settings));
    h.click_text("back to quvyta").advance(TOAST_IN);
    h.click_text("quit to shell");
    h.render();
    assert!(h.screen().contains("quit to shell"), "{}", h.screen());
    assert_eq!(launcher_conf(root.path()), "after_close = \"shell\"\n");
    h.send(Msg::Tab(Tab::Apps));
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).press("enter");
    assert!(h.quit_requested(), "the next close already follows the new setting");
}

#[test]
fn a_folder_that_cannot_be_written_puts_the_value_back_and_says_where_and_why() {
    let (root, mut h) = harness_with_settings_at("after_close = \"return\"\n", 100, 40);
    let config = root.path().join("config");
    fs::set_permissions(&config, fs::Permissions::from_mode(0o555)).expect("read-only");
    h.send(Msg::Tab(Tab::Settings));
    h.send(Msg::Setting(SettingMsg::Change(Change::CheckUpdates(false))));
    h.send(Msg::Setting(SettingMsg::Change(Change::AfterClose(AfterClose::Shell))));
    h.render().advance(TOAST_IN);
    let screen = h.screen();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o755)).expect("writable again");
    assert!(h.app().launcher.check_updates, "the switch goes back");
    assert_eq!(h.app().launcher.after_close, AfterClose::Return, "and so does the choice");
    assert!(screen.contains("back to quvyta"), "{screen}");
    assert!(screen.contains("The setting could not be saved"), "{screen}");
    assert!(screen.contains(&config.display().to_string()), "the notice names the folder:\n{screen}");
    assert!(screen.contains("Permission denied"), "and why:\n{screen}");
    assert_eq!(launcher_conf(root.path()), "after_close = \"return\"\n");
}

#[test]
fn a_broken_file_still_shows_the_settings_with_their_defaults() {
    let (_root, mut h) = harness_with_settings_at("after_close = \"exit\"\n", 100, 40);
    h.send(Msg::Tab(Tab::Settings)).advance(TOAST_IN);
    let screen = h.screen();
    assert!(screen.contains("back to quvyta") && screen.contains("launcher.conf:1:"), "{screen}");
}

#[test]
fn the_path_row_says_yes_when_cargo_s_folder_is_on_path() {
    let root = tempfile::tempdir().expect("temp");
    let h = settings_on(machine(root.path()), 100, 24);
    let row = line_with(&h.screen(), "~/.cargo/bin on PATH").to_owned();
    assert!(row.trim_end().ends_with("yes") && !row.contains("Add"), "{row}");
}

#[test]
fn add_opens_the_path_notice_even_after_not_now() {
    let root = tempfile::tempdir().expect("temp");
    fs::create_dir_all(root.path().join("config")).expect("folder");
    fs::write(root.path().join("config/launcher.conf"), "path_prompt = \"dismissed\"\n").expect("settings");
    let mut machine = machine_off_path(root.path());
    machine.shell = Some("/bin/bash".to_owned());
    let mut h = settings_on(machine, 100, 44);
    let screen = h.screen();
    let row = line_with(&screen, "~/.cargo/bin on PATH");
    assert!(row.contains("no") && row.contains("Add"), "{screen}");
    assert!(!screen.contains("is not on PATH"), "Not now keeps the offer away:\n{screen}");

    h.click_text("Add").render();
    let screen = h.screen();
    for text in ["~/.cargo/bin is not on PATH", "File  ~/.bashrc", "Not now"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(h.is_focused("path-add"), "the notice's Add has the keys");
    h.press("enter").render();
    let bashrc = fs::read_to_string(root.path().join("home/.bashrc")).expect("added");
    assert!(bashrc.contains("export PATH=\"$HOME/.cargo/bin:$PATH\""), "{bashrc}");
    let screen = h.screen();
    assert!(line_with(&screen, "~/.cargo/bin on PATH").contains("in new terminals"), "{screen}");
    assert!(screen.contains("Added."), "{screen}");
}

#[test]
fn enter_on_the_path_row_opens_the_notice_too() {
    let root = tempfile::tempdir().expect("temp");
    let mut machine = machine_off_path(root.path());
    machine.shell = Some("/usr/bin/zsh".to_owned());
    let mut h = settings_on(machine, 100, 44);
    // At start quvyta offered the notice by itself; Not now puts it away.
    h.click_text("Not now");
    assert!(!h.screen().contains("is not on PATH"), "{}", h.screen());
    h.click_text("Check for updates at start");
    h.press("down").press("down").press("enter").render();
    assert!(h.screen().contains("File  ~/.zshrc"), "{}", h.screen());
}

#[test]
fn turkish_reads_naturally() {
    let root = tempfile::tempdir().expect("temp");
    let mut machine = machine_off_path(root.path());
    machine.shell = Some("/bin/bash".to_owned());
    let mut h = settings_on(machine, 100, 44);
    h.set_locale("tr");
    let screen = h.screen();
    for text in [
        "Uygulamalar",
        "Ayarlar",
        "Açılışta güncellemeleri denetle",
        "Uygulama kapanınca",
        "quvyta'ya dön",
        "~/.cargo/bin PATH'te",
        "hayır",
        "Ekle",
        "uygulamalar",
    ] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    let (_root, mut h) = harness(100, 24);
    h.set_locale("tr").send(Msg::Tab(Tab::Settings));
    assert!(line_with(&h.screen(), "PATH'te").trim_end().ends_with("evet"), "{}", h.screen());
    h.click_text("quvyta'ya dön").advance(TOAST_IN);
    assert!(h.screen().contains("kabuğa çık"), "{}", h.screen());
}

#[test]
fn narrow_screens_cut_nothing_and_put_the_path_under_the_title() {
    for locale in ["en", "tr"] {
        for width in [48, 60, 99] {
            let root = tempfile::tempdir().expect("temp");
            let mut machine = machine_off_path(root.path());
            machine.shell = Some("/bin/bash".to_owned());
            let mut h = settings_on(machine, width, 44);
            h.set_locale(locale);
            h.click_text(if locale == "en" { "Not now" } else { "Şimdi değil" });
            let screen = h.screen();
            let body: Vec<&str> = screen.lines().skip(1).collect();
            assert!(!body.iter().any(|line| line.contains('…')), "{locale} at {width}:\n{screen}");
            let title = body.iter().position(|line| line.trim() == "quvyta").expect("the title alone");
            assert!(body[title + 1].contains("launcher.conf"), "{locale} at {width}:\n{screen}");
        }
    }
    let (_root, h) = harness(100, 24);
    let mut h = h;
    h.send(Msg::Tab(Tab::Settings));
    assert!(line_with(&h.screen(), "launcher.conf").contains("quvyta"), "wide, the path is beside the title");
}

#[test]
fn a_path_longer_than_a_narrow_screen_keeps_its_start_and_its_file_name() {
    let root = tempfile::tempdir().expect("temp");
    let mut machine = machine(root.path());
    let deep = root.path().join("a-settings-folder-with-a-rather-long-name/quvyta/launcher.conf");
    machine.launcher_conf = Some(deep);
    let h = settings_on(machine, 48, 44);
    let screen = h.screen();
    let line = line_with(&screen, "…").trim();
    assert!(line.starts_with('/') && line.ends_with("launcher.conf"), "{screen}");
    assert_eq!(qframe::text::width(line), 42, "{screen}");
}

#[test]
fn narrow_ascii_settings_keep_the_rules() {
    for (width, height) in [(40, 16), (48, 20), (60, 20), (100, 24)] {
        for locale in ["en", "tr"] {
            let root = tempfile::tempdir().expect("temp");
            let mut machine = machine_off_path(root.path());
            machine.shell = Some("/bin/bash".to_owned());
            let mut h = settings_on(machine, width, height);
            h.set_locale(locale).set_glyph_mode(GlyphMode::Ascii);
            let screen = h.screen();
            assert!(screen.contains("quvyta"), "{width}x{height}:\n{screen}");
            let heading = if locale == "en" { "Appearance" } else { "Görünüm" };
            assert!(screen.contains(heading), "the shared rows at {width}x{height} {locale}:\n{screen}");
            for forbidden in ['[', ']', '{', '}', '|', '▌'] {
                assert!(!screen.contains(forbidden), "`{forbidden}` at {width}x{height} {locale}:\n{screen}");
            }
        }
    }
}

/// The Settings tab for the visual review: both languages, wide, narrow and short, the whole page
/// with the appearance the family shares, a box cleared, an open language list, with cargo's
/// folder on `PATH` and off it, the notice its Add opens, an open select and a file that
/// cannot be written.
pub(in crate::app) fn review() -> Vec<String> {
    let mut fragments = Vec::new();
    let mut shot = |h: &Harness<Quvyta>, name: String| {
        println!("{name}\n{}", h.screen());
        fragments.push(h.html(&name));
    };
    for locale in ["en", "tr"] {
        for (width, height) in [(100, 22), (48, 26), (72, 14)] {
            let (_root, mut h) = harness(width, height);
            h.set_locale(locale).send(Msg::Tab(Tab::Settings));
            shot(&h, format!("settings {locale} {width}x{height}"));

            let root = tempfile::tempdir().expect("temp");
            let mut machine = machine_off_path(root.path());
            machine.shell = Some("/bin/bash".to_owned());
            let mut h = settings_on(machine, width, height);
            h.set_locale(locale).send(Msg::Path(PathMsg::NotNow));
            shot(&h, format!("settings off PATH {locale} {width}x{height}"));
            h.send(Msg::Setting(SettingMsg::AddToPath)).render();
            shot(&h, format!("settings PATH notice {locale} {width}x{height}"));
        }
        let (_root, mut h) = harness(100, 22);
        h.set_locale(locale).send(Msg::Tab(Tab::Settings));
        h.click_text(if locale == "en" { "back to quvyta" } else { "quvyta'ya dön" }).advance(TOAST_IN);
        shot(&h, format!("settings select open {locale}"));

        // The whole page at once: the rows the family shares with their boxes, then quvyta's own.
        for (width, height) in [(100, 44), (48, 48)] {
            let root = tempfile::tempdir().expect("temp");
            let mut h = shared_settings(root.path());
            h.resize(width, height);
            h.set_locale(locale).render();
            shot(&h, format!("settings appearance {locale} {width}x{height}"));
        }
        let root = tempfile::tempdir().expect("temp");
        let mut h = shared_settings(root.path());
        h.set_locale(locale);
        h.click_text(if locale == "en" { "Theme" } else { "Renk teması" });
        h.press("down").press("space").render();
        shot(&h, format!("settings appearance only here {locale}"));

        let root = tempfile::tempdir().expect("temp");
        let mut h = shared_settings(root.path());
        h.set_locale(locale);
        h.click_text(if locale == "en" { "English" } else { "Türkçe" }).advance(TOAST_IN);
        shot(&h, format!("settings languages open {locale}"));

        let (root, mut h) = harness_with_settings("after_close = \"return\"\n");
        let config = root.path().join("config");
        fs::set_permissions(&config, fs::Permissions::from_mode(0o555)).expect("read-only");
        h.set_locale(locale).send(Msg::Tab(Tab::Settings));
        h.send(Msg::Setting(SettingMsg::Change(Change::CheckUpdates(false)))).render().advance(TOAST_IN);
        fs::set_permissions(&config, fs::Permissions::from_mode(0o755)).expect("writable again");
        shot(&h, format!("settings not saved {locale}"));
    }
    fragments
}
