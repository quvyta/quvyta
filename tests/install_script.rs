//! Runs the checks of `install.sh`, the one-line installer, so the commit gate covers it too.
//!
//! The checks themselves are a shell script: they run the installer in temporary home folders
//! with stand-ins for cargo and rustup, so nothing is downloaded or installed.

use std::path::Path;
use std::process::Command;

use quvyta::{APPS, Status};

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

/// Reads one quoted field such as `Crate = 'quvyta-code'` from a line of `Get-QuvytaApps`.
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

/// The body of one function of `install.sh`, from its opening line to the closing brace.
fn sh_body<'a>(script: &'a str, function: &str) -> &'a str {
    let body =
        &script[script.find(&format!("{function}() {{")).unwrap_or_else(|| panic!("no {function} in install.sh"))..];
    &body[..body.find("\n}").expect("the function is closed")]
}

/// The text `install.sh` prints for a member from one of its `case` functions, such as
/// `code) echo qcode ;;` or `code) echo "coding ..." ;;`.
fn sh_case(script: &str, function: &str, name: &str) -> Option<String> {
    sh_body(script, function).lines().find_map(|line| {
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
fn both_installers_know_the_same_apps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sh = std::fs::read_to_string(root.join("install.sh")).expect("install.sh is readable");
    let ps1 = std::fs::read_to_string(root.join("install.ps1")).expect("install.ps1 is readable");
    let members: Vec<&str> = ps1.lines().filter(|line| line.contains("[pscustomobject]@{ Name = ")).collect();
    let names: Vec<String> = members.iter().map(|line| ps1_field(line, "Name")).collect();
    let sh_names = sh.lines().find_map(|line| line.strip_prefix("names=\"")).expect("install.sh lists names");
    assert_eq!(names.join(" "), sh_names.trim_end_matches('"'), "the same members in the same order");

    let arch_only = sh_body(&sh, "arch_only")
        .lines()
        .map(str::trim)
        .find(|line| line.ends_with(") return 0 ;;"))
        .expect("install.sh marks Arch-only members");
    let soon = sh
        .lines()
        .find_map(|line| line.strip_prefix("soon=\""))
        .expect("install.sh lists the members that are not released yet")
        .trim_end_matches('"');
    for name in soon.split_whitespace() {
        assert!(names.iter().any(|known| known == name), "{name} is not a Quvyta app");
    }
    for line in members {
        let name = ps1_field(line, "Name");
        for (field, function) in [("Crate", "crate_of"), ("Command", "command_of"), ("About", "about")] {
            assert_eq!(Some(ps1_field(line, field)), sh_case(&sh, function, &name), "{field} of {name}");
        }
        let sh_arch = arch_only.trim_end_matches(") return 0 ;;").split('|').any(|word| word.trim() == name);
        assert_eq!(ps1_field(line, "ArchOnly") == "true", sh_arch, "whether {name} is Arch-only");
        let sh_soon = soon.split_whitespace().any(|word| word == name);
        assert_eq!(ps1_field(line, "Soon") == "true", sh_soon, "whether {name} is not released yet");
        // The screen knows the same members the two installers do: the setup wizard offers a
        // member that runs on Arch Linux only nowhere else, as they install it nowhere else.
        let member = APPS.iter().find(|member| member.key == name).expect("the screen knows {name}");
        assert_eq!(member.arch_only, sh_arch, "whether {name} runs on Arch Linux only");
        assert_eq!(member.status == Status::Soon, sh_soon, "whether {name} is out");
    }
}

/// The quoted text after `Self::<variant> => ` in one `match` of `src/checks.rs`, such as
/// `Self::Arch => Some("sudo pacman ...")`, found inside the function that opens with `function`.
fn rs_arm(source: &str, function: &str, variant: &str) -> String {
    let body = &source[source.find(function).unwrap_or_else(|| panic!("no {function} in src/checks.rs"))..];
    let line = body
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(&format!("Self::{variant} => ")))
        .unwrap_or_else(|| panic!("no {variant} arm in {function}"));
    let start = line.find('"').expect("a quoted value") + 1;
    line[start..start + line[start..].find('"').expect("the value is closed")].to_owned()
}

/// install.sh offers to run the command that installs a C linker; the screen shows the same one
/// for the same distribution, so the two cannot name different packages.
#[test]
fn installer_and_screen_name_the_same_linker_commands() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sh = std::fs::read_to_string(root.join("install.sh")).expect("install.sh is readable");
    let rs = std::fs::read_to_string(root.join("src/checks.rs")).expect("src/checks.rs is readable");
    let known = rs
        .lines()
        .find_map(|line| line.trim().strip_prefix("pub const KNOWN: [Self; 3] = ["))
        .expect("src/checks.rs lists the known distributions");
    let variants: Vec<&str> =
        known.trim_end_matches("];").split(',').map(|v| v.trim().trim_start_matches("Self::")).collect();
    let sh_known = sh_body(&sh, "say_linker_commands")
        .lines()
        .find_map(|line| line.trim().strip_prefix("for known in "))
        .expect("install.sh lists the known distributions")
        .trim_end_matches("; do");
    assert_eq!(variants.len(), sh_known.split_whitespace().count(), "the same number of distributions");
    for (variant, id) in variants.iter().zip(sh_known.split_whitespace()) {
        assert_eq!(
            sh_case(&sh, "linker_command", id),
            Some(rs_arm(&rs, "fn linker_command", variant)),
            "the linker command for {variant}"
        );
        assert_eq!(sh_case(&sh, "distro_name", id), Some(rs_arm(&rs, "fn name", variant)), "the name of {variant}");
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
