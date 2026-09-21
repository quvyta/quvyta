//! The first start: the wizard opens while quvyta has no `launcher.conf`, writes nothing until it
//! finishes, and hands what was checked to the install dialog.
//!
//! Every test lives in a temporary root: the settings folder, the fonts the appearance step looks
//! at and cargo are all inside it, so neither the user's settings nor a real font is ever touched.

use std::fs;
use std::path::Path;

use qframe::icons::GlyphMode;
use qframe::prelude::*;
use qframe::widgets::{AppearanceChange, SetupMsg};

use super::WizardMsg;
use crate::app::install_tests::{DIALOG_IN, calls, click_beside, has, settle};
use crate::app::tests::{LANGUAGES, env, index};
use crate::app::{Msg, Quvyta, Tab};
use crate::install::tests::{packages, scenario};
use crate::inventory::tests::{machine_with_cargo, write_program};
use crate::machine::Machine;

/// A machine in `root` with cargo and a C linker, where nothing of the family is installed and
/// quvyta has no settings file: the machine of a first start. `distro` is what
/// `/etc/os-release` says, which decides whether the members that run on Arch Linux only can be
/// chosen.
fn machine(root: &Path, distro: &str) -> Machine {
    let mut machine = machine_with_cargo(root, "", 0);
    write_program(&root.join("bin/cc"), "#!/bin/sh\nexit 0\n");
    machine.path.push(machine.cargo_bin());
    fs::create_dir_all(root.join("etc")).expect("folder");
    fs::write(root.join("etc/os-release"), distro).expect("os-release");
    machine
}

/// quvyta on the machine in `root`, on a screen of `width` by `height`, with what is installed
/// already read.
fn start(root: &Path, distro: &str, width: u16, height: u16) -> Harness<Quvyta> {
    let mut h = Harness::with_env(Quvyta::new(machine(root, distro)), env(), width, height);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    // The first frame asked cargo what is installed; this one hears the answer.
    h.render();
    h
}

/// The wizard on a wide screen of a machine that is not Arch Linux.
fn wizard(root: &Path) -> Harness<Quvyta> {
    start(root, "ID=debian\n", 80, 30)
}

/// What the settings folder holds, by name, in order; empty when there is no folder at all.
fn folder(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root.join("config")) else { return Vec::new() };
    let mut names: Vec<String> =
        entries.map(|entry| entry.expect("entry").file_name().to_string_lossy().into()).collect();
    names.sort();
    names
}

/// Goes to quvyta's own step of the wizard.
fn to_family_step(h: &mut Harness<Quvyta>) {
    h.send(Msg::Setup(SetupMsg::Next));
}

/// Finishes the wizard and lets the writing and the dialogs that follow happen.
fn finish(h: &mut Harness<Quvyta>) {
    h.send(Msg::Setup(SetupMsg::Finish));
    h.render();
    h.advance(DIALOG_IN);
}

#[test]
fn it_opens_while_quvyta_has_no_settings_file_of_its_own() {
    let root = tempfile::tempdir().expect("temp");
    let h = wizard(root.path());
    assert!(h.app().setting_up(), "the first start asks:\n{}", h.screen());
    let screen = h.screen();
    for text in ["quvyta", "Appearance", "Applications", "In every Quvyta application", "Start with the defaults"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("not installed"), "the family list waits its turn:\n{screen}");
}

#[test]
fn someone_who_has_used_quvyta_before_is_not_asked() {
    let root = tempfile::tempdir().expect("temp");
    fs::create_dir_all(root.path().join("config")).expect("folder");
    fs::write(root.path().join("config/launcher.conf"), "after_close = \"shell\"\n").expect("settings");
    let h = start(root.path(), "ID=debian\n", 80, 30);
    assert!(!h.app().setting_up(), "{}", h.screen());
    assert!(h.screen().contains("qcode"), "the family is there at once:\n{}", h.screen());
}

#[test]
fn closing_it_half_way_writes_nothing_at_all() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = wizard(root.path());
    assert_eq!(folder(root.path()), Vec::<String>::new(), "nothing is written before it is asked");
    h.send(Msg::Setup(SetupMsg::Appearance(AppearanceChange::Theme("nordic".to_owned()))));
    to_family_step(&mut h);
    h.send(Msg::Wizard(WizardMsg::Pick(index("code"), true)));
    assert_eq!(folder(root.path()), Vec::<String>::new(), "half-way through, the folder is as it was");
    // Next start: the wizard is there again, with nothing remembered.
    let again = start(root.path(), "ID=debian\n", 80, 30);
    assert!(again.app().setting_up(), "{}", again.screen());
    assert!(!again.app().picked.iter().any(|picked| *picked), "nothing was remembered");
}

#[test]
fn starting_with_the_defaults_writes_both_files_and_never_asks_again() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = wizard(root.path());
    h.click_text("Start with the defaults");
    h.render();
    h.advance(DIALOG_IN);
    assert!(!h.app().setting_up(), "{}", h.screen());
    assert_eq!(folder(root.path()), ["launcher.conf", "quvyta.conf"]);
    let text = fs::read_to_string(root.path().join("config/launcher.conf")).expect("written");
    for line in ["after_close = \"return\"", "check_updates = true"] {
        assert!(text.contains(line), "`{line}` is missing:\n{text}");
    }
    assert!(h.screen().contains("qcode"), "the family has the screen:\n{}", h.screen());
    assert!(h.app().installs.dialog.is_none(), "nothing was asked to be installed");
    let again = start(root.path(), "ID=debian\n", 80, 30);
    assert!(!again.app().setting_up(), "{}", again.screen());
}

#[test]
fn the_family_step_says_what_each_member_is_and_checks_nothing_by_itself() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = wizard(root.path());
    to_family_step(&mut h);
    let screen = h.screen();
    assert!(screen.contains("Which of them would you like?"), "{screen}");
    for (command, line) in [
        ("qcode", "Coding agents inside Podman"),
        ("qfocus", "Tracks what you focus on"),
        ("qframe", "showcase of the framework"),
        ("qdesk", "A desktop inside the terminal"),
    ] {
        assert!(screen.contains(command) && screen.contains(line), "`{command}` is missing:\n{screen}");
    }
    assert!(!screen.lines().any(|line| line.trim() == "quvyta"), "quvyta is not one of the choices:\n{screen}");
    assert!(screen.contains("qframe"), "every member is on the page at once:\n{screen}");
    assert!(!h.app().picked.iter().any(|picked| *picked), "nothing is checked to begin with");
}

#[test]
fn a_member_that_only_runs_on_arch_linux_cannot_be_checked_elsewhere() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = wizard(root.path());
    to_family_step(&mut h);
    let screen = h.screen();
    for command in ["qtools", "qpac"] {
        assert!(screen.contains(command), "`{command}` is still listed:\n{screen}");
    }
    assert!(screen.contains("Arch Linux only"), "{screen}");
    for key in ["tools", "packages"] {
        assert!(!h.app().choosable(index(key)), "{key} cannot be chosen on Debian");
        h.send(Msg::Wizard(WizardMsg::Pick(index(key), true)));
        assert!(!h.app().picked[index(key)], "{key} stays unchecked");
    }

    let arch = tempfile::tempdir().expect("temp");
    let mut h = start(arch.path(), "ID=arch\n", 80, 30);
    to_family_step(&mut h);
    assert!(!h.screen().contains("Arch Linux only"), "on Arch there is nothing to say:\n{}", h.screen());
    h.send(Msg::Wizard(WizardMsg::Pick(index("tools"), true)));
    assert!(h.app().picked[index("tools")], "on Arch qtools can be chosen");
}

#[test]
fn a_member_that_is_not_out_yet_cannot_be_checked_at_all() {
    for distro in ["ID=debian\n", "ID=arch\n"] {
        let root = tempfile::tempdir().expect("temp");
        let mut h = start(root.path(), distro, 80, 30);
        to_family_step(&mut h);
        let screen = h.screen();
        assert!(screen.contains("qdesk") && screen.contains("Not out yet"), "{distro}:\n{screen}");
        assert!(!h.app().choosable(index("desk")), "{distro}");
        h.send(Msg::Wizard(WizardMsg::Pick(index("desk"), true)));
        assert!(!h.app().picked[index("desk")], "{distro}");
    }
}

#[test]
fn finishing_with_two_members_checked_asks_for_each_in_turn_and_queues_them() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = wizard(root.path());
    packages(root.path());
    scenario(root.path(), "  Installing /x/.cargo/bin/qcode\n", 0);
    to_family_step(&mut h);
    h.send(Msg::Wizard(WizardMsg::Pick(index("code"), true)));
    h.send(Msg::Wizard(WizardMsg::Pick(index("framework"), true)));
    finish(&mut h);
    assert!(!h.app().setting_up(), "the wizard is over:\n{}", h.screen());
    assert_eq!(folder(root.path()), ["launcher.conf", "quvyta.conf"]);
    // The first dialog is the first member of the list that was checked; nothing is installed
    // until it is agreed to.
    assert!(has(&h, "Install qcode?"), "{}", h.screen());
    assert_eq!(h.app().selected, index("code"));
    click_beside(&mut h, "Cancel", "Install");
    h.advance(DIALOG_IN);
    assert!(has(&h, "Install qframe?"), "the next one is asked:\n{}", h.screen());
    click_beside(&mut h, "Cancel", "Install");
    settle(&mut h);
    let installs: Vec<String> =
        calls(root.path()).into_iter().filter(|call| call.starts_with("install --locked")).collect();
    assert_eq!(installs.len(), 2, "both went into the queue: {installs:?}");
    assert!(installs[0].contains("quvyta-code"), "qcode first: {installs:?}");
    assert!(installs[1].contains("quvyta-framework-showcase"), "qframe after it: {installs:?}");
}

#[test]
fn finishing_with_nothing_checked_lands_on_the_family_list() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = wizard(root.path());
    to_family_step(&mut h);
    finish(&mut h);
    assert!(!h.app().setting_up());
    assert!(h.app().installs.dialog.is_none() && h.app().installs.queue.is_empty(), "nothing was asked for");
    let screen = h.screen();
    assert_eq!(h.app().tab, Tab::Apps);
    assert!(screen.contains("qcode") && screen.contains("not installed"), "{screen}");
    assert!(h.is_focused("family"), "the list has the keys:\n{screen}");
}

#[test]
fn the_appearance_chosen_in_the_wizard_is_what_the_application_draws() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = wizard(root.path());
    h.send(Msg::Setup(SetupMsg::Appearance(AppearanceChange::Language("tr".to_owned()))));
    assert!(h.screen().contains("Uygulamalar"), "the wizard turns Turkish at once:\n{}", h.screen());
    to_family_step(&mut h);
    finish(&mut h);
    let screen = h.screen();
    assert!(screen.contains("Uygulamalar") && screen.contains("kurulu değil"), "and stays Turkish:\n{screen}");
    let shared = fs::read_to_string(root.path().join("config/quvyta.conf")).expect("written");
    assert!(shared.contains("language = \"tr\""), "the family keeps it:\n{shared}");
    // The Settings tab carries on from what was chosen, rather than from what was detected.
    h.send(Msg::Tab(Tab::Settings));
    assert!(h.screen().contains("Türkçe"), "{}", h.screen());
}

#[test]
fn turkish_reads_naturally() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = wizard(root.path());
    h.set_locale("tr");
    to_family_step(&mut h);
    let screen = h.screen();
    for text in ["Hangilerini istersin?", "Yalnızca Arch Linux", "Henüz çıkmadı", "Kodlama ajanlarını"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
}

#[test]
fn a_narrow_screen_and_ascii_keep_the_rules() {
    for locale in LANGUAGES {
        let root = tempfile::tempdir().expect("temp");
        let mut h = start(root.path(), "ID=debian\n", 48, 26);
        h.set_locale(locale).set_glyph_mode(GlyphMode::Ascii);
        for step in 0..2 {
            if step == 1 {
                to_family_step(&mut h);
            }
            let screen = h.screen();
            assert!(screen.contains("quvyta"), "{locale} step {step}:\n{screen}");
            for forbidden in ['[', ']', '{', '}', '|', '▌'] {
                assert!(!screen.contains(forbidden), "`{forbidden}` in {locale} step {step}:\n{screen}");
            }
            for line in screen.lines() {
                assert!(qframe::text::width(line) <= 48, "`{line}` is too wide in {locale} step {step}");
            }
        }
    }
}

#[test]
fn no_real_font_folder_is_ever_looked_at() {
    let root = tempfile::tempdir().expect("temp");
    let machine = machine(root.path(), "ID=debian\n");
    assert_eq!(machine.font_dirs, Some(vec![root.path().join("fonts")]), "a test root brings its own fonts");
}

/// The wizard's two steps in both languages, wide and narrow, for the visual review.
pub(in crate::app) fn review() -> Vec<String> {
    let mut fragments = Vec::new();
    for (width, height) in [(100, 30), (48, 26)] {
        for locale in ["en", "tr"] {
            let root = tempfile::tempdir().expect("temp");
            let mut h = start(root.path(), "ID=debian\n", width, height);
            h.set_locale(locale);
            for (step, name) in ["appearance", "applications"].iter().enumerate() {
                if step == 1 {
                    to_family_step(&mut h);
                }
                let title = format!("setup {name} {locale} {width}x{height}");
                fragments.push(h.html(&title));
                println!("{title}\n{}", h.screen());
            }
        }
    }
    fragments
}
