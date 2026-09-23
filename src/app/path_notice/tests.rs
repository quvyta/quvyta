use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use qframe::icons::GlyphMode;

use super::super::tests::{TOAST_IN, env, machine_off_path};
use super::*;
use crate::shell_path::COMMENT;

/// The line bash and zsh get for cargo's default folder.
const EXPORT: &str = "export PATH=\"$HOME/.cargo/bin:$PATH\"";

/// The machine in `root` without cargo's folder on `PATH`, using `shell`.
fn machine(root: &Path, shell: &str) -> Machine {
    let mut machine = machine_off_path(root);
    machine.shell = Some(shell.to_owned());
    machine
}

/// The application on `machine` once what is installed is read and the PATH check has answered.
fn start(machine: Machine) -> Harness<Quvyta> {
    let mut h = Harness::with_env(Quvyta::new(machine), env(), 100, 30);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    // One frame hears cargo, which starts the check; the next hears the check.
    h.render().render();
    h
}

fn home(root: &Path) -> std::path::PathBuf {
    root.join("home")
}

#[test]
fn with_cargo_s_folder_on_path_nothing_is_said() {
    let root = tempfile::tempdir().expect("temp");
    let mut machine = machine(root.path(), "/bin/bash");
    machine.path.push(machine.cargo_bin());
    let screen = start(machine).screen();
    assert!(!screen.contains("PATH") && !screen.contains("Terminals"), "{screen}");
}

#[test]
fn nothing_is_checked_while_cargo_installed_nothing() {
    let root = tempfile::tempdir().expect("temp");
    let mut machine = machine(root.path(), "/bin/bash");
    // cargo answers with an empty list and the Quvyta apps' files are gone.
    fs::write(root.path().join("bin/list.out"), "").expect("scenario");
    fs::remove_dir_all(machine.cargo_bin()).expect("no programs");
    machine.path.clear();
    let screen = start(machine).screen();
    assert!(!screen.contains("PATH"), "{screen}");
}

#[test]
fn a_start_up_file_that_already_adds_the_folder_gets_a_quiet_line() {
    let root = tempfile::tempdir().expect("temp");
    fs::create_dir_all(home(root.path())).expect("home");
    fs::write(home(root.path()).join(".bashrc"), ". \"$HOME/.cargo/env\"\n").expect("bashrc");
    let screen = start(machine(root.path(), "/bin/bash")).screen();
    assert!(screen.contains("Terminals you open from now on can start the programs"), "{screen}");
    assert!(!screen.contains("not on PATH") && !screen.contains("Not now"), "{screen}");
}

#[test]
fn each_known_shell_is_offered_its_own_file_and_line() {
    for (shell, file, line) in [
        ("/bin/bash", "~/.bashrc", EXPORT),
        ("/usr/bin/zsh", "~/.zshrc", EXPORT),
        ("/usr/bin/fish", "~/.config/fish/config.fish", "fish_add_path $HOME/.cargo/bin"),
    ] {
        let root = tempfile::tempdir().expect("temp");
        let screen = start(machine(root.path(), shell)).screen();
        for text in [
            "~/.cargo/bin is not on PATH",
            "For the programs cargo installs to open when you",
            "Opening",
            "from quvyta is not affected.",
            &format!("File  {file}"),
            line,
            "Not now",
            "Add",
        ] {
            assert!(screen.contains(text), "{shell}: `{text}` is missing:\n{screen}");
        }
    }
}

#[test]
fn zsh_follows_zdotdir() {
    let root = tempfile::tempdir().expect("temp");
    let mut machine = machine(root.path(), "/usr/bin/zsh");
    machine.zdotdir = Some(home(root.path()).join(".config/zsh"));
    let screen = start(machine).screen();
    assert!(screen.contains("File  ~/.config/zsh/.zshrc"), "{screen}");
}

#[test]
fn an_unknown_shell_shows_the_line_to_add_by_hand_and_no_add_button() {
    let root = tempfile::tempdir().expect("temp");
    let screen = start(machine(root.path(), "/bin/tcsh")).screen();
    for text in ["~/.cargo/bin is not on PATH", "add this line to", "your shell's start-up file", EXPORT] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("Add") && !screen.contains("File "), "{screen}");
}

#[test]
fn the_line_copies_with_a_click() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = start(machine(root.path(), "/bin/bash"));
    h.click_text(EXPORT);
    assert_eq!(h.copied(), [EXPORT]);
}

#[test]
fn add_appends_the_installer_s_three_lines_and_says_so() {
    let root = tempfile::tempdir().expect("temp");
    let bashrc = home(root.path()).join(".bashrc");
    fs::create_dir_all(home(root.path())).expect("home");
    fs::write(&bashrc, "alias ll='ls -l'\n").expect("bashrc");
    let mut h = start(machine(root.path(), "/bin/bash"));
    h.click_text("Add").render();
    assert_eq!(fs::read_to_string(&bashrc).expect("bashrc"), format!("alias ll='ls -l'\n\n{COMMENT}\n{EXPORT}\n"));
    let screen = h.screen();
    for text in
        ["Added. Terminals you open from now on can start the", "Terminals that are already open are not affected."]
    {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("not on PATH") && !screen.contains("Not now"), "{screen}");
    // A second press, from a key still queued, adds nothing more.
    h.send(Msg::Path(PathMsg::Add)).render();
    assert_eq!(fs::read_to_string(&bashrc).expect("bashrc").matches(COMMENT).count(), 1);
}

#[test]
fn after_an_install_the_notice_names_the_member() {
    let root = tempfile::tempdir().expect("temp");
    let machine = machine(root.path(), "/usr/bin/fish");
    let action = shell_path::decide(&machine.shell_env(), shell_path::read_file);
    let tools = APPS.iter().position(|member| member.key == "tools").expect("qtools");
    let mut h = start(machine);
    h.send(Msg::Path(PathMsg::Checked { installed: Some(tools), action, asked: false }));
    assert!(h.screen().contains("For qtools to open when you type its name in a"), "{}", h.screen());
    h.click_text("Add").render();
    assert!(h.screen().contains("Added. Terminals you open from now on can start qtools by"), "{}", h.screen());
    let config = home(root.path()).join(".config/fish/config.fish");
    assert_eq!(
        fs::read_to_string(config).expect("created with its folder"),
        format!("\n{COMMENT}\nfish_add_path $HOME/.cargo/bin\n")
    );
}

#[test]
fn a_read_only_file_says_why_and_keeps_the_line_to_copy() {
    let root = tempfile::tempdir().expect("temp");
    let bashrc = home(root.path()).join(".bashrc");
    fs::create_dir_all(home(root.path())).expect("home");
    fs::write(&bashrc, "").expect("bashrc");
    fs::set_permissions(&bashrc, fs::Permissions::from_mode(0o444)).expect("read-only");
    let mut h = start(machine(root.path(), "/bin/bash"));
    h.click_text("Add").render();
    let screen = h.screen();
    for text in ["The line could not be added", "~/.bashrc: Permission denied", EXPORT, "Not now"] {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    assert!(!screen.contains("Add"), "trying again would fail the same way:\n{screen}");
    assert_eq!(fs::read_to_string(&bashrc).expect("bashrc"), "");
    h.click_text(EXPORT);
    assert_eq!(h.copied(), [EXPORT]);
}

#[test]
fn not_now_is_remembered_and_keeps_the_other_settings() {
    let root = tempfile::tempdir().expect("temp");
    let conf = root.path().join("config/launcher.conf");
    fs::create_dir_all(root.path().join("config")).expect("folder");
    fs::write(&conf, "after_close = \"shell\"\n").expect("settings");
    let mut h = start(machine(root.path(), "/bin/bash"));
    h.click_text("Not now").render();
    assert!(!h.screen().contains("not on PATH"), "{}", h.screen());
    let saved = Launcher::load(Some(&conf));
    assert_eq!(saved.path_prompt, PathPrompt::Dismissed);
    assert_eq!(saved.after_close, crate::launcher::AfterClose::Shell);
    assert!(fs::read_to_string(&conf).expect("settings").contains("path_prompt = \"dismissed\""));

    let h = start(machine(root.path(), "/bin/bash"));
    assert!(!h.screen().contains("not on PATH"), "the offer does not come back:\n{}", h.screen());
    assert!(!home(root.path()).join(".bashrc").exists(), "nothing was written");
}

#[test]
fn not_now_that_cannot_be_saved_says_so() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = start(machine(root.path(), "/bin/bash"));
    // A settings folder that takes nothing new: launcher.conf is read, but the answer cannot be
    // written back into it.
    let config = root.path().join("config");
    fs::set_permissions(&config, fs::Permissions::from_mode(0o500)).expect("blocker");
    h.click_text("Not now").render();
    let screen = h.advance(TOAST_IN).screen();
    // The folder is let go again, so the temporary folder can be cleaned up.
    fs::set_permissions(&config, fs::Permissions::from_mode(0o700)).expect("released");
    assert!(screen.contains("launcher.conf could not remember the answer"), "{screen}");
    assert!(!screen.contains("not on PATH"), "the notice still goes for now:\n{screen}");
}

#[test]
fn turkish_reads_naturally_and_ascii_keeps_the_rules() {
    let root = tempfile::tempdir().expect("temp");
    let mut h = start(machine(root.path(), "/bin/bash"));
    h.set_locale("tr");
    let screen = h.screen();
    for text in ["~/.cargo/bin PATH'te değil", "quvyta'dan açmak bundan", "Dosya  ~/.bashrc", "Şimdi değil", "Ekle"]
    {
        assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
    }
    h.set_glyph_mode(GlyphMode::Ascii);
    for (width, height) in [(40, 30), (100, 30)] {
        h.resize(width, height).send(Msg::ShowDetail(0));
        let screen = h.screen();
        for forbidden in ['[', ']', '{', '}', '|', '▌', '⟦'] {
            assert!(!screen.contains(forbidden), "`{forbidden}` at {width}x{height}:\n{screen}");
        }
    }
}

/// The notice in each of its forms, in both languages, wide and narrow, for the visual review.
pub(in crate::app) fn review() -> Vec<String> {
    let mut fragments = Vec::new();
    for locale in ["en", "tr"] {
        for (name, shell, width) in
            [("add", "/usr/bin/fish", 100), ("add narrow", "/bin/bash", 48), ("unknown shell", "/bin/tcsh", 100)]
        {
            let root = tempfile::tempdir().expect("temp");
            let mut h = start(machine(root.path(), shell));
            h.set_locale(locale).resize(width, 30);
            if width < 60 {
                h.send(Msg::ShowDetail(0));
            }
            fragments.push(h.html(&format!("PATH {name} {locale}")));
            println!("PATH {name} {locale}\n{}", h.screen());
        }
        let root = tempfile::tempdir().expect("temp");
        let mut h = start(machine(root.path(), "/bin/bash"));
        h.set_locale(locale).click_text(EXPORT).click_text(if locale == "en" { "Add" } else { "Ekle" }).render();
        fragments.push(h.html(&format!("PATH added {locale}")));
        println!("PATH added {locale}\n{}", h.screen());

        let root = tempfile::tempdir().expect("temp");
        let bashrc = home(root.path()).join(".bashrc");
        fs::create_dir_all(home(root.path())).expect("home");
        fs::write(&bashrc, "").expect("bashrc");
        fs::set_permissions(&bashrc, fs::Permissions::from_mode(0o444)).expect("read-only");
        let mut h = start(machine(root.path(), "/bin/bash"));
        h.set_locale(locale).click_text(if locale == "en" { "Add" } else { "Ekle" }).render();
        fragments.push(h.html(&format!("PATH failed {locale}")));
        println!("PATH failed {locale}\n{}", h.screen());

        let root = tempfile::tempdir().expect("temp");
        fs::create_dir_all(home(root.path())).expect("home");
        fs::write(home(root.path()).join(".bashrc"), ". \"$HOME/.cargo/env\"\n").expect("bashrc");
        let mut h = start(machine(root.path(), "/bin/bash"));
        h.set_locale(locale);
        fragments.push(h.html(&format!("PATH already configured {locale}")));
        println!("PATH already configured {locale}\n{}", h.screen());
    }
    fragments
}
