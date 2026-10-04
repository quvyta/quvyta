//! What changed in the version you now run: the page on the first start after an update, what
//! it says, the three ways out of it, and the reader that takes one version's section out of the
//! changelog as it ships.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use qframe::event::MouseKind;
use qframe::i18n::Arg;
use qframe::icons::GlyphMode;
use tempfile::TempDir;

use super::install_tests::{has, run, settle};
use super::tests::{LANGUAGES, LONGEST_LANGUAGES, TOAST_IN, line_with, machine};
use super::whats_new::{CHANGELOG, section};
use super::*;
use crate::install::tests::packages;
use crate::inventory::tests::machine_with_cargo;
use crate::updates::tests::search_scenario;

/// A version nobody has run, so the running one is a version they have not seen.
const OLDER: &str = "0.0.1";

/// crates.io answering that qcode, qtools and the showcase have newer versions, so the line under
/// the list says something the list has to come back to.
fn search() -> String {
    format!(
        "\
quvyta-code = \"0.1.2\"                  # Runs containers
quvyta-tools = \"0.1.2\"                 # Applies the recommended Arch Linux settings
quvyta-framework-showcase = \"0.1.5\"    # Every component, live
quvyta = \"{version}\"                   # This application
",
        version = env!("CARGO_PKG_VERSION")
    )
}

/// Writes the file in the data folder, making the folder first as quvyta does.
fn remember(root: &Path, version: &str) {
    std::fs::create_dir_all(root.join("data")).expect("folder");
    std::fs::write(root.join("data/seen.toml"), format!("version = \"{version}\"\n")).expect("seen");
}

/// The version in the file in the data folder, `None` when there is no file or no version in it.
fn remembered(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("data/seen.toml")).ok()?;
    qframe::storage::Settings::parse_str("seen.toml", &text).get("version")
}

/// quvyta on a machine in `root` that has seen `version` as its last version, crates.io
/// answering, at `width` by `height`. `None` for a machine that has seen nothing at all.
fn start(root: &Path, version: Option<&str>, width: u16, height: u16) -> Harness<Quvyta> {
    if let Some(version) = version {
        remember(root, version);
    }
    let app = Quvyta::new(machine(root));
    search_scenario(root, &search(), 0);
    packages(root);
    let mut h = run(app, width, height);
    settle(&mut h);
    h
}

/// A machine in a folder of its own that has seen `version`, and the page open on it. The folder
/// lives as long as the returned guard, which is what the data folder lives in.
fn page(version: &str, width: u16, height: u16) -> (TempDir, Harness<Quvyta>) {
    let root = tempfile::tempdir().expect("temp");
    let h = start(root.path(), Some(version), width, height);
    (root, h)
}

/// The title the page shows, in the language it is showing.
fn title(h: &Harness<Quvyta>) -> String {
    h.env().i18n().translate("whats-new.title", &[("version", Arg::from(env!("CARGO_PKG_VERSION")))])
}

/// The rows the page takes: under the header, above the hint bar.
fn rows(h: &Harness<Quvyta>) -> Vec<String> {
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    lines[1..lines.len().saturating_sub(1)].iter().map(|line| (*line).to_owned()).collect()
}

/// The marks the page draws around the entry: the pillar down its left edge and the bullet in
/// front of a line of it, both the framework's own drawing.
fn marks(h: &Harness<Quvyta>) -> Vec<char> {
    std::iter::once('\u{258c}').chain(h.env().icons().glyph("bullet").chars()).collect()
}

/// The words the entry itself is shown in, in the order the page shows them: the rows under the
/// title, which every language names with the version, without the page's own marks. Once the
/// title has scrolled above the fold there is nothing of the page's own left on screen.
fn shown(h: &Harness<Quvyta>) -> Vec<String> {
    let marks = marks(h);
    let rows = rows(h);
    let under = rows.iter().position(|row| row.contains(env!("CARGO_PKG_VERSION"))).map_or(0, |at| at + 1);
    rows[under..]
        .iter()
        .flat_map(|row| row.split_whitespace())
        .filter(|word| !word.chars().next().is_some_and(|first| marks.contains(&first)))
        .map(ToOwned::to_owned)
        .collect()
}

/// The running version's entry as the changelog that ships writes it: the file's own words, with
/// the marks a document draws them with taken out.
fn entry() -> Vec<String> {
    let found = section(CHANGELOG, env!("CARGO_PKG_VERSION")).expect("the running version has an entry");
    found
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim_start().trim_start_matches("- ").replace(['*', '`'], ""))
        .flat_map(|line| line.split_whitespace().map(ToOwned::to_owned).collect::<Vec<_>>())
        .collect()
}

/// The entry's first three words, which open its first line at any width the page is tested at.
fn opening() -> String {
    entry()[..3].join(" ")
}

/// Where in the entry the words on screen begin: the one place where they follow each other word
/// for word, since the top of the screen can cut a line of the entry in two.
fn at_in(entry: &[String], shown: &[String]) -> Option<usize> {
    if shown.is_empty() || shown.len() > entry.len() {
        return None;
    }
    (0..=entry.len() - shown.len()).find(|at| entry[*at..*at + shown.len()] == *shown)
}

#[test]
fn the_first_start_after_an_update_opens_what_changed_in_that_version() {
    // A screen tall enough for the whole entry, so every line of it can be compared with the file.
    let (root, mut h) = page(OLDER, 120, 200);
    let version = env!("CARGO_PKG_VERSION");
    let screen = h.screen();
    assert_eq!(title(&h), format!("What changed in {version}"), "the title names the version that runs");
    assert!(!screen.contains(OLDER), "not the version that was there before:\n{screen}");

    let words = entry();
    assert_eq!(shown(&h), words, "the entry is the file's own words, every one of them, in its order");
    assert_eq!(remembered(root.path()), Some(version.to_owned()), "the version is kept");

    h.press("esc");
    let list = h.screen();
    assert!(list.contains("qcode") && !list.contains("What changed in"), "{list}");
}

#[test]
fn esc_and_the_back_button_bring_the_list_back_as_it_was() {
    let root = tempfile::tempdir().expect("temp");

    let mut with_esc = start(root.path(), Some(OLDER), 100, 30);
    assert!(has(&with_esc, "What changed in"), "{}", with_esc.screen());
    with_esc.press("esc");
    let list = with_esc.screen();
    assert!(line_with(&list, "qcode").contains('▌'), "the app that was selected is the one again:\n{list}");
    assert!(list.contains("2 updates") && list.contains("Install updates"), "the line under it is there:\n{list}");

    // The same machine again, at the older version once more, left with the button this time. The
    // keys reach it and `enter` presses it, which the page's own surface would not answer: a
    // click anywhere closes the page, so only the keyboard tells the button from the page.
    let mut with_button = start(root.path(), Some(OLDER), 100, 30);
    assert!(has(&with_button, "What changed in"), "{}", with_button.screen());
    for _ in 0..4 {
        with_button.press("tab");
        if with_button.is_focused("back") {
            break;
        }
    }
    assert!(with_button.is_focused("back"), "tab reaches the Back button:\n{}", with_button.screen());
    with_button.press("enter");
    assert_eq!(with_button.screen(), list, "the button and the key leave the same list behind");
}

#[test]
fn a_click_closes_it_too() {
    let (_root, mut h) = page(OLDER, 100, 30);
    assert!(has(&h, "What changed in"), "{}", h.screen());
    // On the page itself, not on the button: the surface answers the pointer like the key does.
    let (x, y) = h.find(&opening()).expect("the first line of the entry");
    h.click(x + 1, y);
    let screen = h.screen();
    assert!(!screen.contains("What changed in") && screen.contains("qcode"), "{screen}");
}

#[test]
fn the_second_start_says_nothing() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = start(root.path(), Some(OLDER), 100, 30);
    h.press("esc");
    drop(h);

    let h = start(root.path(), None, 100, 30);
    let screen = h.screen();
    assert!(!screen.contains("What changed in"), "the entry of this version was seen already:\n{screen}");
    assert!(screen.contains("qcode") && line_with(&screen, "qcode").contains('▌'), "{screen}");
    assert_eq!(remembered(root.path()), Some(env!("CARGO_PKG_VERSION").to_owned()));
}

#[test]
fn someone_who_used_quvyta_before_it_kept_the_version_sees_the_page_once() {
    // quvyta set up here long ago, before any version was kept: `launcher.conf` is there and
    // `seen.toml` is not. That is everyone who updates from a version without this page.
    let root = tempfile::tempdir().expect("temp");
    let app = Quvyta::new(machine(root.path()));
    std::fs::remove_file(root.path().join("data/seen.toml")).expect("no version kept yet");
    search_scenario(root.path(), &search(), 0);
    packages(root.path());
    let mut h = run(app, 100, 30);
    settle(&mut h);
    assert!(has(&h, "What changed in"), "they updated, so they are told what changed:\n{}", h.screen());
    assert_eq!(remembered(root.path()), Some(env!("CARGO_PKG_VERSION").to_owned()), "and the version is kept");
    h.press("esc");
    drop(h);

    let h = start(root.path(), None, 100, 30);
    assert!(!has(&h, "What changed in"), "once is enough:\n{}", h.screen());
}

#[test]
fn someone_new_meets_the_wizard_and_never_the_changelog() {
    let root = tempfile::tempdir().expect("temp");
    // A first start: no `launcher.conf`, so the wizard has the screen.
    let app = Quvyta::new(machine_with_cargo(root.path(), "", 0));
    search_scenario(root.path(), &search(), 0);
    packages(root.path());
    let mut h = run(app, 100, 30);
    settle(&mut h);
    assert!(h.app().setting_up(), "{}", h.screen());
    assert!(!has(&h, "What changed in"), "{}", h.screen());
    h.click_text("Start with the defaults");
    settle(&mut h);
    let screen = h.screen();
    assert!(!h.app().setting_up() && screen.contains("qcode"), "the list follows the wizard:\n{screen}");
    assert!(!screen.contains("What changed in"), "and no changelog after it:\n{screen}");
    assert_eq!(remembered(root.path()), Some(env!("CARGO_PKG_VERSION").to_owned()), "the running version is kept");
    drop(h);

    // Their second start is not an update either.
    let h = start(root.path(), None, 100, 30);
    assert!(!has(&h, "What changed in"), "{}", h.screen());
}

#[test]
fn a_broken_file_is_a_cache_gone_missing_and_the_page_opens_its_once() {
    let root = tempfile::tempdir().expect("temp");
    std::fs::create_dir_all(root.path().join("data")).expect("folder");
    // Not a file quvyta wrote whole: half a write from somewhere else, or a hand edit.
    std::fs::write(root.path().join("data/seen.toml"), "version = [").expect("broken");

    let mut h = start(root.path(), None, 100, 30);
    assert!(has(&h, "What changed in"), "a file that is not ours is only a cache gone missing:\n{}", h.screen());
    h.press("esc");
    assert_eq!(remembered(root.path()), Some(env!("CARGO_PKG_VERSION").to_owned()), "and it is written whole again");

    let h = start(root.path(), None, 100, 30);
    assert!(!has(&h, "What changed in"), "so the page has opened its once:\n{}", h.screen());
}

#[test]
fn the_reader_takes_the_section_of_the_version_and_nothing_else() {
    let text = "\
# Changelog

## Unreleased

- Nothing yet.

## 0.3.1 (2026-09-25)

- The first thing.
- The second thing.

---

- A third thing, after a rule.

### 0.3.0

Not a section of its own.

## 0.3.10 (2026-10-01)

- A later version.
";
    assert_eq!(
        section(text, "0.3.1").expect("0.3.1 has a section"),
        "- The first thing.\n- The second thing.\n\n\n- A third thing, after a rule.\n\n### 0.3.0\n\nNot a section of its own.\n",
        "the heading is left out, the rule between the entries is not shown, and the next heading ends the section"
    );
    assert!(section(text, "0.3.0").is_none(), "a `###` heading is not a section of its own");
    assert!(
        section("## 0.3.10 (2026-10-01)\n\n- Only the later one.\n", "0.3.1").is_none(),
        "0.3.10 is not the section of 0.3.1"
    );
    assert!(section(text, "9.9.9").is_none(), "a version that is not in the file has nothing to show");
    assert!(section("## 0.3.1\n\n## 0.3.2\n", "0.3.1").is_none(), "a section with nothing in it shows nothing");
}

#[test]
fn the_running_version_has_its_own_entry_or_the_build_is_between_releases() {
    let version = env!("CARGO_PKG_VERSION");
    let newest = CHANGELOG.lines().find_map(|line| line.strip_prefix("## ")).expect("a heading in the file");
    let found = section(CHANGELOG, version);
    if found.is_none() {
        assert!(
            newest.starts_with("Unreleased"),
            "the release of {version} has no entry of its own and the newest heading is `{newest}`: \
             a release writes the entry of its version under a heading of its own, before it bumps the version"
        );
    }
    assert!(
        !found.unwrap_or_default().trim().is_empty(),
        "`## {version}` has nothing under it: an entry with no sentences is no entry"
    );
}

#[test]
fn a_file_that_cannot_be_written_says_so_quietly() {
    let root = tempfile::tempdir().expect("temp");
    remember(root.path(), OLDER);
    let data = root.path().join("data");
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o555)).expect("read-only");

    let app = Quvyta::new(machine(root.path()));
    search_scenario(root.path(), &search(), 0);
    packages(root.path());
    let mut h = run(app, 100, 30);
    settle(&mut h);
    let screen = h.advance(TOAST_IN).screen();
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o755)).expect("writable again");

    assert!(screen.contains("What changed in"), "the entry is read whether it can be kept or not:\n{screen}");
    assert!(screen.contains("cannot remember"), "and the page says it cannot remember it:\n{screen}");
    // Nothing crashes, and the page is a page: it is left the way every page is.
    h.press("esc");
    assert!(h.screen().contains("qcode"), "{}", h.screen());
    assert_eq!(remembered(root.path()), Some(OLDER.to_owned()), "so the next start opens it again");
}

#[test]
fn a_long_entry_scrolls_on_a_short_terminal() {
    // The entry is the running version's, whatever its length, so the terminals are narrow and
    // short: an entry of a few lines still runs past them.
    for (width, height) in [(50, 8), (40, 8)] {
        let (_root, mut h) = page(OLDER, width, height);
        let entry = entry();
        let start = shown(&h);
        assert!(!start.is_empty(), "the entry is on screen at {width}x{height}");
        let first = at_in(&entry, &start);
        assert_eq!(
            first,
            Some(0),
            "the page opens at the first line of the entry, whole words in the file's order, at {width}x{height}:\n{}",
            h.screen()
        );

        // The scrollbar is the framework's: the last column, a thumb on a track, in the colours
        // the theme gives them, and the entry never runs under it.
        let surface = h.env().theme().color("active");
        let track = h.env().theme().color("raised");
        let last = width - 1;
        let bar = |h: &Harness<Quvyta>| (1..height - 1).map(|row| h.bg(last, row)).collect::<Vec<_>>();
        // The thumb is whatever the theme paints on the track, brightened while the bar is under
        // the pointer, so it is read as the one cell of the column that is not the track.
        let thumb = |column: &[Option<_>]| column.iter().position(|cell| *cell != track).expect("a thumb on the bar");
        let column = bar(&h);
        assert!(
            column.iter().all(|cell| *cell != surface),
            "the bar is the last column and the page's surface stops before it at {width}x{height}: {column:?}"
        );
        assert!(column.contains(&track), "painted in the theme's own track colour");
        assert_eq!(thumb(&column), 0, "and its thumb is at the top, as the entry starts:\n{}", h.screen());
        let canvas = h.env().theme().color("canvas");
        for row in 1..height - 1 {
            assert_eq!(
                h.bg(last - 1, row),
                canvas,
                "the framework leaves the bar a column of its own at {width}x{height}"
            );
            assert_eq!(h.bg(last - 2, row), surface, "and the entry is on the page's own surface, short of it");
        }

        // The wheel a person scrolls with takes the entry down, past the title and into the words.
        for _ in 0..3 {
            h.mouse(MouseKind::ScrollDown, 10, 5);
        }
        let scrolled = shown(&h);
        let after = at_in(&entry, &scrolled);
        assert!(
            after.is_some_and(|after| after > first.unwrap_or(0)),
            "the page scrolls down the entry, still whole words of the file in its order, at {width}x{height}:\n{}",
            h.screen()
        );
        assert!(
            !rows(&h).iter().any(|row| row.contains(env!("CARGO_PKG_VERSION"))),
            "and the title is above the fold:\n{}",
            h.screen()
        );

        // A click at the foot of the bar takes it to the end of the entry, and the thumb down with it.
        h.click(i32::from(width) - 1, i32::from(height) - 2);
        let end = shown(&h);
        assert_eq!(
            end.last(),
            entry.last(),
            "the last line of the entry is in view at {width}x{height}:\n{}",
            h.screen()
        );
        let column = bar(&h);
        assert!(thumb(&column) > column.len() / 2, "and the thumb is at the foot of the bar:\n{}", h.screen());
        h.press("esc");
        assert!(h.screen().contains("qcode"), "the list is whole again at {width}x{height}");
    }
}

#[test]
fn the_page_reads_in_every_language_around_its_own_words() {
    let version = env!("CARGO_PKG_VERSION");
    for locale in LANGUAGES {
        let (_root, mut h) = page(OLDER, 40, 24);
        h.set_locale(locale);
        let i18n = h.env().i18n();
        // The words around the entry are the language's own, the ones its file already holds.
        let expected = i18n.translate("whats-new.title", &[("version", Arg::from(version))]);
        let back = i18n.translate("nav.back", &[]);
        let screen = h.screen();
        assert!(screen.contains(&expected), "`{expected}` in {locale}:\n{screen}");
        assert!(screen.contains(&back), "`{back}` in {locale}:\n{screen}");
        // Each language's own words, written out: a language that fell back to English, or lost
        // the version on the way, is caught here.
        let own = match locale {
            "tr" => format!("{version} sürümünde ne değişti"),
            "de" => format!("Was sich in {version} geändert hat"),
            "es" => format!("Qué cambió en {version}"),
            "fr" => format!("Ce qui a changé dans {version}"),
            "ja" => format!("{version} の変更点"),
            "pt-BR" => format!("O que mudou em {version}"),
            "ru" => format!("Что изменилось в {version}"),
            "zh-Hans" => format!("{version} 版本的变化"),
            _ => format!("What changed in {version}"),
        };
        assert_eq!(expected, own, "the title is written in {locale}");
        assert!(!screen.contains('\u{2026}'), "nothing of the page is cut in {locale}:\n{screen}");
        // The entry itself stays as the file writes it, in every language: it is the file's words.
        assert!(screen.contains(&opening()), "the entry is the file's own words in {locale}:\n{screen}");
        h.press("esc");
        assert!(h.screen().contains("qcode"), "{locale}: {}", h.screen());
    }

    // The page is a surface of its own, so it is drawn in the plain glyph mode too: the pillar and
    // the bullet are whatever that mode has, and nothing the rules forbid comes in with them.
    for locale in LONGEST_LANGUAGES {
        let (_root, mut h) = page(OLDER, 40, 24);
        h.set_glyph_mode(GlyphMode::Ascii).set_locale(locale);
        let screen = h.screen();
        for forbidden in ['[', ']', '{', '}', '|', '\u{27e6}'] {
            assert!(!screen.contains(forbidden), "`{forbidden}` on the page in {locale}:\n{screen}");
        }
        let title = h.env().i18n().translate("whats-new.title", &[("version", Arg::from(env!("CARGO_PKG_VERSION")))]);
        assert!(screen.contains(&title), "and the page is still there in {locale}:\n{screen}");
    }
}
