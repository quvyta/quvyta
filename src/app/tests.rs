use std::path::Path;

use qframe::env::{AssetDirs, Env};
use qframe::icons::GlyphMode;
use tempfile::TempDir;

use super::*;
use crate::ecosystem::Status;
use crate::inventory::tests::{installed_command, machine_with_cargo, write_program};

/// Built-in files plus the compiled-in locales and keys, as the runtime loads them.
pub(super) fn env() -> Env {
    let dirs = AssetDirs {
        locale_sources: crate::LOCALES.iter().map(|(file, text)| ((*file).to_owned(), (*text).to_owned())).collect(),
        keymap_source: Some(("keymap.toml".to_owned(), crate::KEYS.to_owned())),
        ..AssetDirs::default()
    };
    Env::load(&dirs).expect("the locales are readable")
}

/// What cargo answers on the machine of [`machine`].
pub(super) const LIST: &str = "\
quvyta-code v0.1.1:
    qcode
    quvyta-code
quvyta-framework-showcase v0.1.4:
    qframe
    quvyta-framework-showcase
";

/// A machine in `root` where cargo installed qcode 0.1.1 and qframe 0.1.4, qfocus is on `PATH`
/// from elsewhere, and qtools and qpac are missing. It has a C linker, so installs can start.
/// Cargo's folder is on `PATH` too, so the PATH notice has nothing to say.
pub(super) fn machine(root: &Path) -> Machine {
    let mut machine = machine_off_path(root);
    machine.path.push(machine.cargo_bin());
    machine
}

/// The machine of [`machine`] with cargo's folder missing from `PATH`, and no `SHELL`.
pub(super) fn machine_off_path(root: &Path) -> Machine {
    set_up(root);
    let machine = machine_with_cargo(root, LIST, 0);
    write_program(&root.join("bin/cc"), "#!/bin/sh\nexit 0\n");
    installed_command(&machine, "qcode");
    installed_command(&machine, "qframe");
    write_program(&root.join("bin/qfocus"), "#!/bin/sh\nexit 0\n");
    machine
}

/// The application on that machine, with what is installed already read. The folder lives as
/// long as the returned guard.
pub(super) fn harness(width: u16, height: u16) -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    let h = start(root.path(), width, height);
    (root, h)
}

/// Makes `root` a machine quvyta has been set up on: an empty `launcher.conf` is enough, since
/// the first-run wizard opens only while quvyta has no file of its own. Without this every test
/// of the app list would see the wizard, which is [`super::wizard::tests`]' own subject.
pub(super) fn set_up(root: &Path) {
    let conf = root.join("config/launcher.conf");
    if !conf.exists() {
        std::fs::create_dir_all(root.join("config")).expect("folder");
        std::fs::write(&conf, "").expect("settings");
    }
}

/// The application on the machine in `root`, which may already hold a `config/launcher.conf`.
pub(super) fn start(root: &Path, width: u16, height: u16) -> Harness<Quvyta> {
    let app = Quvyta::new(machine(root));
    let mut h = Harness::with_env(app, env(), width, height);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    // The first frame asked cargo in the background; this frame hears the answer.
    h.render();
    h
}

/// The application with `launcher.conf` holding `settings`.
pub(super) fn harness_with_settings(settings: &str) -> (TempDir, Harness<Quvyta>) {
    harness_with_settings_at(settings, 100, 24)
}

/// [`harness_with_settings`] on a screen of `width` by `height`, for the whole Settings tab,
/// which is longer than a screen of the usual height.
pub(super) fn harness_with_settings_at(settings: &str, width: u16, height: u16) -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    std::fs::create_dir_all(root.path().join("config")).expect("folder");
    std::fs::write(root.path().join("config/launcher.conf"), settings).expect("settings");
    let h = start(root.path(), width, height);
    (root, h)
}

pub(super) fn index(key: &str) -> usize {
    APPS.iter().position(|member| member.key == key).expect("a member")
}

/// Long enough for a toast to have slid in.
pub(super) const TOAST_IN: std::time::Duration = std::time::Duration::from_secs(1);

/// The first screen row under the header holding `text`.
pub(super) fn line_with<'a>(screen: &'a str, text: &str) -> &'a str {
    screen.lines().skip(1).find(|line| line.contains(text)).unwrap_or_else(|| panic!("`{text}` is missing:\n{screen}"))
}

/// Every language quvyta speaks, English first: the sweeps that are cheap walk all of them.
pub(in crate::app) const LANGUAGES: [&str; 9] = ["en", "tr", "de", "es", "fr", "pt-BR", "ru", "zh-Hans", "ja"];

/// What the expensive sweeps walk instead: English, the longest of the Latin languages, the
/// Cyrillic one and the one whose characters take two cells. Between them they catch a label
/// that is cut on a narrow screen and a script that is measured wrongly.
pub(in crate::app) const LONGEST_LANGUAGES: [&str; 4] = ["en", "de", "ru", "ja"];

#[test]
fn the_language_files_load_without_problems_and_every_language_is_complete() {
    let env = env();
    assert_eq!(env.diagnostics(), &[]);
    for locale in LANGUAGES {
        assert_eq!(env.i18n().missing_keys(locale, "en"), Vec::<String>::new(), "keys missing in `{locale}`");
    }
    let known: Vec<String> = env.i18n().list().into_iter().map(|(code, _)| code).collect();
    for locale in LANGUAGES {
        assert!(known.iter().any(|code| code == locale), "`{locale}` is not among the loaded languages: {known:?}");
    }
}

#[test]
fn the_list_shows_every_program_with_how_it_is_installed() {
    let (_root, h) = harness(100, 24);
    let screen = h.screen();
    let version = env!("CARGO_PKG_VERSION");
    for (command, state) in [
        ("qcode", "0.1.1".to_owned()),
        ("qfocus", "unknown version".to_owned()),
        ("qtools", "not installed".to_owned()),
        ("qpac", "not installed".to_owned()),
        ("qdesk", "coming soon".to_owned()),
        ("qframe", "0.1.4".to_owned()),
        ("quvyta", format!("{version}  this application")),
    ] {
        let line = line_with(&screen, &format!("{command} "));
        assert!(line.contains(&state), "`{command}` should say `{state}`:\n{screen}");
    }
    let rows: Vec<&str> = screen.lines().skip(1).collect();
    let order: Vec<usize> = ["qcode ", "qfocus ", "qtools ", "qpac ", "qdesk ", "qframe ", "quvyta "]
        .iter()
        .map(|command| rows.iter().position(|row| row.contains(command)).expect("listed"))
        .collect();
    assert!(order.is_sorted(), "the list keeps the order of APPS:\n{screen}");
}

#[test]
fn the_selected_row_carries_the_pillar() {
    let (_root, mut h) = harness(100, 24);
    assert!(line_with(&h.screen(), "qcode").contains('▌'), "{}", h.screen());
    h.press("down");
    assert!(line_with(&h.screen(), "qfocus").contains('▌'), "{}", h.screen());
    assert!(!line_with(&h.screen(), "qcode ").contains('▌'), "{}", h.screen());
}

#[test]
fn an_installed_member_says_which_version_and_where() {
    let (_root, h) = harness(100, 24);
    let screen = h.screen();
    for text in ["Quvyta Code", "Beta", "quvyta-code", "Installed  0.1.1, ~/.cargo/bin/qcode", "Source", "A beta"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("cargo install quvyta-code"), "an installed member needs no install line:\n{screen}");
}

#[test]
fn the_keyboard_moves_the_details_along_the_list() {
    let (root, mut h) = harness(100, 24);
    assert!(h.is_focused("apps"), "the list has the keys from the start");
    h.press("down");
    let screen = h.screen();
    let path = root.path().join("bin/qfocus").display().to_string();
    for text in ["Quvyta Focus", &format!("Installed  unknown version, {path}"), "Installed without cargo"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    h.press("down");
    let screen = h.screen();
    for text in ["Quvyta Tools", "Install"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
}

#[test]
fn a_click_on_a_row_selects_it_and_opens_nothing() {
    let (_root, mut h) = harness(100, 24);
    h.click_text("qframe");
    let screen = h.screen();
    for text in
        ["Quvyta Framework", "Released", "quvyta-framework-showcase", "Add to a project", "cargo add quvyta-framework"]
    {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("A beta"), "the framework is not a beta:\n{screen}");
    // A second click on the selected row is the double click that must not do anything.
    h.click_text("qframe");
    assert!(h.screen().contains("Quvyta Framework"));
}

#[test]
fn quvyta_itself_shows_the_running_version() {
    let (_root, mut h) = harness(100, 24);
    h.send(Msg::Select(index("quvyta")));
    let screen = h.screen();
    let running = format!("Running  {}", env!("CARGO_PKG_VERSION"));
    for text in ["Quvyta", &running, "https://github.com/quvyta/quvyta"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
}

#[test]
fn every_program_installs_under_its_package_name() {
    for member in APPS {
        assert!(member.repository.starts_with("https://github.com/quvyta/"), "{}", member.repository);
        let expected = match member.key {
            "framework" => Status::Released,
            "desk" => Status::Soon,
            _ => Status::Beta,
        };
        assert_eq!(member.status, expected, "{}", member.package);
    }
}

#[test]
fn an_unknown_index_changes_nothing() {
    let (_root, mut h) = harness(100, 24);
    h.send(Msg::Select(APPS.len())).send(Msg::ShowDetail(APPS.len()));
    assert!(h.screen().contains("Quvyta Code"));
}

#[test]
fn before_cargo_answers_the_rows_say_nothing_rather_than_guess() {
    let root = tempfile::tempdir().expect("temp");
    let h = Harness::with_env(Quvyta::new(machine(root.path())), env(), 100, 24);
    let screen = h.screen();
    assert!(!screen.contains("not installed") && !screen.contains("Installed"), "{screen}");
    assert!(screen.contains("qcode"), "{screen}");
}

#[test]
fn a_narrow_screen_shows_the_list_and_opens_details_on_their_own_page() {
    let (_root, mut h) = harness(48, 20);
    let screen = h.screen();
    assert!(screen.contains("qtools") && screen.contains("not installed"), "{screen}");
    assert!(!screen.contains("Quvyta Code"), "the details wait for enter:\n{screen}");
    assert!(screen.contains("details"), "the footer says what enter does:\n{screen}");

    h.press("down").press("enter");
    let screen = h.screen();
    assert!(screen.contains("Quvyta Focus") && !screen.contains("qtools"), "{screen}");
    assert!(screen.contains("Back"), "a mouse needs a way back too:\n{screen}");

    h.press("esc");
    let screen = h.screen();
    assert!(screen.contains("qtools") && !screen.contains("Quvyta Focus"), "{screen}");
    assert!(line_with(&screen, "qfocus").contains('▌'), "the selection is kept:\n{screen}");
    assert!(h.is_focused("apps"), "and the list has the keys again");
}

#[test]
fn on_a_narrow_screen_a_click_opens_the_details_and_back_returns() {
    let (_root, mut h) = harness(48, 20);
    h.click_text("qpac");
    assert!(h.screen().contains("Quvyta Packages"), "{}", h.screen());
    h.click_text("Back");
    assert!(h.screen().contains("qframe") && !h.screen().contains("Quvyta Packages"), "{}", h.screen());
}

#[test]
fn widening_the_screen_puts_the_details_beside_the_list() {
    let (_root, mut h) = harness(48, 20);
    h.press("enter");
    h.resize(100, 24);
    let screen = h.screen();
    assert!(screen.contains("qtools") && screen.contains("Quvyta Code"), "{screen}");
    assert!(!screen.contains("Back"), "{screen}");
}

#[test]
fn every_member_reads_fully_in_every_language() {
    for locale in LANGUAGES {
        let (_root, mut h) = harness(100, 30);
        h.set_locale(locale);
        for index in 0..APPS.len() {
            h.send(Msg::Select(index));
            let screen = h.screen();
            assert!(!screen.contains('⟦'), "a key is missing in `{locale}`:\n{screen}");
        }
    }
}

#[test]
fn turkish_uses_its_own_words() {
    let (_root, mut h) = harness(100, 24);
    h.set_locale("tr");
    let screen = h.screen();
    for text in ["Özenle yapılmış terminal uygulamaları", "kurulu değil", "sürümü bilinmiyor", "bu uygulama", "Kurulu"]
    {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    h.send(Msg::Select(index("framework")));
    for text in ["Yayında", "Projeye ekle", "Kaynak"] {
        assert!(h.screen().contains(text), "`{text}` is missing:\n{}", h.screen());
    }
}

/// A file that is loaded but never reaches the screen looks exactly like a working one to a
/// test that only counts keys: the English words come out either way. So each language is asked
/// here for three lines only it can produce — its own promise and its own two row states.
#[test]
fn every_language_shows_its_own_words() {
    let own_words = [
        ("en", ["Terminal applications, made with care", "not installed", "unknown version"]),
        ("tr", ["Özenle yapılmış terminal uygulamaları", "kurulu değil", "sürümü bilinmiyor"]),
        ("de", ["Terminal-Anwendungen, mit Sorgfalt gemacht", "nicht installiert", "Version unbekannt"]),
        ("es", ["Aplicaciones de terminal, hechas con cuidado", "no instalado", "versión desconocida"]),
        ("fr", ["Des applications en terminal, faites avec soin", "non installé", "version inconnue"]),
        ("pt-BR", ["Aplicativos de terminal, feitos com cuidado", "não instalado", "versão desconhecida"]),
        ("ru", ["Терминальные приложения, сделанные с заботой", "не установлено", "версия неизвестна"]),
        ("zh-Hans", ["用心打造的终端应用", "未安装", "版本未知"]),
        ("ja", ["ていねいに作られたターミナルアプリ", "未インストール", "バージョン不明"]),
    ];
    assert_eq!(own_words.len(), LANGUAGES.len(), "every language quvyta speaks is asked for its own words");
    for (locale, words) in own_words {
        assert!(LANGUAGES.contains(&locale), "`{locale}` is not one of the languages quvyta speaks");
        let (_root, mut h) = harness(100, 24);
        h.set_locale(locale);
        let screen = h.screen();
        for text in words {
            assert!(screen.contains(text), "`{locale}` should say `{text}`:\n{screen}");
        }
    }
}

#[test]
fn narrow_ascii_screens_keep_the_rules() {
    for (width, height) in [(40, 16), (48, 20), (60, 20), (100, 24)] {
        for locale in LONGEST_LANGUAGES {
            let (_root, mut h) = harness(width, height);
            h.set_glyph_mode(GlyphMode::Ascii).set_locale(locale);
            for index in 0..APPS.len() {
                h.send(Msg::Select(index));
                for page in [false, true] {
                    if page {
                        h.send(Msg::ShowDetail(index));
                    }
                    let screen = h.screen();
                    assert!(screen.contains("quvyta"), "{locale} at {width}x{height}:\n{screen}");
                    for forbidden in ['[', ']', '{', '}', '|', '▌', '⟦'] {
                        assert!(
                            !screen.contains(forbidden),
                            "`{forbidden}` in {locale} at {width}x{height}:\n{screen}"
                        );
                    }
                    h.send(Msg::Back);
                }
            }
        }
    }
}

/// With `QUVYTA_REVIEW=1`, writes every member in both languages, wide, narrow and short, to
/// `target/quvyta-review.html` in colour for a visual review; a narrow screen shows its list and
/// then every member's page. The notices after a member closes, for a broken settings file and
/// about PATH follow, then the Settings tab.
#[test]
fn visual_review() {
    if std::env::var_os("QUVYTA_REVIEW").is_none() {
        return;
    }
    let mut fragments = Vec::new();
    for (width, height) in [(100, 22), (48, 26), (72, 14)] {
        for locale in ["en", "tr"] {
            let (_root, mut h) = harness(width, height);
            h.set_locale(locale);
            if width < WIDE {
                fragments.push(h.html(&format!("list {locale} {width}x{height}")));
                println!("list {locale} {width}x{height}\n{}", h.screen());
            }
            for (index, member) in APPS.iter().enumerate() {
                h.send(if width < WIDE { Msg::ShowDetail(index) } else { Msg::Select(index) });
                fragments.push(h.html(&format!("{} {locale} {width}x{height}", member.key)));
                println!("{} {locale} {width}x{height}\n{}", member.key, h.screen());
            }
        }
    }
    for locale in ["en", "tr"] {
        let (_root, mut h) = harness(100, 22);
        h.set_locale(locale).set_handoff_outcome(HandoffOutcome::Finished { code: Some(1) });
        h.press("enter").advance(TOAST_IN);
        fragments.push(h.html(&format!("closed with a code {locale}")));
        println!("closed with a code {locale}\n{}", h.screen());
        let (_root, mut h) = harness_with_settings("after_close = \"exit\"\n");
        h.set_locale(locale).advance(TOAST_IN);
        fragments.push(h.html(&format!("broken launcher.conf {locale}")));
        println!("broken launcher.conf {locale}\n{}", h.screen());
    }
    fragments.extend(super::path_notice::tests::review());
    fragments.extend(super::wizard::tests::review());
    fragments.extend(super::settings::tests::review());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/quvyta-review.html");
    std::fs::write(path, qframe::runtime::html_page(&fragments)).expect("review page written");
}

#[test]
fn a_short_terminal_scrolls_to_the_focused_value() {
    let (_root, mut h) = harness(100, 12);
    h.send(Msg::Select(index("framework")));
    assert!(
        !h.screen().contains("https://github.com/quvyta/framework"),
        "the source starts below the fold:\n{}",
        h.screen()
    );
    for _ in 0..6 {
        h.press("tab");
        if h.is_focused("source") {
            break;
        }
    }
    assert!(h.is_focused("source"), "tab reaches the source value");
    assert!(h.screen().contains("https://github.com/quvyta/framework"), "and scrolls it into view:\n{}", h.screen());
}

#[test]
fn enter_opens_the_selected_member_by_its_full_path_in_the_home_folder() {
    let (root, mut h) = harness(100, 24);
    h.press("enter");
    let program = root.path().join("home/.cargo/bin/qcode");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, program.as_os_str());
    assert!(request.args.is_empty());
    assert_eq!(request.notice.as_deref(), Some("Opening qcode…"));
    let opening = h.app().opening(0).expect("qcode can be opened");
    assert_eq!(opening, Opening { program, dir: root.path().join("home") });
}

#[test]
fn the_open_button_opens_a_member_installed_elsewhere_from_where_it_was_found() {
    let (root, mut h) = harness(100, 24);
    h.press("down");
    assert!(h.screen().contains("Open"), "{}", h.screen());
    h.click_text("Open");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, root.path().join("bin/qfocus").as_os_str());
}

#[test]
fn quvyta_itself_and_missing_members_have_nothing_to_open() {
    let (_root, mut h) = harness(100, 24);
    h.send(Msg::Select(index("quvyta")));
    assert!(!h.screen().contains("Open") && !h.screen().contains("Install"), "{}", h.screen());
    assert!(!h.screen().contains("enter"), "the footer offers nothing for enter:\n{}", h.screen());
    h.press("enter").send(Msg::Open(index("quvyta")));
    assert!(h.handoffs().is_empty());
    h.send(Msg::Select(index("tools")));
    assert!(!h.screen().contains("Open"), "{}", h.screen());
    h.send(Msg::Open(index("tools")));
    assert!(h.handoffs().is_empty());
}

#[test]
fn a_member_that_fails_says_so_quietly_and_a_good_close_says_nothing() {
    for (outcome, toast) in [
        (HandoffOutcome::Finished { code: Some(1) }, Some("qcode closed with code 1")),
        (HandoffOutcome::Finished { code: None }, Some("qcode was stopped by a signal")),
        (HandoffOutcome::Failed("No such file or directory".to_owned()), Some("qcode could not be opened")),
        (HandoffOutcome::Finished { code: Some(0) }, None),
    ] {
        let (_root, mut h) = harness(100, 24);
        h.set_handoff_outcome(outcome.clone()).press("enter").advance(TOAST_IN);
        let screen = h.screen();
        match toast {
            Some(text) => assert!(screen.contains(text), "{outcome:?}:\n{screen}"),
            None => assert!(!screen.contains("qcode closed") && !screen.contains("could not"), "{screen}"),
        }
        if let HandoffOutcome::Failed(reason) = &outcome {
            assert!(screen.contains(reason.as_str()), "the reason is told:\n{screen}");
        }
        assert!(!h.quit_requested(), "quvyta comes back by default");
    }
}

#[test]
fn with_after_close_shell_quvyta_quits_when_the_member_closes() {
    let (_root, mut h) = harness_with_settings("after_close = \"shell\"\n");
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(0) }).press("enter");
    assert!(h.quit_requested());

    let (_root, mut h) = harness_with_settings("after_close = \"shell\"\n");
    h.set_handoff_outcome(HandoffOutcome::Failed("Permission denied".to_owned())).press("enter");
    assert!(!h.quit_requested(), "a member that never opened leaves quvyta to say why");

    let (_root, mut h) = harness_with_settings("after_close = \"return\"\n");
    h.press("enter");
    assert!(!h.quit_requested());
}

#[test]
fn a_broken_setting_falls_back_and_says_where() {
    let (_root, mut h) = harness_with_settings("after_close = \"exit\"\n");
    let screen = h.advance(TOAST_IN).screen();
    assert!(screen.contains("launcher.conf:1:"), "{screen}");
    h.press("enter");
    assert!(!h.quit_requested(), "the default, coming back, stands in");
}

#[test]
fn what_is_installed_is_read_again_after_a_member_closes_keeping_the_selection() {
    let (root, mut h) = harness(100, 24);
    h.send(Msg::Select(index("framework")));
    // While qframe was open, the user installed qtools with cargo.
    let list = format!("{LIST}quvyta-tools v0.1.2:\n    qtools\n");
    std::fs::write(root.path().join("bin/list.out"), list).expect("scenario");
    installed_command(&h.app().machine, "qtools");
    // One frame hands qframe the terminal and gets it back; the next hears cargo again.
    h.press("enter").render();
    let screen = h.screen();
    assert!(line_with(&screen, "qtools ").contains("0.1.2"), "{screen}");
    assert!(line_with(&screen, "qframe ").contains('▌'), "the selection stays:\n{screen}");
}

#[test]
fn on_a_narrow_screen_enter_shows_the_details_then_opens() {
    let (root, mut h) = harness(48, 20);
    h.press("enter");
    assert!(h.handoffs().is_empty(), "the first enter only shows the details");
    assert!(h.screen().contains("Open") && h.screen().contains("enter  open"), "{}", h.screen());
    h.press("enter");
    let [request] = h.handoffs() else { panic!("one handoff: {:?}", h.handoffs()) };
    assert_eq!(request.program, root.path().join("home/.cargo/bin/qcode").as_os_str());
}

/// The labels the details show for the member at `index`, in the harness' language: its title,
/// its badge, and the buttons its state puts on the screen. These come from the language files,
/// so they are the ones a narrow screen cuts.
fn labels_on_show(h: &Harness<Quvyta>, index: usize) -> Vec<String> {
    let say = |key: &str| h.env().i18n().translate(key, &[]);
    let member = &APPS[index];
    let app = h.app();
    let mut labels = vec![
        say(&format!("apps.{}.title", member.key)),
        say(match member.status {
            Status::Released => "status.released",
            Status::Beta => "status.beta",
            Status::Soon => "status.soon",
        }),
    ];
    if app.opening(index).is_some() {
        labels.push(say("detail.open"));
        if app.updatable(index) {
            labels.push(say("detail.update"));
        }
        if app.removable(index) {
            labels.push(say("detail.remove"));
        }
    } else if app.updatable(index) {
        labels.push(say("detail.update"));
    } else if app.installable(index) {
        labels.push(say("detail.install"));
    }
    labels
}

/// Whether `line` is the one a `CopyValue` of `member` draws: it begins with the head of the
/// value it carries, which the widget keeps when it shortens the middle.
fn carries_a_copyable(line: &str, member: &Member) -> bool {
    let mut values = member.library.into_iter().chain(std::iter::once(member.repository));
    values.any(|value| {
        let head: String = value.chars().take(8).collect();
        line.trim_start().starts_with(&head)
    })
}

/// Nothing the language files say may be cut, in any language, at any width the split or the
/// page is drawn at. `CopyValue` shortens its own middle by design and is left out of the sweep
/// by the value it carries; everything else stands whole or the layout is wrong.
#[test]
fn no_label_is_cut_on_a_narrow_screen_in_any_language() {
    for width in [40, 48, 56, 58, 60, 62, 70, 100] {
        for locale in LANGUAGES {
            let (_root, mut h) = harness(width, 44);
            h.set_locale(locale);
            for (index, member) in APPS.iter().enumerate() {
                for page in [false, true] {
                    h.send(Msg::Select(index));
                    if page {
                        h.send(Msg::ShowDetail(index));
                    }
                    let screen = h.screen();
                    let place = format!("`{}` in {locale} at {width} columns", member.key);
                    for line in screen.lines() {
                        assert!(
                            !line.contains('…') || carries_a_copyable(line, member),
                            "a line is cut short, {place}: `{}`\n{screen}",
                            line.trim_end()
                        );
                    }
                    if h.app().wide() || h.app().detail_page {
                        for label in labels_on_show(&h, index) {
                            assert!(screen.contains(&label), "`{label}` is not whole, {place}:\n{screen}");
                        }
                    }
                    h.send(Msg::Back);
                }
            }
        }
    }
}
