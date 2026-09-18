//! Runs the checks of `install.sh`, the one-line installer, so the commit gate covers it too.
//!
//! The checks themselves are a shell script: they run the installer in temporary home folders
//! with stand-ins for cargo and rustup, so nothing is downloaded or installed.

use std::path::Path;
use std::process::Command;

#[test]
fn install_script_cases_pass() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output =
        Command::new("sh").arg(root.join("tests/install/run.sh")).output().expect("sh runs the installer checks");
    assert!(
        output.status.success(),
        "installer checks failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
