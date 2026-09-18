use qframe::env::{AssetDirs, Env};
use qframe::icons::GlyphMode;

use super::*;

/// Built-in files plus the compiled-in locales, as the runtime loads them.
fn env() -> Env {
    let dirs = AssetDirs {
        locale_sources: crate::LOCALES.iter().map(|(file, text)| ((*file).to_owned(), (*text).to_owned())).collect(),
        ..AssetDirs::default()
    };
    Env::load(&dirs).expect("the locales are readable")
}

fn harness(width: u16, height: u16) -> Harness<Quvyta> {
    let mut h = Harness::with_env(Quvyta::default(), env(), width, height);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    h
}

#[test]
fn the_language_files_load_without_problems_and_turkish_is_complete() {
    let env = env();
    assert_eq!(env.diagnostics(), &[]);
    assert_eq!(env.i18n().missing_keys("tr", "en"), Vec::<String>::new());
}

#[test]
fn starts_on_the_framework_with_how_to_add_it() {
    let h = harness(100, 24);
    let screen = h.screen();
    for text in [
        "quvyta",
        "Quvyta Framework",
        "Released",
        "quvyta-framework",
        "Add to a project",
        "cargo add quvyta-framework",
        "quit",
    ] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("Command"), "a library has no command:\n{screen}");
    assert!(!screen.contains("beta"), "the library is not marked as a beta:\n{screen}");
}

#[test]
fn a_click_on_a_tab_shows_that_application() {
    let mut h = harness(100, 24);
    h.click_text("qcode");
    let screen = h.screen();
    for text in [
        "Quvyta Code",
        "Beta",
        "Command",
        "qcode",
        "https://github.com/quvyta/code",
        "Install",
        "cargo install quvyta-code",
        "A beta",
    ] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("cargo add"), "an application is installed, not added:\n{screen}");
}

#[test]
fn every_application_installs_under_its_package_name() {
    for member in FAMILY.iter().filter(|member| member.command.is_some()) {
        assert_eq!(member.install, format!("cargo install {}", member.package));
        assert_eq!(member.status, Status::Beta, "{}", member.package);
        assert!(member.repository.starts_with("https://github.com/quvyta/"), "{}", member.repository);
    }
}

#[test]
fn the_keyboard_moves_between_tabs() {
    let mut h = harness(100, 24);
    h.press("tab");
    assert!(h.is_focused("family"), "the tabs take focus first");
    h.press("right").press("right");
    assert!(h.screen().contains("Quvyta Focus"), "{}", h.screen());
}

#[test]
fn copying_the_install_command_puts_it_on_the_clipboard() {
    let mut h = harness(100, 24);
    h.click_text("cargo add quvyta-framework");
    assert_eq!(h.copied(), ["cargo add quvyta-framework"]);
}

#[test]
fn an_unknown_index_changes_nothing() {
    let mut h = harness(100, 24);
    h.send(Msg::Select(FAMILY.len()));
    assert!(h.screen().contains("Quvyta Framework"));
}

#[test]
fn every_member_reads_fully_in_every_language() {
    for locale in ["en", "tr"] {
        let mut h = harness(100, 24);
        h.set_locale(locale);
        for index in 0..FAMILY.len() {
            h.send(Msg::Select(index));
            let screen = h.screen();
            assert!(!screen.contains('⟦'), "a key is missing in `{locale}`:\n{screen}");
        }
    }
}

#[test]
fn turkish_uses_its_own_words() {
    let mut h = harness(100, 24);
    h.set_locale("tr");
    let screen = h.screen();
    for text in ["Özenle yapılmış terminal uygulamaları", "Yayında", "Projeye ekle", "Kaynak"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
}

#[test]
fn narrow_ascii_screens_keep_the_rules() {
    for (width, height) in [(40, 16), (60, 20), (100, 24)] {
        let mut h = harness(width, height);
        h.set_glyph_mode(GlyphMode::Ascii);
        for index in 0..FAMILY.len() {
            h.send(Msg::Select(index));
            let screen = h.screen();
            assert!(screen.contains("quvyta"), "{width}x{height}:\n{screen}");
            for forbidden in ['[', ']', '{', '}', '|'] {
                assert!(!screen.contains(forbidden), "`{forbidden}` at {width}x{height}:\n{screen}");
            }
        }
    }
}

/// With `QUVYTA_REVIEW=1`, writes every member in both languages, wide and narrow, to
/// `target/quvyta-review.html` in colour for a visual review.
#[test]
fn visual_review() {
    if std::env::var_os("QUVYTA_REVIEW").is_none() {
        return;
    }
    let mut fragments = Vec::new();
    for (width, height) in [(100, 22), (48, 26), (72, 14)] {
        for locale in ["en", "tr"] {
            let mut h = harness(width, height);
            h.set_locale(locale);
            for (index, member) in FAMILY.iter().enumerate() {
                h.send(Msg::Select(index));
                fragments.push(h.html(&format!("{} {locale} {width}x{height}", member.key)));
                println!("{} {locale} {width}x{height}\n{}", member.key, h.screen());
            }
        }
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/quvyta-review.html");
    std::fs::write(path, qframe::runtime::html_page(&fragments)).expect("review page written");
}

#[test]
fn a_short_terminal_scrolls_to_the_focused_value() {
    let mut h = harness(72, 14);
    assert!(!h.screen().contains("cargo add"), "the install value starts below the fold:\n{}", h.screen());
    for _ in 0..4 {
        h.press("tab");
        if h.is_focused("install") {
            break;
        }
    }
    assert!(h.is_focused("install"), "tab reaches the install value");
    assert!(h.screen().contains("cargo add quvyta-framework"), "and scrolls it into view:\n{}", h.screen());
}
