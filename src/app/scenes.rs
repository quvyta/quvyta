//! The scenes of the README: the family list, the question before an install, an install under
//! way and the PATH notice after one, on a made-up machine.
//!
//! The machine lives in a temporary folder whose home is where cargo's folder is, so every path
//! on screen starts with `~`. Its cargo is named by the path a reader expects to see,
//! `~/.cargo/bin/cargo`, which is no program at all: nothing a scene does can start a real
//! cargo, and what cargo installed is told to the application instead of read. An install runs
//! no process either; the scene sends the lines and the ending its task would.

use std::path::{Path, PathBuf};
use std::time::Duration;

use qframe::icons::GlyphMode;
use tempfile::TempDir;

use super::installs::InstallMsg;
use super::tests::{TOAST_IN, env, index};
use super::*;
use crate::cargo::Installed;
use crate::install::Outcome;
use crate::inventory::tests::write_program;

/// The README's size for the list and its details, which end well before 36 rows: a taller
/// screen would show mostly empty rows.
const WIDE_SCENE: (u16, u16) = (120, 24);
/// The size for an install, whose progress, cargo's lines and notices fill the height.
const TALL_SCENE: (u16, u16) = (120, 36);
/// A narrower window, as a terminal split in two leaves it.
const NARROW_SCENE: (u16, u16) = (80, 24);

/// Long enough for a dialog to have popped in.
const DIALOG_IN: Duration = Duration::from_millis(300);

/// What cargo installed on the machine of the family list: member key and version.
const INSTALLED: [(&str, &str); 4] =
    [("code", "0.1.1"), ("focus", "0.1.0"), ("packages", "0.1.0"), ("framework", "0.1.5")];

/// What crates.io answers in the scenes: qfocus has a newer version than the one installed.
const SEARCH: &str = "quvyta-code = \"0.1.1\"\nquvyta-focus = \"0.1.2\"\nquvyta-tools = \"0.1.2\"\nquvyta-packages = \"0.1.0\"\nquvyta-framework-showcase = \"0.1.5\"\nquvyta = \"0.2.0\"\n";

/// What cargo installed before the install scenes: qtools and qpac are still to come.
const BEFORE_INSTALL: [(&str, &str); 3] = [("code", "0.1.1"), ("focus", "0.1.0"), ("framework", "0.1.5")];

/// The cargo output of an install of qtools up to the middle of its build, as the install task
/// hands it over: one line at a time, terminal escapes already gone. The progress line gives the
/// counts; the lines after it fill the tail that Details shows.
const CARGO_LINES: [&str; 17] = [
    "    Updating crates.io index",
    " Downloading crates ...",
    "  Downloaded quvyta-tools v0.1.2",
    "  Downloaded 231 crates (18.4 MB) in 2.31s",
    "    Building [================>           ] 142/231: serde",
    "   Compiling proc-macro2 v1.0.95",
    "   Compiling unicode-ident v1.0.18",
    "   Compiling libc v0.2.172",
    "   Compiling bitflags v2.9.1",
    "   Compiling memchr v2.7.4",
    "   Compiling serde v1.0.219",
    "   Compiling unicode-width v0.2.0",
    "   Compiling rustix v1.0.5",
    "   Compiling crossterm v0.29.0",
    "   Compiling toml v0.8.23",
    "   Compiling quvyta-framework v0.1.5",
    "   Compiling ratatui v0.29.0",
];

/// A scene: the application on its screen, and the folder of its machine, which lives as long
/// as the scene.
pub(super) struct Scene {
    root: TempDir,
    pub(super) h: Harness<Quvyta>,
}

/// How a scene looks: size, theme and language.
#[derive(Clone, Copy)]
pub(super) struct Look {
    size: (u16, u16),
    theme: &'static str,
    locale: &'static str,
}

impl Look {
    const fn new(size: (u16, u16), theme: &'static str, locale: &'static str) -> Self {
        Self { size, theme, locale }
    }
}

/// Every scene with its file name, in the order the README shows them.
pub(super) fn all() -> Vec<(&'static str, Scene)> {
    vec![
        ("family", family(Look::new(WIDE_SCENE, "monochrome", "en"))),
        ("install-confirm", install_confirm(Look::new(WIDE_SCENE, "monochrome", "en"))),
        ("installing", installing(Look::new(TALL_SCENE, "nordic", "en"))),
        ("path-notice", path_notice(Look::new(TALL_SCENE, "iris", "en"))),
        ("family-narrow", family(Look::new(NARROW_SCENE, "iris", "en"))),
        ("family-tr", family(Look::new(WIDE_SCENE, "nordic", "tr"))),
    ]
}

/// A machine in `root` where cargo installed `installed`, with a C linker so the question
/// before an install finds nothing in the way, and what cargo would list for it.
fn machine(root: &Path, installed: &[(&str, &str)]) -> (Machine, Inventory) {
    let mut machine = Machine::in_root(root);
    std::fs::create_dir_all(&machine.home).expect("home");
    write_program(&root.join("bin/cc"), "#!/bin/sh\nexit 0\n");
    machine.cargo = Some(PathBuf::from("~/.cargo/bin/cargo"));
    // A fresh answer from crates.io, so the scenes show one update instead of asking the network.
    let latest = crate::updates::Latest::from_search(SEARCH, crate::updates::now());
    if let Some(path) = crate::updates::cache_path(&machine) {
        std::fs::create_dir_all(path.parent().expect("data folder")).expect("data folder");
        latest.write(&path).expect("latest versions");
    }
    let inventory = listed(&machine, installed);
    (machine, inventory)
}

/// What `installed` looks like on `machine`, with its commands in cargo's folder.
fn listed(machine: &Machine, installed: &[(&str, &str)]) -> Inventory {
    let listed: Vec<Installed> = installed
        .iter()
        .map(|(key, version)| {
            let member = &FAMILY[index(key)];
            write_program(&machine.cargo_bin().join(member.command), "#!/bin/sh\nexit 0\n");
            Installed {
                package: member.package.to_owned(),
                version: (*version).to_owned(),
                commands: vec![member.command.to_owned()],
            }
        })
        .collect();
    Inventory::from_list(machine, &listed)
}

/// The application on its screen in `look`, told what is installed.
fn show(root: TempDir, app: Quvyta, inventory: Inventory, look: Look) -> Scene {
    let (width, height) = look.size;
    let mut h = Harness::with_env(app, env(), width, height);
    h.set_theme(look.theme).set_locale(look.locale).set_glyph_mode(GlyphMode::Nerd);
    // The first frame asked the scene's cargo, which is not there; the answer is told instead.
    h.render().send(Msg::Inventory(inventory));
    Scene { root, h }
}

/// Queues the member `key` as the dialog would: the first one queued becomes the running
/// install, whose task is made but never run.
fn queue(app: &mut Quvyta, key: &str) {
    let index = index(key);
    let _ = app.update(Msg::Install(InstallMsg::Ask(index)));
    let _ = app.update(Msg::Install(InstallMsg::Checked { index, problems: Vec::new(), other_window: false }));
    let _ = app.update(Msg::Install(InstallMsg::Confirm));
}

/// The family with qcode selected: its version, where it is and the button that opens it.
pub(super) fn family(look: Look) -> Scene {
    let root = tempfile::tempdir().expect("temp");
    let (mut machine, inventory) = machine(root.path(), &INSTALLED);
    machine.path.push(machine.cargo_bin());
    show(root, Quvyta::new(machine), inventory, look)
}

/// The question before qtools is installed: source, version, target and command.
pub(super) fn install_confirm(look: Look) -> Scene {
    let root = tempfile::tempdir().expect("temp");
    let (mut machine, inventory) = machine(root.path(), &INSTALLED);
    machine.path.push(machine.cargo_bin());
    let mut scene = show(root, Quvyta::new(machine), inventory, look);
    scene.h.send(Msg::Select(index("tools"))).send(Msg::Install(InstallMsg::Ask(index("tools")))).advance(DIALOG_IN);
    scene
}

/// qtools building with its counts and cargo's own lines under Details, qpac queued after it.
pub(super) fn installing(look: Look) -> Scene {
    let root = tempfile::tempdir().expect("temp");
    let (mut machine, inventory) = machine(root.path(), &BEFORE_INSTALL);
    machine.path.push(machine.cargo_bin());
    let mut app = Quvyta::new(machine);
    app.inventory = Some(inventory.clone());
    queue(&mut app, "tools");
    queue(&mut app, "packages");
    app.selected = index("tools");
    let mut scene = show(root, app, inventory, look);
    let tools = index("tools");
    for line in CARGO_LINES {
        scene.h.send(Msg::Install(InstallMsg::Line(tools, line.to_owned())));
    }
    scene.h.send(Msg::Install(InstallMsg::ToggleDetails));
    scene
}

/// qtools just installed into a folder bash cannot find programs in: the toast, and the notice
/// offering the line that fixes it.
pub(super) fn path_notice(look: Look) -> Scene {
    let root = tempfile::tempdir().expect("temp");
    let (mut machine, inventory) = machine(root.path(), &BEFORE_INSTALL);
    machine.shell = Some("/bin/bash".to_owned());
    let mut app = Quvyta::new(machine);
    app.inventory = Some(inventory.clone());
    queue(&mut app, "tools");
    app.selected = index("tools");
    let mut scene = show(root, app, inventory, look);
    let tools = index("tools");
    let installed = Outcome::Installed { version: Some("0.1.2".to_owned()) };
    scene.h.send(Msg::Install(InstallMsg::Finished { index: tools, outcome: installed, problems: Vec::new() }));
    // The ending asked cargo again, which is not there; the answer is told instead.
    let mut after = BEFORE_INSTALL.to_vec();
    after.push(("tools", "0.1.2"));
    let after = listed(&scene.h.app().machine, &after);
    scene.h.render().send(Msg::Inventory(after)).advance(TOAST_IN);
    scene
}

impl Scene {
    fn screen(&self) -> String {
        self.h.screen()
    }

    /// Asserts that every one of `texts` is on screen.
    fn shows(&self, texts: &[&str]) {
        let screen = self.screen();
        for text in texts {
            assert!(screen.contains(text), "`{text}` is missing:\n{screen}");
        }
    }
}

#[test]
fn the_family_scene_shows_every_member_and_the_selected_one_opens() {
    let scene = family(Look::new(WIDE_SCENE, "monochrome", "en"));
    let running = format!("{}  this application", env!("CARGO_PKG_VERSION"));
    scene.shows(&[
        "qcode",
        "0.1.1",
        "qfocus",
        "qtools",
        "not installed",
        "qpac",
        "qframe",
        "0.1.5",
        &running,
        "Quvyta Code",
        "Installed  0.1.1, ~/.cargo/bin/qcode",
        "Open",
    ]);
    assert!(super::tests::line_with(&scene.screen(), "qcode").contains('▌'), "{}", scene.screen());
}

#[test]
fn the_narrow_and_turkish_family_scenes_keep_the_list_beside_the_details() {
    let narrow = family(Look::new(NARROW_SCENE, "iris", "en"));
    narrow.shows(&["qtools", "not installed", "Quvyta Code", "~/.cargo/bin/qcode", "Open"]);
    let turkish = family(Look::new(WIDE_SCENE, "nordic", "tr"));
    turkish.shows(&["Özenle yapılmış terminal uygulamaları", "kurulu değil", "bu uygulama", "Kurulu", "Aç"]);
}

#[test]
fn the_install_confirm_scene_asks_with_everything_it_will_do() {
    let scene = install_confirm(Look::new(WIDE_SCENE, "monochrome", "en"));
    scene.shows(&[
        "Install qtools?",
        "Source   crates.io, quvyta-tools",
        "Version  0.1.2",
        "Target   ~/.cargo/bin/qtools",
        "~/.cargo/bin/cargo install --locked quvyta-tools --vers",
        "No sudo needed; it writes only under ~/.cargo.",
        "Cancel",
    ]);
}

#[test]
fn the_installing_scene_shows_the_phase_the_counts_and_the_queue() {
    let scene = installing(Look::new(TALL_SCENE, "nordic", "en"));
    scene.shows(&["qtools installing", "Compiling", "142 / 231", "ratatui", "Stop", "Compiling serde v1.0.219"]);
    let screen = scene.screen();
    assert!(super::tests::line_with(&screen, "qtools ").contains("installing 61%"), "{screen}");
    assert!(super::tests::line_with(&screen, "qpac ").contains("queued"), "{screen}");
}

#[test]
fn the_path_notice_scene_names_the_file_and_the_line() {
    let scene = path_notice(Look::new(TALL_SCENE, "iris", "en"));
    scene.shows(&[
        "qtools 0.1.2 installed",
        "~/.cargo/bin is not on PATH",
        "For qtools to open when you type its name",
        "~/.bashrc",
        "export PATH=\"$HOME/.cargo/bin:$PATH\"",
        "Add",
    ]);
    assert!(super::tests::line_with(&scene.screen(), "qtools ").contains("0.1.2"), "{}", scene.screen());
}

#[test]
fn no_scene_shows_where_its_machine_really_is() {
    for (name, scene) in all() {
        let screen = scene.screen();
        let root = scene.root.path().display().to_string();
        assert!(!screen.contains(&root) && !screen.contains("/tmp"), "{name}:\n{screen}");
        assert!(!screen.contains('⟦'), "a key is missing in {name}:\n{screen}");
        assert!(scene.h.handoffs().is_empty(), "{name} opened something");
    }
}
