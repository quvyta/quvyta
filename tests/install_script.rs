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

/// Reads one quoted field such as `Crate = 'quvyta-code'` from a line of `Get-QuvytaFamily`.
fn ps1_field(line: &str, field: &str) -> String {
    let start = line.find(&format!("{field} = ")).unwrap_or_else(|| panic!("no {field} in: {line}")) + field.len() + 3;
    let rest = &line[start..];
    let quote = rest.chars().next().expect("a value follows");
    if quote == '$' {
        return rest[1..].split(';').next().expect("a value").trim().to_owned();
    }
    let body = &rest[1..];
    body[..body.find(quote).expect("the value is closed")].to_owned()
}

/// The text `install.sh` prints for a member from one of its `case` functions, such as
/// `code) echo qcode ;;` or `code) echo "coding ..." ;;`.
fn sh_case(script: &str, function: &str, name: &str) -> Option<String> {
    let body = &script[script.find(&format!("{function}() {{"))?..];
    let body = &body[..body.find("\n}")?];
    body.lines().find_map(|line| {
        let line = line.trim();
        let (pattern, action) = line.split_once(") echo ")?;
        let matches = pattern.split('|').any(|word| word.trim() == name || word.trim() == "*");
        if !matches {
            return None;
        }
        let value = action.trim_end_matches(";;").trim().trim_matches('"');
        Some(value.replace("quvyta-$1", &format!("quvyta-{name}")))
    })
}

#[test]
fn both_installers_know_the_same_family() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sh = std::fs::read_to_string(root.join("install.sh")).expect("install.sh is readable");
    let ps1 = std::fs::read_to_string(root.join("install.ps1")).expect("install.ps1 is readable");
    let members: Vec<&str> = ps1.lines().filter(|line| line.contains("[pscustomobject]@{ Name = ")).collect();
    let names: Vec<String> = members.iter().map(|line| ps1_field(line, "Name")).collect();
    let sh_names = sh.lines().find_map(|line| line.strip_prefix("names=\"")).expect("install.sh lists names");
    assert_eq!(names.join(" "), sh_names.trim_end_matches('"'), "the same members in the same order");

    let arch_only = sh
        .lines()
        .map(str::trim)
        .find(|line| line.ends_with(") return 0 ;;"))
        .expect("install.sh marks Arch-only members");
    for line in members {
        let name = ps1_field(line, "Name");
        for (field, function) in [("Crate", "crate_of"), ("Command", "command_of"), ("About", "about")] {
            assert_eq!(Some(ps1_field(line, field)), sh_case(&sh, function, &name), "{field} of {name}");
        }
        let sh_arch = arch_only.trim_end_matches(") return 0 ;;").split('|').any(|word| word.trim() == name);
        assert_eq!(ps1_field(line, "ArchOnly") == "true", sh_arch, "whether {name} is Arch-only");
    }
}

/// Runs the checks of `install.ps1` in a PowerShell container. It needs podman and the image
/// `mcr.microsoft.com/powershell:7.5-ubuntu-24.04`, so it is run by hand with `--ignored`.
#[test]
#[ignore = "needs podman and a PowerShell image"]
fn install_ps1_cases_pass() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("sh")
        .arg(root.join("tests/install-ps1/run.sh"))
        .output()
        .expect("sh runs the PowerShell installer checks");
    assert!(
        output.status.success(),
        "PowerShell installer checks failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
