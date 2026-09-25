//! The command line: what quvyta was asked to do before its screen opens.
//!
//! Without arguments quvyta opens the list of Quvyta apps. `install` opens it straight into the install
//! dialog of the members named, one after another, so nothing is installed without the same
//! consent the screen asks for. `show` opens it on one member's page, so another program can
//! send its user to that member. `--help` and `--version` answer on standard output; anything
//! else is a mistake, told on standard error with exit code 2 and without opening the screen.
//!
//! The words come from the language files, in the language the screen would use.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use qframe::i18n::I18n;
use qframe::t;

use crate::ecosystem::APPS;

/// What quvyta was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    /// Opens the screen.
    Open(Start),
    /// Prints the usage.
    Help,
    /// Prints the version.
    Version,
    /// The arguments ask for something quvyta does not do.
    Wrong(Mistake),
}

/// How the screen starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// On the list of Quvyta apps.
    List,
    /// Asking to install these members, indexes of [`APPS`] in the order named.
    Install(Vec<usize>),
    /// On the page of this member, an index of [`APPS`].
    Show(usize),
}

/// What is wrong with the arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mistake {
    /// An option quvyta does not have.
    Option(String),
    /// A name no member goes by.
    Name(String),
    /// A first word that is not something quvyta does.
    Command(String),
    /// `install` without a name.
    NoNames,
    /// `install` naming a member that is not released yet, an index of [`APPS`].
    Unreleased(usize),
    /// `show` without a name.
    NoShowName,
    /// `show` with more than one name: there is one page to open.
    ShowMany,
}

/// Reads the arguments, without the program's own name.
///
/// `--help` anywhere wins, then `--version`, so either answers whatever else was typed.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Parsed {
    // A name that is not UTF-8 matches no member; shown as well as it can be, it says which.
    let args: Vec<String> = args.into_iter().map(|arg| arg.to_string_lossy().into_owned()).collect();
    if args.iter().any(|arg| matches!(arg.as_str(), "-h" | "--help")) {
        return Parsed::Help;
    }
    if args.iter().any(|arg| matches!(arg.as_str(), "-V" | "--version")) {
        return Parsed::Version;
    }
    if let Some(option) = args.iter().find(|arg| arg.len() > 1 && arg.starts_with('-')) {
        return Parsed::Wrong(Mistake::Option(option.clone()));
    }
    let Some((first, names)) = args.split_first() else { return Parsed::Open(Start::List) };
    match first.as_str() {
        "install" => install(names),
        "show" => show(names),
        _ => Parsed::Wrong(Mistake::Command(first.clone())),
    }
}

/// `install` with `names`.
fn install(names: &[String]) -> Parsed {
    if names.is_empty() {
        return Parsed::Wrong(Mistake::NoNames);
    }
    let mut members = Vec::new();
    for name in names {
        let Some(index) = member(name) else { return Parsed::Wrong(Mistake::Name(name.clone())) };
        // The whole line is refused, as for a wrong name: the ones that can be installed are
        // better asked for again than half of what was typed started.
        if !APPS[index].published() {
            return Parsed::Wrong(Mistake::Unreleased(index));
        }
        // Naming a member twice asks once.
        if !members.contains(&index) {
            members.push(index);
        }
    }
    Parsed::Open(Start::Install(members))
}

/// `show` with `names`. Every member has a page, the ones not released yet and quvyta too.
fn show(names: &[String]) -> Parsed {
    match names {
        [] => Parsed::Wrong(Mistake::NoShowName),
        [name] => match member(name) {
            Some(index) => Parsed::Open(Start::Show(index)),
            None => Parsed::Wrong(Mistake::Name(name.clone())),
        },
        _ => Parsed::Wrong(Mistake::ShowMany),
    }
}

/// The member that goes by `name`: its short name, its command or its package, in any case.
pub(crate) fn member(name: &str) -> Option<usize> {
    APPS.iter().position(|member| {
        [member.key, member.command, member.package].iter().any(|known| known.eq_ignore_ascii_case(name))
    })
}

/// The language files over the framework's, in the language of the environment `lookup` reads,
/// as the screen would pick it.
pub fn i18n(lookup: impl Fn(&str) -> Option<String>) -> I18n {
    let mut i18n = I18n::builtin();
    for (file, text) in crate::LOCALES {
        i18n.add_source(file, text);
    }
    if let Some(code) = i18n.detect(lookup) {
        i18n.set_active(&code);
    }
    i18n
}

/// What to do with what was parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Open the screen.
    Start(Start),
    /// Print `text` on standard output and end well.
    Say(String),
    /// Print `text` on standard error and end with exit code 2.
    Refuse(String),
}

impl Answer {
    /// The answer to `parsed`, in the language of the translator in scope.
    pub fn to(parsed: Parsed) -> Self {
        match parsed {
            Parsed::Open(start) => Self::Start(start),
            Parsed::Help => Self::Say(usage()),
            Parsed::Version => Self::Say(format!("quvyta {}\n", env!("CARGO_PKG_VERSION"))),
            Parsed::Wrong(mistake) => {
                let what = match mistake {
                    Mistake::Option(option) => t!("cli.unknown-option", option = option),
                    Mistake::Name(name) => t!("cli.unknown-name", name = name),
                    Mistake::Command(command) => t!("cli.unknown-command", command = command),
                    Mistake::NoNames => t!("cli.no-names"),
                    Mistake::NoShowName => t!("cli.show-no-name"),
                    Mistake::ShowMany => t!("cli.show-many"),
                    // The help has nothing to add: the name was right, the member is only not
                    // out yet.
                    Mistake::Unreleased(index) => {
                        let command = APPS.get(index).map_or("", |member| member.command);
                        return Self::Refuse(format!("quvyta: {}\n", t!("cli.unreleased", command = command)));
                    }
                };
                Self::Refuse(format!("quvyta: {what}\n{}\n", t!("cli.hint")))
            }
        }
    }

    /// The exit code of an answer that ends quvyta; `None` when the screen opens.
    pub fn code(&self) -> Option<ExitCode> {
        match self {
            Self::Start(_) => None,
            Self::Say(_) => Some(ExitCode::SUCCESS),
            Self::Refuse(_) => Some(ExitCode::from(2)),
        }
    }

    /// Prints the text of the answer where it belongs.
    pub fn print(&self) {
        // A closed pipe, as `quvyta --help | head -1` leaves, is no reason to fail.
        let _ = match self {
            Self::Start(_) => Ok(()),
            Self::Say(text) => std::io::stdout().lock().write_all(text.as_bytes()),
            Self::Refuse(text) => std::io::stderr().lock().write_all(text.as_bytes()),
        };
    }
}

/// The usage: what quvyta is, its forms with what each does, and what a name may be.
fn usage() -> String {
    let forms = [
        ("quvyta".to_owned(), t!("cli.list")),
        (format!("quvyta install {}...", t!("cli.name")), t!("cli.install")),
        (format!("quvyta show {}", t!("cli.name")), t!("cli.show")),
        ("quvyta --help, -h".to_owned(), t!("cli.help")),
        ("quvyta --version, -V".to_owned(), t!("cli.version")),
    ];
    let width = forms.iter().map(|(form, _)| form.chars().count()).max().unwrap_or(0);
    let mut text = format!("quvyta {}\n{}\n\n{}\n", env!("CARGO_PKG_VERSION"), t!("cli.about"), t!("cli.usage"));
    for (form, what) in forms {
        text.push_str(&format!("  {form:width$}  {what}\n"));
    }
    text.push('\n');
    text.push_str(&t!("cli.names"));
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn index(key: &str) -> usize {
        APPS.iter().position(|member| member.key == key).expect("a member")
    }

    fn parsed(args: &[&str]) -> Parsed {
        parse(args.iter().map(OsString::from))
    }

    fn install(keys: &[&str]) -> Parsed {
        Parsed::Open(Start::Install(keys.iter().map(|key| index(key)).collect()))
    }

    fn wrong(mistake: Mistake) -> Parsed {
        Parsed::Wrong(mistake)
    }

    /// The answer to `args` in `language`, as a terminal set to it would get.
    fn answer(language: &str, args: &[&str]) -> Answer {
        let lang = format!("{language}_{}.UTF-8", language.to_uppercase());
        let i18n = Arc::new(i18n(|name| (name == "LANG").then(|| lang.clone())));
        qframe::i18n::scope(i18n, || Answer::to(parsed(args)))
    }

    #[test]
    fn every_form_of_the_command_line_is_read() {
        let option = |text: &str| wrong(Mistake::Option(text.to_owned()));
        let cases: [(&[&str], Parsed); 21] = [
            (&[], Parsed::Open(Start::List)),
            (&["--help"], Parsed::Help),
            (&["-h"], Parsed::Help),
            (&["install", "--help"], Parsed::Help),
            (&["--version", "--help"], Parsed::Help),
            (&["--version"], Parsed::Version),
            (&["-V"], Parsed::Version),
            (&["install", "qcode", "-V"], Parsed::Version),
            (&["install", "code"], install(&["code"])),
            (&["install", "qcode"], install(&["code"])),
            (&["install", "quvyta-code"], install(&["code"])),
            (&["install", "QPAC"], install(&["packages"])),
            (&["install", "qframe", "framework", "quvyta-framework-showcase"], install(&["framework"])),
            (&["install", "tools", "qpac", "quvyta"], install(&["tools", "packages", "quvyta"])),
            (&["install"], wrong(Mistake::NoNames)),
            (&["install", "qcode", "qcod"], wrong(Mistake::Name("qcod".to_owned()))),
            (&["install", "quvyta-framework"], wrong(Mistake::Name("quvyta-framework".to_owned()))),
            (&["qcode"], wrong(Mistake::Command("qcode".to_owned()))),
            (&["--frobnicate"], option("--frobnicate")),
            (&["install", "-y", "qcode"], option("-y")),
            (&["-"], wrong(Mistake::Command("-".to_owned()))),
        ];
        for (args, expected) in cases {
            assert_eq!(parsed(args), expected, "{args:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_name_that_is_not_utf8_is_a_wrong_name() {
        use std::os::unix::ffi::OsStringExt;
        let args = [OsString::from("install"), OsString::from_vec(vec![b'q', 0xff])];
        assert_eq!(parse(args), wrong(Mistake::Name("q\u{fffd}".to_owned())));
    }

    #[test]
    fn help_lists_every_form_in_columns() {
        let Answer::Say(text) = answer("en", &["--help"]) else { panic!("help is said") };
        let version = env!("CARGO_PKG_VERSION");
        let expected = format!(
            "\
quvyta {version}
Installs, opens and updates the terminal applications of the Quvyta ecosystem.

Usage
  quvyta                  open the list of Quvyta apps
  quvyta install NAME...  ask to install these members, one dialog after another
  quvyta show NAME        open on this member's page
  quvyta --help, -h       show this help
  quvyta --version, -V    show the version

A name is a member's short name (code), its command (qcode) or its package (quvyta-code).
"
        );
        assert_eq!(text, expected);
        assert_eq!(Answer::Say(text).code(), Some(ExitCode::SUCCESS));
    }

    #[test]
    fn help_reads_naturally_in_turkish() {
        let Answer::Say(text) = answer("tr", &["-h"]) else { panic!("help is said") };
        for line in [
            "Kullanım",
            "  quvyta install AD...  bu üyeleri kurmak için sırayla onay ister",
            "  quvyta show AD        bu üyenin sayfasını açar",
            "paketi (quvyta-code)",
        ] {
            assert!(text.contains(line), "`{line}` is missing:\n{text}");
        }
    }

    #[test]
    fn the_version_is_the_package_s() {
        let expected = Answer::Say(format!("quvyta {}\n", env!("CARGO_PKG_VERSION")));
        assert_eq!(answer("en", &["--version"]), expected);
        assert_eq!(answer("tr", &["-V"]), expected, "the same in every language");
    }

    #[test]
    fn mistakes_are_one_line_and_a_hint_with_exit_code_2() {
        let hint = "Run quvyta --help to see what it takes.";
        let cases = [
            (&["--frobnicate"][..], "quvyta: there is no --frobnicate option"),
            (&["install", "qcod"], "quvyta: there is no Quvyta app called qcod"),
            (&["open"], "quvyta: open is not something quvyta does"),
            (&["install"], "quvyta: install needs at least one name, such as qcode"),
        ];
        for (args, line) in cases {
            let refused = answer("en", args);
            assert_eq!(refused, Answer::Refuse(format!("{line}\n{hint}\n")), "{args:?}");
            assert_eq!(refused.code(), Some(ExitCode::from(2)));
        }
        let Answer::Refuse(text) = answer("tr", &["install", "qcod"]) else { panic!("refused") };
        assert_eq!(
            text,
            "quvyta: qcod adında bir Quvyta uygulaması yok\nNeler yazılabileceğini görmek için: quvyta --help\n"
        );
    }

    #[test]
    fn show_opens_on_one_member_by_any_of_its_names() {
        let show = |key: &str| Parsed::Open(Start::Show(index(key)));
        let cases: [(&[&str], Parsed); 10] = [
            (&["show", "qfocus"], show("focus")),
            (&["show", "focus"], show("focus")),
            (&["show", "Quvyta-Focus"], show("focus")),
            (&["show", "qdesk"], show("desk")),
            (&["show", "quvyta"], show("quvyta")),
            (&["show", "qframe", "--help"], Parsed::Help),
            (&["show"], wrong(Mistake::NoShowName)),
            (&["show", "qcode", "qfocus"], wrong(Mistake::ShowMany)),
            (&["show", "qcode", "qcode"], wrong(Mistake::ShowMany)),
            (&["show", "qcod"], wrong(Mistake::Name("qcod".to_owned()))),
        ];
        for (args, expected) in cases {
            assert_eq!(parsed(args), expected, "{args:?}");
        }
    }

    #[test]
    fn show_s_mistakes_are_told_in_the_screen_s_language() {
        let hint = "Run quvyta --help to see what it takes.";
        for (args, line) in [
            (&["show"][..], "quvyta: show needs a name, such as qfocus"),
            (&["show", "qcode", "qfocus"], "quvyta: show takes only one name"),
            (&["show", "qcod"], "quvyta: there is no Quvyta app called qcod"),
        ] {
            let refused = answer("en", args);
            assert_eq!(refused, Answer::Refuse(format!("{line}\n{hint}\n")), "{args:?}");
            assert_eq!(refused.code(), Some(ExitCode::from(2)));
        }
        let Answer::Refuse(text) = answer("tr", &["show", "a", "b"]) else { panic!("refused") };
        assert_eq!(text, "quvyta: show yalnızca bir ad alır\nNeler yazılabileceğini görmek için: quvyta --help\n");
        let Answer::Refuse(text) = answer("tr", &["show"]) else { panic!("refused") };
        assert!(text.starts_with("quvyta: show bir ad bekliyor, örneğin qfocus\n"), "{text}");
        let _soon = crate::ecosystem::tests::unreleased("desk");
        assert_eq!(answer("en", &["show", "qdesk"]).code(), None, "a member still to come has a page too");
    }

    #[test]
    fn a_member_not_released_yet_is_refused_in_one_line() {
        let desk = index("desk");
        assert_eq!(parsed(&["install", "qdesk"]), Parsed::Open(Start::Install(vec![desk])), "qdesk is out");
        let _soon = crate::ecosystem::tests::unreleased("desk");
        for name in ["qdesk", "desk", "QUVYTA-DESKTOP"] {
            assert_eq!(parsed(&["install", name]), wrong(Mistake::Unreleased(desk)), "{name}");
        }
        assert_eq!(parsed(&["install", "qcode", "qdesk"]), wrong(Mistake::Unreleased(desk)), "nothing starts");
        let refused = answer("en", &["install", "desk"]);
        assert_eq!(
            refused,
            Answer::Refuse("quvyta: qdesk is not released yet, so it cannot be installed\n".to_owned())
        );
        assert_eq!(refused.code(), Some(ExitCode::from(2)));
        let turkish = answer("tr", &["install", "qdesk"]);
        assert_eq!(turkish, Answer::Refuse("quvyta: qdesk henüz yayımlanmadı, bu yüzden kurulamaz\n".to_owned()));
    }

    #[test]
    fn opening_the_screen_prints_nothing() {
        assert_eq!(answer("en", &[]), Answer::Start(Start::List));
        assert_eq!(answer("en", &["install", "qtools"]).code(), None);
    }

    #[test]
    fn an_unknown_language_falls_back_to_english() {
        assert_eq!(answer("xx", &["open"]), answer("en", &["open"]));
    }
}
