//! The Quvyta apps this program lists, opens and will install.

use qframe::storage::MEMBERS;

/// How settled a member is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Released, with an interface that is expected to stay.
    Released,
    /// Released as a beta: usable, but its interface and files may still change.
    Beta,
    /// Released only as pre-releases so far, such as `0.1.0-alpha.2`: early, and changing. cargo
    /// installs a pre-release only when its version is named, so it is installed at the newest
    /// version crates.io names.
    Alpha,
    /// Not released yet: listed so the list is whole, but there is nothing on crates.io
    /// to install or update it from.
    Soon,
}

/// Why a member cannot be installed from the machine quvyta runs on, though it is on the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotHere {
    /// It runs on Arch Linux only.
    ArchOnly,
    /// It is not built for Windows, so it does not even build there.
    UnixOnly,
}

/// A Quvyta app. Names and descriptions live in the language files under `apps.<key>`.
#[derive(Debug)]
pub struct Member {
    /// The segment of its language keys.
    pub key: &'static str,
    /// The package name on crates.io.
    pub package: &'static str,
    /// The short terminal command.
    pub command: &'static str,
    /// The icon of its list row, from the framework's icon set.
    pub icon: &'static str,
    /// Where the source lives.
    pub repository: &'static str,
    /// The line that adds its library to a project, for the member that shows off a library.
    pub library: Option<&'static str>,
    /// Whether it only runs on Arch Linux, as `install.sh` and `install.ps1` also know it: on
    /// another system it is listed, and told apart, but not offered for installing.
    pub arch_only: bool,
    /// Whether it is built for Linux and macOS only, as `install.ps1` also knows it: on Windows
    /// it is listed, and told apart, but not offered for installing.
    pub unix_only: bool,
    /// How settled it is. quvyta reads it through `Member::status()`, which a test can answer
    /// otherwise.
    pub status: Status,
}

impl Member {
    /// Whether this member is the program that is running.
    pub(crate) fn is_self(&self) -> bool {
        self.package == env!("CARGO_PKG_NAME")
    }

    /// The id its settings file goes under in the shared settings folder, `<id>.conf`, as the
    /// framework's list of Quvyta apps gives it: the id the member itself asks the framework for,
    /// which is not always its key. qdesk's file is `desktop.conf`, the showcase's `showcase.conf`
    /// and quvyta's `launcher.conf`, since `quvyta.conf` is the file every Quvyta app shares. Taken
    /// from the framework rather than kept here, so the two cannot drift apart.
    pub(crate) fn settings_id(&self) -> &'static str {
        MEMBERS.iter().find(|member| member.package == self.package).map_or(self.key, |member| member.settings_id)
    }

    /// How settled it is: its `status` field, except in a test that made it one still to come
    /// with `tests::unreleased`.
    pub(crate) fn status(&self) -> Status {
        #[cfg(test)]
        if tests::is_unreleased(self.key) {
            return Status::Soon;
        }
        self.status
    }

    /// Whether crates.io has it, so quvyta may install and update it.
    pub(crate) fn published(&self) -> bool {
        self.status() != Status::Soon
    }

    /// Whether it builds on the system quvyta runs on.
    pub(crate) fn runs_here(&self) -> bool {
        !self.unix_only || cfg!(unix)
    }

    /// Why this member cannot be installed from a machine that is Arch Linux when `arch` says so,
    /// or `None` when there is nothing in the way. A member not out yet has no reason here: it is
    /// the wizard's own line, and it is about quvyta rather than about the platform. The order is
    /// the wizard's, so one member never says two different things about itself.
    pub(crate) fn not_here(&self, arch: bool) -> Option<NotHere> {
        if !self.published() {
            return None;
        }
        if self.arch_only && !arch {
            return Some(NotHere::ArchOnly);
        }
        (!self.runs_here()).then_some(NotHere::UnixOnly)
    }

    /// Whether quvyta may offer to install this member from a machine that is Arch Linux when
    /// `arch` says so: it is out on crates.io, it is not quvyta itself, and it runs here. The one
    /// rule the list, the details, the first-run wizard and the command line all ask, so a member
    /// cannot be offered in one of them and held back in another.
    pub(crate) fn offered(&self, arch: bool) -> bool {
        self.published() && !self.is_self() && self.not_here(arch).is_none()
    }
}

/// Every member, in the order they are listed: the applications, the ones still to come after
/// the ones that can be installed, then the framework and quvyta itself, which are about the
/// ecosystem rather than apps to work in. The framework library is not a program, so its
/// showcase stands for it and carries its `cargo add` line.
pub const APPS: [Member; 10] = [
    Member {
        key: "code",
        package: "quvyta-code",
        command: "qcode",
        icon: "prompt",
        repository: "https://github.com/quvyta/code",
        library: None,
        arch_only: false,
        unix_only: false,
        status: Status::Beta,
    },
    Member {
        key: "focus",
        package: "quvyta-focus",
        command: "qfocus",
        icon: "dot-outline",
        repository: "https://github.com/quvyta/focus",
        library: None,
        arch_only: false,
        unix_only: false,
        status: Status::Beta,
    },
    Member {
        key: "tools",
        package: "quvyta-tools",
        command: "qtools",
        icon: "settings",
        repository: "https://github.com/quvyta/tools",
        library: None,
        arch_only: true,
        unix_only: false,
        status: Status::Beta,
    },
    Member {
        key: "packages",
        package: "quvyta-packages",
        command: "qpac",
        icon: "inbox",
        repository: "https://github.com/quvyta/packages",
        library: None,
        arch_only: true,
        unix_only: false,
        status: Status::Beta,
    },
    Member {
        key: "explorer",
        package: "quvyta-explorer",
        command: "qexp",
        icon: "category-files",
        repository: "https://github.com/quvyta/explorer",
        library: None,
        arch_only: false,
        unix_only: true,
        status: Status::Beta,
    },
    Member {
        key: "browser",
        package: "quvyta-browser",
        command: "qbrow",
        icon: "category-network",
        repository: "https://github.com/quvyta/browser",
        library: None,
        arch_only: false,
        unix_only: true,
        status: Status::Beta,
    },
    Member {
        key: "cli",
        package: "quvyta-cli",
        command: "qcli",
        icon: "terminal",
        repository: "https://github.com/quvyta/cli",
        library: None,
        arch_only: false,
        unix_only: true,
        status: Status::Alpha,
    },
    Member {
        key: "desk",
        package: "quvyta-desktop",
        command: "qdesk",
        // A filled square in Unicode, a folder in Nerd Font: a screen of windows with a file
        // manager on it.
        icon: "folder",
        repository: "https://github.com/quvyta/desktop",
        library: None,
        arch_only: false,
        unix_only: false,
        status: Status::Beta,
    },
    Member {
        key: "framework",
        package: "quvyta-framework-showcase",
        command: "qframe",
        icon: "project",
        repository: "https://github.com/quvyta/framework",
        library: Some("cargo add quvyta-framework"),
        arch_only: false,
        unix_only: false,
        status: Status::Released,
    },
    Member {
        key: "quvyta",
        package: "quvyta",
        command: "quvyta",
        icon: "dot",
        repository: "https://github.com/quvyta/quvyta",
        library: None,
        arch_only: false,
        unix_only: false,
        status: Status::Beta,
    },
];

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::Cell;

    use super::{APPS, MEMBERS, NotHere, Status};

    thread_local! {
        /// The key of the member the running test treats as not released yet.
        static UNRELEASED: Cell<Option<&'static str>> = const { Cell::new(None) };
    }

    /// While the returned guard lives, the member `key` is one still to come on this thread,
    /// whatever its real status: every member is out today, and what quvyta does with one that is
    /// not has to stay tested for the next one. The screen's work runs on the test's thread in
    /// the harness, so everything the screen asks sees it.
    pub(crate) fn unreleased(key: &'static str) -> Unreleased {
        assert!(APPS.iter().any(|member| member.key == key), "`{key}` is not a member");
        UNRELEASED.with(|cell| cell.set(Some(key)));
        Unreleased
    }

    /// Puts the member back as it really is when dropped.
    pub(crate) struct Unreleased;

    impl Drop for Unreleased {
        fn drop(&mut self) {
            UNRELEASED.with(|cell| cell.set(None));
        }
    }

    pub(super) fn is_unreleased(key: &str) -> bool {
        UNRELEASED.with(|cell| cell.get() == Some(key))
    }

    #[test]
    fn a_member_is_still_to_come_only_while_a_test_says_so() {
        let desk = APPS.iter().find(|member| member.key == "desk").expect("qdesk is listed");
        assert_eq!(desk.status(), Status::Beta);
        {
            let _soon = unreleased("desk");
            assert_eq!(desk.status(), Status::Soon);
            assert!(!desk.published());
            let code = APPS.iter().find(|member| member.key == "code").expect("qcode is listed");
            assert!(code.published(), "only the member named");
        }
        assert!(desk.published(), "the guard puts it back");
    }

    #[test]
    fn every_member_is_one_the_framework_knows_and_keeps_its_settings_where_the_framework_says() {
        for member in &APPS {
            let known = MEMBERS.iter().find(|known| known.package == member.package);
            let known = known.unwrap_or_else(|| panic!("the framework does not list `{}`", member.package));
            assert_eq!(
                (member.command, member.settings_id()),
                (known.command, known.settings_id),
                "{}",
                member.package
            );
        }
        let desk = APPS.iter().find(|member| member.command == "qdesk").expect("qdesk is listed");
        assert_eq!(desk.settings_id(), "desktop", "qdesk keeps its settings in desktop.conf");
    }

    #[test]
    fn a_member_is_offered_only_where_it_runs() {
        let member = |key: &str| APPS.iter().find(|member| member.key == key).expect("a member");
        // The whole table, written out: offered on Arch Linux, offered anywhere else. Nothing here
        // is worked out from the fields, so a member whose `arch_only` or `unix_only` changes is
        // caught in this test rather than on somebody's screen.
        for (key, (on_arch, elsewhere)) in [
            ("code", (true, true)),
            ("focus", (true, true)),
            ("tools", (true, false)),
            ("packages", (true, false)),
            ("explorer", (true, true)),
            ("browser", (true, true)),
            ("cli", (true, true)),
            ("desk", (true, true)),
            ("framework", (true, true)),
            ("quvyta", (false, false)),
        ] {
            assert_eq!(member(key).offered(true), on_arch, "{key} on Arch Linux");
            assert_eq!(member(key).offered(false), elsewhere, "{key} on another system");
        }
        // The two reasons, named as the list and the details name them.
        for arch in [false, true] {
            assert_eq!(member("tools").not_here(arch), (!arch).then_some(NotHere::ArchOnly), "qtools");
            assert_eq!(member("packages").not_here(arch), (!arch).then_some(NotHere::ArchOnly), "qpac");
            assert_eq!(member("explorer").not_here(arch), None, "qexp is built where the tests run");
            assert_eq!(member("code").not_here(arch), None, "qcode runs everywhere");
        }
        // Not out yet is quvyta's own line, not a platform's, so it is never a reason here.
        let _soon = unreleased("desk");
        for arch in [false, true] {
            assert_eq!(member("desk").not_here(arch), None, "qdesk is not out yet, not anywhere");
            assert!(!member("desk").offered(arch), "and there is nothing to install");
        }
    }

    #[test]
    fn the_first_run_wizard_and_the_list_ask_the_same_rule() {
        // The wizard's own rule, as it stood before the list had one of its own: kept here so the
        // two cannot drift apart by a field changing in one place only.
        for arch in [false, true] {
            for member in &APPS {
                let wizard =
                    member.published() && !member.is_self() && (!member.arch_only || arch) && member.runs_here();
                assert_eq!(member.offered(arch), wizard, "{}", member.package);
            }
        }
    }

    #[test]
    fn every_member_has_a_settings_file_of_its_own_and_quvyta_s_is_launcher_conf() {
        let ids: Vec<&str> = APPS.iter().map(super::Member::settings_id).collect();
        for (index, id) in ids.iter().enumerate() {
            assert!(!ids[..index].contains(id), "`{id}` is the settings id of two members");
            assert_ne!(*id, "quvyta", "`quvyta.conf` is the file every Quvyta app shares");
        }
        let quvyta = APPS.iter().find(|member| member.is_self()).expect("quvyta is listed");
        assert_eq!(quvyta.settings_id(), crate::machine::LAUNCHER);
    }
}
