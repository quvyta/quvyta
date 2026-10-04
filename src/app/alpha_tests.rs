//! A member that is out only as a pre-release: cargo installs it only when its version is named,
//! so the question before installing names the version, and the command it shows is the very
//! command that runs, whether or not crates.io was asked for updates before.
//!
//! Every test stands in a temporary root with the stand-in cargo; nothing is installed.

use super::install_tests::{DIALOG_IN, calls, click_beside, has, run, settle};
use super::tests::{index, machine};
use super::*;
use crate::install::tests::packages;
use crate::updates::tests::search_scenario;

const CLI: &str = "quvyta-cli = \"0.1.0-alpha.3\"    # A small coding agent in the terminal (alpha)\n";

#[test]
fn the_question_about_an_alpha_shows_the_command_that_runs() {
    let root = crate::app::tests::temp_root();
    let app = Quvyta::new(machine(root.path()));
    // The check at start knew nothing of qcli: it failed, or the shared update notice is off.
    search_scenario(root.path(), "", 1);
    packages(root.path());
    let mut h = run(app, 120, 40);
    settle(&mut h);
    // crates.io answers now, when the question asks.
    search_scenario(root.path(), CLI, 0);
    for _ in 0..index("cli") {
        h.press("down");
    }
    h.press("enter").advance(DIALOG_IN);
    settle(&mut h);
    let screen = h.screen();
    println!("{screen}");
    assert!(has(&h, "Install qcli?"), "{screen}");
    let cargo = root.path().join("bin/cargo").display().to_string();
    let command = format!("{cargo} install --locked quvyta-cli --version 0.1.0-alpha.3");
    let squeezed = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(squeezed(&screen).contains(&squeezed(&command)), "the version is named in the command:\n{screen}");
    assert!(screen.contains("Version  0.1.0-alpha.3"), "and in the version line:\n{screen}");

    click_beside(&mut h, "Cancel", "Install");
    settle(&mut h);
    let ran: Vec<String> = calls(root.path()).into_iter().filter(|call| call.starts_with("install --locked")).collect();
    assert_eq!(ran, ["install --locked quvyta-cli --version 0.1.0-alpha.3"], "what ran is what was shown");
}
