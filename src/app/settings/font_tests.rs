//! The Nerd Font row of the Settings tab: the offer on a machine that has none, the question
//! before installing it, the install itself and the row going away once the font is there.
//!
//! Every machine is a temporary root: the font folders the row reads and the folder the install
//! writes lie inside it, and the archive is a file in it, fetched with a `file://` address, so no
//! test looks at the owner's fonts, reaches the network or tells the system about a font.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Shell;
use std::time::Duration;

use qframe::icons::GlyphMode;
use qframe::icons::nerd_font::{self, Archive, Progress};
use qframe::runtime::Harness;

use super::super::tests::{LANGUAGES, TOAST_IN, env, machine};
use super::*;

/// The font folder of the machine in `root`, the one the row looks in.
fn fonts(root: &Path) -> PathBuf {
    root.join("fonts")
}

/// The folder the install writes, inside the machine's font folder.
fn target(root: &Path) -> PathBuf {
    fonts(root).join("QuvytaNerdFont")
}

/// A font file where the machine in `root` looks for one.
fn put_font(root: &Path) {
    fs::create_dir_all(fonts(root)).expect("font folder");
    fs::write(fonts(root).join(nerd_font::FONT_FILE), b"not a font").expect("font file");
}

/// An archive in `root` holding the font file at its top, as the release holds it, and its
/// SHA-256; `wrong` gives it a checksum it does not have.
fn archive(root: &Path, wrong: bool) -> Archive {
    let content = root.join("archive-content");
    fs::create_dir_all(&content).expect("archive content");
    fs::write(content.join(nerd_font::FONT_FILE), b"glyphs").expect("the font");
    let path = root.join("symbols.tar");
    let packed = Shell::new("tar")
        .arg("-cf")
        .arg(&path)
        .arg("-C")
        .arg(&content)
        .arg(nerd_font::FONT_FILE)
        .status()
        .expect("tar");
    assert!(packed.success(), "the archive is packed");
    let digest = Shell::new("sha256sum").arg(&path).output().expect("sha256sum");
    let digest = String::from_utf8_lossy(&digest.stdout).split_whitespace().next().expect("a digest").to_owned();
    let digest = if wrong { "0".repeat(64) } else { digest };
    Archive::new(format!("file://{}", path.display()), &digest)
}

/// The Settings tab of the machine in `root` at `width` by `height`, installing from `archive`.
fn font_tab(root: &Path, archive: Archive, width: u16, height: u16) -> Harness<Quvyta> {
    let mut app = Quvyta::new(machine(root));
    app.font.archive = archive;
    let mut h = Harness::with_env(app, env(), width, height);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    h.render().render();
    h.click_text("Settings");
    h
}

/// The tab of [`font_tab`] on a screen tall enough for the whole appearance section, with an
/// archive that installs.
fn tab(root: &Path) -> Harness<Quvyta> {
    font_tab(root, archive(root, false), 100, 44)
}

/// What a key says in the language on screen.
fn said(h: &Harness<Quvyta>, key: &str) -> String {
    h.env().i18n().translate(key, &[])
}

/// Where `text` starts on `screen`.
fn at(screen: &str, text: &str) -> usize {
    screen.find(text).unwrap_or_else(|| panic!("`{text}` is missing:\n{screen}"))
}

/// Clicks the dialog's action `label`, on the line of its Cancel, since the same word may also
/// stand in the title or on the page behind the dialog.
fn click_action(h: &mut Harness<Quvyta>, label: &str) {
    let cancel = said(h, "confirm.cancel");
    let (_, y) = h.find(&cancel).unwrap_or_else(|| panic!("no dialog:\n{}", h.screen()));
    let line = h.screen().lines().nth(usize::try_from(y).expect("a row")).expect("the line").to_owned();
    let start = at(&line, &cancel) + cancel.len();
    let x = start + at(&line[start..], label);
    let column = i32::from(qframe::text::width(&line[..x]));
    h.click(column, y);
}

/// Opens the question with the row's own button.
fn ask(h: &mut Harness<Quvyta>) {
    let install = said(h, "quvyta.setup.install");
    // The dialog pops in over a moment, and its buttons only stand once it has.
    h.click_text(&install).advance(TOAST_IN);
}

#[test]
fn a_machine_without_a_nerd_font_is_offered_one_in_the_appearance_section() {
    let root = tempfile::tempdir().expect("temp");
    let h = tab(root.path());
    let screen = h.screen();
    let missing = said(&h, "quvyta.nerd-font.missing");
    assert!(screen.contains(&said(&h, "quvyta.setup.install")), "the button:\n{screen}");
    // Among the shared rows, after the icon set it is for, and above quvyta's own settings.
    assert!(at(&screen, "Icons") < at(&screen, &missing), "{screen}");
    assert!(at(&screen, &missing) < at(&screen, "launcher.conf"), "{screen}");
}

#[test]
fn a_machine_with_a_nerd_font_is_told_nothing() {
    let root = tempfile::tempdir().expect("temp");
    put_font(root.path());
    let h = tab(root.path());
    let screen = h.screen();
    assert!(!screen.contains(&said(&h, "quvyta.nerd-font.missing")), "{screen}");
    assert!(!screen.contains(&said(&h, "quvyta.setup.install")), "no button either:\n{screen}");
    assert!(screen.contains("Icons"), "the rest of the section is there:\n{screen}");
}

#[test]
fn install_asks_first_and_says_what_will_run_and_where() {
    let root = crate::app::tests::temp_root();
    let archive = archive(root.path(), false);
    let url = archive.url().to_owned();
    let mut h = font_tab(root.path(), archive, 120, 44);
    ask(&mut h);
    let screen = h.screen();
    let title = format!("Install {}?", nerd_font::FAMILY);
    // The source is the archive this run would really fetch, and the target the folder it would
    // write, both inside this test's root.
    let folder = target(root.path()).display().to_string();
    for text in [title.as_str(), url.as_str(), nerd_font::RELEASE, folder.as_str(), "No sudo needed"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(screen.contains("To undo it, delete that folder"), "{screen}");
    assert!(!target(root.path()).exists(), "asking writes nothing");
}

#[test]
fn cancel_esc_and_the_close_mark_leave_the_machine_alone() {
    for close in ["Cancel", "esc", "×"] {
        let root = tempfile::tempdir().expect("temp");
        let mut h = tab(root.path());
        ask(&mut h);
        let title = format!("Install {}?", nerd_font::FAMILY);
        assert!(h.screen().contains(&title), "{close}: {}", h.screen());
        if close == "esc" {
            h.press("esc");
        } else {
            h.click_text(close);
        }
        h.advance(TOAST_IN);
        let screen = h.screen();
        assert!(!screen.contains(&title), "{close} closes the question:\n{screen}");
        assert!(screen.contains(&said(&h, "quvyta.nerd-font.missing")), "{close} leaves the offer:\n{screen}");
        assert!(!target(root.path()).exists(), "{close} installs nothing");
    }
}

#[test]
fn agreeing_installs_into_the_machine_s_font_folder_and_the_offer_goes() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = tab(root.path());
    ask(&mut h);
    click_action(&mut h, "Install");
    h.advance(Duration::from_millis(0));
    let screen = h.screen();
    assert!(target(root.path()).join(nerd_font::FONT_FILE).is_file(), "the font is in its folder:\n{screen}");
    let done = format!("{} is installed", nerd_font::FAMILY);
    assert!(screen.contains(&done), "what it did:\n{screen}");
    assert!(screen.contains("JetBrainsMono"), "and what to do if the samples are still boxes:\n{screen}");
    // The samples that note points at: the Unicode ones every terminal draws.
    assert!(screen.contains("■  ✓  ⌕  ▤"), "the samples to look at:\n{screen}");
    assert!(!screen.contains(&said(&h, "quvyta.nerd-font.missing")), "the offer went:\n{screen}");
    assert!(!screen.contains(&said(&h, "quvyta.setup.install")), "and its button:\n{screen}");
}

/// While the install runs, its step in the framework's words and a bar take the button's place, so
/// a second install cannot be started over the first. The steps come from the install's own
/// thread, which is what this test stands in for.
#[test]
fn a_running_install_shows_its_step_and_no_button() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = tab(root.path());
    h.send(Msg::Setting(SettingMsg::FontStep(Progress::Downloading { fraction: Some(0.4) })));
    let screen = h.screen();
    assert!(screen.contains("Downloading the font, 40%"), "{screen}");
    assert!(screen.contains("40%"), "the bar says how far too:\n{screen}");
    assert!(!screen.contains(&said(&h, "quvyta.setup.install")), "no second install while one runs:\n{screen}");
}

#[test]
fn the_keys_reach_the_offer_and_enter_asks() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = tab(root.path());
    for _ in 0..8 {
        if h.is_focused("appearance") {
            break;
        }
        h.press("tab");
    }
    assert!(h.is_focused("appearance"), "{}", h.screen());
    // Arrow down through the shared rows to the offer; each row's own control is left alone.
    let title = format!("Install {}?", nerd_font::FAMILY);
    for _ in 0..12 {
        if h.screen().contains(&title) {
            break;
        }
        h.press("down");
        if focused_line(&h).contains(&said(&h, "quvyta.nerd-font.missing")) {
            h.press("enter").advance(TOAST_IN);
        }
    }
    assert!(h.screen().contains(&title), "enter on the row asks:\n{}", h.screen());
}

/// The screen line the settings list marks as its current row: the one whose pillar is drawn.
fn focused_line(h: &Harness<Quvyta>) -> String {
    h.screen().lines().skip(1).find(|line| line.starts_with('▌')).unwrap_or_default().to_owned()
}

#[test]
fn a_failed_install_says_why_and_the_offer_stays() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = font_tab(root.path(), archive(root.path(), true), 100, 44);
    ask(&mut h);
    click_action(&mut h, "Install");
    h.advance(Duration::from_millis(0));
    let screen = h.screen();
    let mark = h.env().icons().glyph("warning").into_owned();
    assert!(screen.contains(&format!("{mark} The download does not match its checksum")), "{screen}");
    assert!(screen.contains(&said(&h, "quvyta.setup.install")), "it can be tried again:\n{screen}");
    assert!(!target(root.path()).join(nerd_font::FONT_FILE).exists(), "nothing was installed");
}

#[test]
fn no_real_font_folder_is_looked_at_or_written() {
    let root = tempfile::tempdir().expect("temp");
    let h = tab(root.path());
    assert_eq!(h.app().font.install.target_dir(), Some(target(root.path()).as_path()), "the install stays in the root");
    assert!(!h.app().font.installed, "an empty root has no font, whatever the machine running the test has");
}

#[test]
fn the_row_and_its_question_cut_nothing_in_any_language() {
    for width in [40, 48, 60, 100] {
        for locale in LANGUAGES {
            for state in ["offer", "asking", "failed", "done"] {
                let root = tempfile::tempdir().expect("temp");
                let mut h = font_tab(root.path(), archive(root.path(), state == "failed"), width, 60);
                h.set_locale(locale).set_glyph_mode(GlyphMode::Ascii);
                if state != "offer" {
                    ask(&mut h);
                }
                if matches!(state, "failed" | "done") {
                    let install = said(&h, "confirm.install");
                    click_action(&mut h, &install);
                }
                h.advance(Duration::from_millis(0));
                let screen = h.screen();
                let context = format!("{state} in {locale} at {width}");
                // Each state is on screen, so the sweep measures what it names.
                let shown = match state {
                    "asking" => said(&h, "confirm.cancel"),
                    "done" => nerd_font::FAMILY.to_owned(),
                    _ => said(&h, "quvyta.setup.install"),
                };
                assert!(screen.contains(&shown), "`{shown}` {context}:\n{screen}");
                for forbidden in ['[', ']', '{', '}', '|', '▌', '⟦'] {
                    assert!(!screen.contains(forbidden), "`{forbidden}` {context}:\n{screen}");
                }
                for line in screen.lines() {
                    assert!(qframe::text::width(line) <= width, "`{line}` is too wide {context}");
                    if line.contains('\u{2026}') {
                        assert!(line.trim_end().ends_with("launcher.conf"), "`{}` is cut {context}", line.trim());
                    }
                }
            }
        }
    }
}

/// The row and its question in both languages, wide and narrow, for the visual review: the offer,
/// the question and the finished install.
pub(in crate::app) fn review() -> Vec<String> {
    let mut fragments = Vec::new();
    for (width, height) in [(100, 44), (48, 60)] {
        for locale in ["en", "tr"] {
            for state in ["offer", "asking", "done"] {
                let root = tempfile::tempdir().expect("temp");
                let mut h = font_tab(root.path(), archive(root.path(), false), width, height);
                h.set_locale(locale);
                if state != "offer" {
                    ask(&mut h);
                }
                if state == "done" {
                    let install = said(&h, "confirm.install");
                    click_action(&mut h, &install);
                    h.advance(Duration::from_millis(0));
                }
                let title = format!("settings nerd font {state} {locale} {width}x{height}");
                fragments.push(h.html(&title));
                println!("{title}\n{}", h.screen());
            }
        }
    }
    fragments
}
