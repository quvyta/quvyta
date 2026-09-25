use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use qframe::i18n::I18n;
use qframe::icons::GlyphMode;
use qframe::runtime::{HandoffOutcome, Harness};
use qframe::storage::{Family, Source};

use super::super::tests::{
    LANGUAGES, TOAST_IN, env, harness, harness_with_settings, harness_with_settings_at, line_with, machine,
    machine_off_path,
};
use super::*;
use crate::app::{PathMsg, Tab};
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

/// What the shared file holds now.
fn quvyta_conf(root: &Path) -> String {
    fs::read_to_string(root.join("config/quvyta.conf")).expect("written")
}

/// The appearance every Quvyta app shares, as the shared file holds it before a test starts. Written by
/// hand rather than detected, so the rows start from the same values on every machine.
const SHARED: &str = "language = \"en\"\ntheme = \"monochrome\"\nicons = \"unicode\"\n";

/// The machine of [`machine`] whose shared folder already holds the shared file `shared` and, when
/// `own` is not empty, quvyta's own `launcher.conf`.
fn shared_files(root: &Path, shared: &str, own: &str) -> Machine {
    fs::create_dir_all(root.join("config")).expect("folder");
    fs::write(root.join("config/quvyta.conf"), shared).expect("shared file");
    if !own.is_empty() {
        fs::write(root.join("config/launcher.conf"), own).expect("launcher.conf");
    }
    machine(root)
}

/// The Settings tab on a screen tall enough for both sections, with the shared file in place.
fn shared_settings(root: &Path) -> Harness<Quvyta> {
    settings_on(shared_files(root, SHARED, ""), 100, 44)
}

/// Where `text` starts on `screen`.
fn at(screen: &str, text: &str) -> usize {
    screen.find(text).unwrap_or_else(|| panic!("`{text}` is missing:\n{screen}"))
}

#[test]
fn the_appearance_every_app_shares_stands_above_quvyta_s_own_settings() {
    let root = tempfile::tempdir().expect("temp");
    let h = shared_settings(root.path());
    let screen = h.screen();
    for text in ["Appearance", "Language", "Theme", "Icons", "Reduce motion", "Pillar"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert_eq!(screen.matches("In every Quvyta application").count(), 4, "one box under each shared row:\n{screen}");
    assert!(at(&screen, "Appearance") < at(&screen, "launcher.conf"), "the shared rows come first:\n{screen}");
    assert!(at(&screen, "Pillar") < at(&screen, "Say when an update is out"), "{screen}");
    assert!(
        at(&screen, "Say when an update is out") < at(&screen, "launcher.conf"),
        "the shared switch is its own:\n{screen}"
    );
    // quvyta writes no appearance row of its own: the four shared rows and their boxes are the only ones.
    // The follow table below names the same keys over its columns.
    let appearance = &screen[..at(&screen, "Do the applications follow the shared settings")];
    assert_eq!(appearance.matches("Language").count(), 1, "{screen}");
    let prefs = h.app().preferences();
    assert_eq!(prefs.theme().source, Source::Family, "quvyta follows the shared values it shows");
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
    // Another Quvyta app reads the same theme from the shared file.
    let folder = root.path().join("config");
    let code = Family::QUVYTA.preferences_in(&folder, "code", &I18n::builtin());
    assert_eq!(code.theme().value, "nordic");
}

#[test]
fn clearing_the_box_keeps_the_choice_to_quvyta_and_leaves_the_shared_file_alone() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = settings_on(shared_files(root.path(), SHARED, "after_close = \"shell\"\n"), 100, 44);
    // The box under the theme row: the keys reach it from the row above, since all four boxes
    // read the same.
    h.click_text("Theme");
    h.press("down").press("space").render();
    assert_eq!(h.app().preferences().theme().source, Source::App, "only here now");
    h.send(Msg::Setting(SettingMsg::Appearance(AppearanceChange::Theme("iris".to_owned()))));
    assert_eq!(h.env().theme().id(), "iris");
    let own = launcher_conf(root.path());
    assert!(own.contains("theme = \"iris\"") && own.contains("after_close = \"shell\""), "{own}");
    assert!(quvyta_conf(root.path()).contains("theme = \"monochrome\""), "the shared file keeps its theme");

    // Checking it again hands the value to the shared file and quvyta follows it once more.
    h.send(Msg::Setting(SettingMsg::Appearance(AppearanceChange::Everywhere(qframe::storage::Shared::Theme, true))));
    assert!(quvyta_conf(root.path()).contains("theme = \"iris\""), "{}", quvyta_conf(root.path()));
    assert!(launcher_conf(root.path()).contains("theme = \"quvyta\""), "{}", launcher_conf(root.path()));
}

#[test]
fn quvyta_in_its_own_file_follows_the_shared_value_and_its_own_value_does_not() {
    let root = tempfile::tempdir().expect("temp");
    let shared = "language = \"tr\"\ntheme = \"nordic\"\nicons = \"unicode\"\n";
    let following = shared_files(root.path(), shared, "theme = \"quvyta\"\nafter_close = \"shell\"\n");
    let h = settings_on(following, 100, 44);
    let prefs = h.app().preferences();
    assert_eq!((prefs.theme().value.as_str(), prefs.theme().source), ("nordic", Source::Family));
    assert_eq!(prefs.language().value, "tr", "the shared language too");
    assert_eq!(h.app().launcher.after_close, AfterClose::Shell, "the rows of launcher.conf are read as before");
    assert_eq!(h.app().settings().diagnostics(), [], "`quvyta` is a value its own file may hold");
    let screen = h.screen();
    assert_eq!(screen.matches("In every Quvyta application").count(), 4, "{screen}");

    let own = shared_files(root.path(), shared, "theme = \"amber\"\n");
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
    let (_root, mut h) = harness(100, 40);
    h.press("down");
    h.click_text("Settings");
    let screen = h.screen();
    for text in ["Say when an update is out", "When an app closes", "back to quvyta", "~/.cargo/bin on PATH"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("qtools") && !screen.contains("Quvyta Focus"), "{screen}");
    h.click_text("Apps");
    let screen = h.screen();
    assert!(line_with(&screen, "qfocus").contains('▌'), "the selection is kept:\n{screen}");
    assert!(!screen.contains("Say when an update is out"), "{screen}");
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
    assert!(h.screen().contains("Say when an update is out"), "{}", h.screen());
    assert!(h.is_focused("tabs"), "the keys stay on the tabs");
    h.press("left");
    assert!(h.screen().contains("Quvyta Code"), "{}", h.screen());

    h.press("right").press("tab");
    assert!(h.is_focused("page"), "tab goes on to the page, which scrolls");
    h.press("tab");
    assert!(h.is_focused("appearance"), "and then to the appearance every app shares");
    h.press("tab");
    assert!(h.is_focused("following-table"), "and then to the table of who follows the shared values");
    h.press("tab");
    assert!(h.is_focused("settings"), "and then to quvyta's own settings");
    h.press("esc");
    assert!(h.screen().contains("Quvyta Code"), "esc goes back to the app list:\n{}", h.screen());
    assert!(h.is_focused("apps"), "and gives the list the keys");
}

#[test]
fn the_settings_hints_leave_out_the_app_list_s_keys_and_those_keys_do_nothing() {
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
fn the_update_notice_is_one_shared_switch_written_to_the_shared_file() {
    let root = tempfile::tempdir().expect("temp");
    let own = "after_close = \"shell\"\npath_prompt = \"dismissed\"\n";
    let mut h = settings_on(shared_files(root.path(), SHARED, own), 100, 44);
    let screen = h.screen();
    assert!(screen.contains("For all Quvyta applications at once"), "in the framework's words:\n{screen}");
    assert!(!screen.contains("Check for updates at start"), "one switch, not quvyta's own beside it:\n{screen}");
    assert!(at(&screen, "Say when an update is out") < at(&screen, "When an app closes"), "{screen}");
    h.click_text("Say when an update is out");
    assert!(h.is_focused("appearance"), "a click on a label gives the list the keys");
    h.press("space").render();
    assert!(quvyta_conf(root.path()).contains("update-notice = false"), "{}", quvyta_conf(root.path()));
    assert!(!h.app().preferences().update_notice(), "the running quvyta has it at once");
    assert_eq!(launcher_conf(root.path()), own, "quvyta's own file is not where it goes");
}

#[test]
fn choosing_quit_to_shell_writes_it_and_the_next_close_leaves_with_the_member() {
    let (root, mut h) = harness(100, 40);
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
    // Chosen where a person chooses it: the select, then its other choice.
    h.click_text("back to quvyta").advance(TOAST_IN);
    h.click_text("quit to shell");
    h.render().advance(TOAST_IN);
    let screen = h.screen();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o755)).expect("writable again");
    assert_eq!(h.app().launcher.after_close, AfterClose::Return, "the choice goes back");
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
    let h = settings_on(machine(root.path()), 100, 40);
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
    h.click_text("When an app closes");
    h.press("down").press("enter").render();
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
        "Güncelleme çıkınca haber ver",
        "Uygulama kapanınca",
        "quvyta'ya dön",
        "~/.cargo/bin PATH'te",
        "hayır",
        "Ekle",
        "uygulamalar",
    ] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    let (_root, mut h) = harness(100, 40);
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
            let mut h = settings_on(machine, width, 60);
            h.set_locale(locale);
            h.click_text(if locale == "en" { "Not now" } else { "Şimdi değil" });
            let screen = h.screen();
            let body: Vec<&str> = screen.lines().skip(1).collect();
            assert!(!body.iter().any(|line| line.contains('…')), "{locale} at {width}:\n{screen}");
            let title = body.iter().position(|line| line.trim() == "quvyta").expect("the title alone");
            assert!(body[title + 1].contains("launcher.conf"), "{locale} at {width}:\n{screen}");
        }
    }
    let (_root, h) = harness(100, 40);
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
        for locale in LANGUAGES {
            let root = tempfile::tempdir().expect("temp");
            let mut machine = machine_off_path(root.path());
            machine.shell = Some("/bin/bash".to_owned());
            let mut h = settings_on(machine, width, height);
            h.set_locale(locale).set_glyph_mode(GlyphMode::Ascii);
            let screen = h.screen();
            assert!(screen.contains("quvyta"), "{width}x{height}:\n{screen}");
            // The heading of the shared rows comes from the framework, in this
            // language, so the test asks it rather than spelling it out nine times.
            let heading = h.env().i18n().translate("quvyta.appearance.heading", &[]);
            assert!(screen.contains(&heading), "the shared rows at {width}x{height} {locale}:\n{screen}");
            for forbidden in ['[', ']', '{', '}', '|', '▌', '⟦'] {
                assert!(!screen.contains(forbidden), "`{forbidden}` at {width}x{height} {locale}:\n{screen}");
            }
        }
    }
}

/// The Settings tab for the visual review: both languages, wide, narrow and short, the whole page
/// with the appearance every app shares, a box cleared, an open language list, with cargo's
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

        // The whole page at once: the shared rows with their boxes, then quvyta's own.
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

        // Who follows the shared values: an own value, a member never opened and a broken file.
        for (width, height) in [(100, 60), (48, 60)] {
            let root = tempfile::tempdir().expect("temp");
            let mut h = settings_on(member_files(root.path(), &[("code", QCODE), ("showcase", BROKEN)]), width, height);
            h.set_locale(locale).advance(TOAST_IN);
            shot(&h, format!("settings following {locale} {width}x{height}"));
        }

        let (root, mut h) = harness_with_settings("after_close = \"return\"\n");
        let config = root.path().join("config");
        fs::set_permissions(&config, fs::Permissions::from_mode(0o555)).expect("read-only");
        h.set_locale(locale).send(Msg::Tab(Tab::Settings));
        h.render();
        h.click_text(if locale == "en" { "back to quvyta" } else { "quvyta'ya dön" }).advance(TOAST_IN);
        h.click_text(if locale == "en" { "quit to shell" } else { "kabuğa çık" }).render().advance(TOAST_IN);
        fs::set_permissions(&config, fs::Permissions::from_mode(0o755)).expect("writable again");
        shot(&h, format!("settings not saved {locale}"));
    }
    fragments
}

/// Nothing on the Settings tab is cut in any language: this page carries the rows every Quvyta
/// app shows, so a word that does not fit here is a word that does not fit anywhere.
///
/// All three states of the PATH row are walked, because its value is the one line on this page
/// that may not wrap: a `yes` that broke over two lines would look like a second setting.
///
/// Two lines shorten themselves on purpose and are let through by name: the path of the
/// settings file and the `PATH` line a shell would be given. The language select, which once cut
/// `Português (Brasil)`, has no exception: the framework's select fits it now.
#[test]
fn nothing_on_the_settings_tab_is_cut_in_any_language() {
    for (width, height) in [(40, 30), (48, 40), (60, 40), (80, 44), (100, 44)] {
        for locale in LANGUAGES {
            for reach in ["missing", "new-terminals", "here"] {
                let root = tempfile::tempdir().expect("temp");
                let mut machine = machine_off_path(root.path());
                machine.shell = Some("/bin/bash".to_owned());
                match reach {
                    // The folder is on `PATH` right now.
                    "here" => machine.path.push(machine.cargo_bin()),
                    // The shell file already carries the line, so it will be on `PATH` in the
                    // next terminal: the state a person is in after Add, set the way their
                    // machine would be rather than by pressing a button that a short screen
                    // does not even show.
                    "new-terminals" => {
                        fs::create_dir_all(root.path().join("home")).expect("home");
                        fs::write(root.path().join("home/.bashrc"), "export PATH=\"$HOME/.cargo/bin:$PATH\"\n")
                            .expect("bashrc");
                    }
                    _ => {}
                }
                let mut h = settings_on(machine, width, height);
                h.set_locale(locale);
                let screen = h.screen();
                for line in screen.lines() {
                    if !line.contains('\u{2026}') {
                        continue;
                    }
                    let shortens_itself = line.ends_with("launcher.conf") || line.contains("export PATH=");
                    assert!(
                        shortens_itself,
                        "`{}` is cut in {locale} at {width}x{height} with PATH {reach}:\n{screen}",
                        line.trim()
                    );
                }
            }
        }
    }
    // The follow table with the longest language name, every key a member's own, and a broken
    // file, on a screen tall enough to hold the whole page.
    let own = "language = \"pt-BR\"\ntheme = \"nordic\"\nicons = \"ascii\"\nreduced-motion = true\n";
    for width in [40, 48, 60, 80, 100, 120] {
        for locale in LANGUAGES {
            let root = tempfile::tempdir().expect("temp");
            let machine =
                member_files(root.path(), &[("code", own), ("focus", "language = \"ru\"\n"), ("showcase", BROKEN)]);
            let mut h = settings_on(machine, width, 90);
            h.set_locale(locale);
            let i18n = h.env().i18n();
            let screen = h.screen();
            // The title's start: a narrow screen wraps the rest, in scripts without spaces too.
            let title: String = i18n.translate("settings.follow-title", &[]).chars().take(6).collect();
            assert!(screen.contains(&title), "the follow section is on screen in {locale} at {width}:\n{screen}");
            let words = [
                "Português".to_owned(),
                "Nordic".to_owned(),
                "Русский".to_owned(),
                i18n.translate("quvyta.appearance.icons-ascii", &[]),
                i18n.translate("settings.motion-reduced", &[]),
                i18n.translate("settings.follow-unreadable", &[]),
            ];
            for word in words {
                assert!(screen.contains(&word), "`{word}` in {locale} at {width}:\n{screen}");
            }
            for line in screen.lines().filter(|line| line.contains('\u{2026}')) {
                assert!(line.ends_with("launcher.conf"), "`{}` is cut in {locale} at {width}:\n{screen}", line.trim());
            }
        }
    }
}

/// The machine of [`shared_files`] with the settings files of members, each an id and its text.
fn member_files(root: &Path, files: &[(&str, &str)]) -> Machine {
    let machine = shared_files(root, SHARED, "");
    for (id, text) in files {
        fs::write(root.join(format!("config/{id}.conf")), text).expect("member file");
    }
    machine
}

/// qcode names its own language and theme and shares its icons, qfocus has never been opened and
/// the showcase's file is broken: the three members the test machine has installed.
const QCODE: &str = "language = \"tr\"\ntheme = \"nordic\"\nreduced_motion = true\n";
const BROKEN: &str = "theme = \"amber\nlanguage = \n";

/// The screen row holding the follow table's row of `command`: after the section's title, so a
/// word elsewhere on the page is never taken for it.
fn follow_line(screen: &str, command: &str) -> String {
    // The title's start, since a narrow screen wraps the rest of it.
    let title = at(screen, "Do the applications follow");
    line_with(&screen[title..], &format!("{command} ")).to_owned()
}

#[test]
fn the_follow_table_shows_each_installed_member_s_own_values_and_what_it_shares() {
    let root = tempfile::tempdir().expect("temp");
    let machine = member_files(root.path(), &[("code", QCODE), ("showcase", BROKEN)]);
    let h = settings_on(machine, 100, 60);
    let screen = h.screen();
    let qcode = follow_line(&screen, "qcode");
    // A language in its own name and a theme's name, as the appearance rows name them.
    assert!(qcode.contains("Türkçe") && qcode.contains("Nordic") && qcode.contains("shared"), "{screen}");
    assert!(at(&qcode, "Türkçe") < at(&qcode, "Nordic") && at(&qcode, "Nordic") < at(&qcode, "shared"), "{qcode}");
    assert!(follow_line(&screen, "qfocus").contains("not opened yet"), "{screen}");
    assert_eq!(follow_line(&screen, "qframe").matches("unreadable").count(), 4, "{screen}");
    // Only what is installed, and quvyta's own following is the boxes above.
    let note = "Open applications see a change the next time they start.";
    let section = &screen[at(&screen, "Do the applications follow")..at(&screen, note)];
    for absent in ["qtools", "qpac", "qdesk", "quvyta"] {
        assert!(!section.contains(absent), "`{absent}` is listed:\n{section}");
    }
    // Shape by tone: the member's own value in the row's own colour, as its name is, and what it
    // shares faint.
    let muted = h.env().theme().color("muted");
    // The colour `text` is drawn in on the follow table's row of `command`.
    let tone = |command: &str, text: &str| {
        let row = follow_line(&screen, command);
        let y = screen.lines().position(|line| line == row).expect("the row is on screen");
        let x = row.find(text).expect("in the row");
        let x = qframe::text::width(&row[..x]);
        h.fg(x, u16::try_from(y).expect("y"))
    };
    assert_eq!(tone("qcode", "Nordic"), tone("qcode", "qcode"), "{screen}");
    assert_ne!(tone("qcode", "Nordic"), muted, "{screen}");
    for (command, faint) in [("qcode", "shared"), ("qfocus", "not opened yet"), ("qframe", "unreadable")] {
        assert_eq!(tone(command, faint), muted, "`{faint}`:\n{screen}");
    }
    assert!(h.handoffs().is_empty(), "nothing reaches the desktop");
}

#[test]
fn a_member_s_own_reduced_motion_has_a_column_of_its_own_and_following_it_puts_that_key_back() {
    let root = tempfile::tempdir().expect("temp");
    let focus = "reduced-motion = true\ntheme = \"amber\"\n";
    let code = "reduced-motion = false\n";
    let mut h = settings_on(member_files(root.path(), &[("focus", focus), ("code", code)]), 100, 60);
    let screen = h.screen();
    let header = line_with(&screen[at(&screen, "Do the applications follow")..], "Language").to_owned();
    let column = at(&header, "Motion");
    let qfocus = follow_line(&screen, "qfocus");
    assert_eq!(qfocus.find("reduced"), Some(column), "under its column:\n{screen}");
    assert_eq!(follow_line(&screen, "qcode").find("full"), Some(column), "off is its own value too:\n{screen}");
    click_follow_row(&mut h, "qfocus");
    h.click_text("Follow the shared motion setting").advance(TOAST_IN);
    let own = member_conf(root.path(), "focus");
    assert!(own.contains("reduced-motion = \"quvyta\"") && own.contains("theme = \"amber\""), "{own}");
    let qfocus = follow_line(&h.screen(), "qfocus");
    assert!(!qfocus.contains("reduced") && qfocus.contains("Amber"), "{}", h.screen());
    assert!(h.handoffs().is_empty());
}

#[test]
fn a_broken_member_file_is_told_once_and_left_exactly_as_it_was() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = settings_on(member_files(root.path(), &[("showcase", BROKEN)]), 100, 60);
    h.advance(TOAST_IN);
    let screen = h.screen();
    assert!(screen.contains("An application's settings could not be read"), "{screen}");
    assert!(screen.contains("showcase.conf:1:"), "the reason is located:\n{screen}");
    // Back and forth between the tabs reads the files again but tells the same reason once.
    h.press("esc");
    h.click_text("Settings").advance(TOAST_IN);
    h.advance(std::time::Duration::from_secs(30));
    h.press("esc");
    h.click_text("Settings").advance(TOAST_IN);
    assert!(!h.screen().contains("could not be read"), "told again:\n{}", h.screen());
    assert_eq!(fs::read_to_string(root.path().join("config/showcase.conf")).expect("read"), BROKEN);
    let mut names: Vec<String> = fs::read_dir(root.path().join("config"))
        .expect("list")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["launcher.conf", "quvyta.conf", "showcase.conf"], "no backup, nothing new");
}

#[test]
fn a_file_a_member_wrote_meanwhile_is_read_again_when_the_settings_tab_opens() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = settings_on(member_files(root.path(), &[]), 100, 60);
    assert!(follow_line(&h.screen(), "qfocus").contains("not opened yet"), "{}", h.screen());
    h.press("esc");
    fs::write(root.path().join("config/focus.conf"), "icons = \"ascii\"\n").expect("qfocus's first start");
    h.click_text("Settings");
    let line = follow_line(&h.screen(), "qfocus");
    assert!(line.contains("ASCII") && line.matches("shared").count() == 3, "{}", h.screen());
}

#[test]
fn with_no_other_member_installed_one_line_says_so() {
    let root = tempfile::tempdir().expect("temp");
    super::super::tests::set_up(root.path());
    let machine = crate::inventory::tests::machine_with_cargo(root.path(), "", 0);
    let h = settings_on(machine, 100, 60);
    let screen = h.screen();
    let title = at(&screen, "Do the applications follow the shared settings");
    let next: Vec<&str> = screen[title..].lines().skip(1).take(2).collect();
    assert_eq!(next[0].trim(), "No other Quvyta application is installed.", "{screen}");
    assert_eq!(next[1].trim(), "", "and nothing more:\n{screen}");
    assert!(!screen.contains("next time they start"), "{screen}");
}

#[test]
fn a_narrow_screen_gives_each_member_one_line_with_only_what_it_does_not_share() {
    let root = tempfile::tempdir().expect("temp");
    let machine = member_files(root.path(), &[("code", QCODE), ("showcase", "theme = \"quvyta\"\n")]);
    let mut h = settings_on(machine, 60, 60);
    let screen = h.screen();
    assert_eq!(follow_line(&screen, "qcode").trim(), "qcode   Language: Türkçe  Theme: Nordic", "{screen}");
    assert_eq!(follow_line(&screen, "qfocus").trim(), "qfocus  not opened yet", "{screen}");
    assert_eq!(follow_line(&screen, "qframe").trim(), "qframe  all shared", "{screen}");
    h.set_locale("tr");
    let screen = h.screen();
    let title = at(&screen, "Uygulamalar ortak ayarı izliyor mu");
    assert_eq!(line_with(&screen[title..], "qcode").trim(), "qcode   Dil: Türkçe  Renk teması: Nordic", "{screen}");
    assert_eq!(line_with(&screen[title..], "qframe").trim(), "qframe  hepsi ortak", "{screen}");

    // Keys that do not fit on one line each take a line, rather than a value breaking in two.
    let root = tempfile::tempdir().expect("temp");
    let long = "language = \"ru\"\ntheme = \"nordic\"\n";
    let h = settings_on(member_files(root.path(), &[("code", long)]), 40, 60);
    let screen = h.screen();
    let first = follow_line(&screen, "qcode");
    assert_eq!(first.trim(), "qcode   Language: Русский", "{screen}");
    let next = screen.lines().skip_while(|line| *line != first).nth(1).expect("a line after");
    assert_eq!(next.trim(), "Theme: Nordic", "{screen}");
}

/// Clicks the follow table's row of `command` on its name, where a person would.
fn click_follow_row(h: &mut Harness<Quvyta>, command: &str) {
    let screen = h.screen();
    let row = follow_line(&screen, command);
    let y = screen.lines().position(|line| line == row).expect("the row is on screen");
    let x = qframe::text::width(&row[..row.find(command).expect("in the row")]);
    h.click(i32::from(x), i32::try_from(y).expect("y")).advance(TOAST_IN);
}

fn member_conf(root: &Path, id: &str) -> String {
    fs::read_to_string(root.join(format!("config/{id}.conf"))).expect("the member's file")
}

#[test]
fn a_click_on_a_member_offers_its_own_keys_and_following_changes_that_key_alone() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = settings_on(member_files(root.path(), &[("code", QCODE)]), 100, 60);
    let shared_before = quvyta_conf(root.path());
    click_follow_row(&mut h, "qcode");
    let screen = h.screen();
    assert!(screen.contains("Follow the shared language") && screen.contains("Follow the shared theme"), "{screen}");
    assert!(!screen.contains("Follow the shared icons"), "qcode shares its icons already:\n{screen}");
    h.click_text("Follow the shared theme").advance(TOAST_IN);
    let own = member_conf(root.path(), "code");
    assert!(own.contains("theme = \"quvyta\""), "{own}");
    assert!(own.contains("language = \"tr\"") && own.contains("reduced_motion = true"), "the rest stays:\n{own}");
    assert_eq!(quvyta_conf(root.path()), shared_before, "the shared file is not touched");
    let qcode = follow_line(&h.screen(), "qcode");
    assert!(qcode.contains("Türkçe") && !qcode.contains("Nordic"), "the table reads the file again:\n{}", h.screen());
    assert_eq!(qcode.matches("shared").count(), 3, "{qcode}");
    assert!(h.handoffs().is_empty());
}

#[test]
fn enter_on_a_member_opens_the_same_choices() {
    let root = tempfile::tempdir().expect("temp");
    // qfocus this time, so a choice that reached the wrong member's file would show.
    let mut h = settings_on(member_files(root.path(), &[("focus", QCODE), ("code", QCODE)]), 100, 60);
    click_follow_row(&mut h, "qfocus");
    h.press("esc").advance(TOAST_IN);
    assert!(!h.screen().contains("Follow the shared theme"), "esc closes it:\n{}", h.screen());
    h.press("enter").advance(TOAST_IN);
    assert!(h.screen().contains("Follow the shared theme"), "{}", h.screen());
    h.press("down").press("enter").advance(TOAST_IN);
    assert!(member_conf(root.path(), "focus").contains("theme = \"quvyta\""), "{}", h.screen());
    assert!(member_conf(root.path(), "focus").contains("language = \"tr\""), "one key only");
    assert_eq!(member_conf(root.path(), "code"), QCODE, "and one member only");
}

#[test]
fn a_member_that_shares_everything_or_cannot_be_read_offers_nothing() {
    let root = tempfile::tempdir().expect("temp");
    let machine = member_files(root.path(), &[("code", "theme = \"quvyta\"\n"), ("showcase", BROKEN)]);
    let mut h = settings_on(machine, 100, 60);
    h.advance(TOAST_IN);
    for command in ["qcode", "qframe", "qfocus"] {
        click_follow_row(&mut h, command);
        assert!(!h.screen().contains("Follow the shared"), "{command} offers a choice:\n{}", h.screen());
    }
    assert_eq!(member_conf(root.path(), "showcase"), BROKEN, "left exactly as it was");
    assert!(!root.path().join("config/focus.conf").exists(), "no file is made for a member never opened");
}

#[test]
fn a_narrow_screen_puts_the_choices_under_the_member_as_buttons() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = settings_on(member_files(root.path(), &[("code", QCODE)]), 60, 60);
    let screen = h.screen();
    let choices = following_choices(&screen);
    assert_eq!(
        choices.split_whitespace().collect::<Vec<_>>(),
        ["Back", "to", "shared:", "Language", "Theme"],
        "{screen}"
    );
    // On the line of the choices, not the appearance row of the same name above.
    let y = screen.lines().position(|line| line == choices).expect("on screen");
    let x = qframe::text::width(&choices[..choices.find("Language").expect("the button")]);
    h.click(i32::from(x), i32::try_from(y).expect("y")).advance(TOAST_IN);
    let own = member_conf(root.path(), "code");
    assert!(own.contains("language = \"quvyta\"") && own.contains("theme = \"nordic\""), "{own}");
    let screen = h.screen();
    assert_eq!(
        following_choices(&screen).split_whitespace().last(),
        Some("Theme"),
        "only what is still its own:\n{screen}"
    );
    assert_eq!(follow_line(&screen, "qcode").trim(), "qcode   Theme: Nordic", "{screen}");
}

#[test]
fn a_member_file_that_cannot_be_written_says_so_and_stays_as_it_was() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = settings_on(member_files(root.path(), &[("code", QCODE)]), 100, 60);
    let config = root.path().join("config");
    click_follow_row(&mut h, "qcode");
    fs::set_permissions(&config, fs::Permissions::from_mode(0o555)).expect("read-only");
    h.click_text("Follow the shared theme").advance(TOAST_IN);
    let screen = h.screen();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o755)).expect("writable again");
    assert!(screen.contains("The setting could not be saved"), "{screen}");
    assert_eq!(member_conf(root.path(), "code"), QCODE);
    assert!(follow_line(&screen, "qcode").contains("Nordic"), "{screen}");
}

/// The line of the narrow follow section offering the choices of its only member with any.
fn following_choices(screen: &str) -> String {
    line_with(&screen[at(screen, "Do the applications follow")..], "Back to shared:").to_owned()
}
