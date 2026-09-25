//! The Quvyta apps this program lists, opens and will install.

use qframe::storage::MEMBERS;

/// How settled a member is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Released, with an interface that is expected to stay.
    Released,
    /// Released as a beta: usable, but its interface and files may still change.
    Beta,
    /// Not released yet: listed so the list is whole, but there is nothing on crates.io
    /// to install or update it from.
    Soon,
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
}

/// Every member, in the order they are listed: the applications, the ones still to come after
/// the ones that can be installed, then the framework and quvyta itself, which are about the
/// ecosystem rather than apps to work in. The framework library is not a program, so its
/// showcase stands for it and carries its `cargo add` line.
pub const APPS: [Member; 7] = [
    Member {
        key: "code",
        package: "quvyta-code",
        command: "qcode",
        icon: "prompt",
        repository: "https://github.com/quvyta/code",
        library: None,
        arch_only: false,
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
        status: Status::Beta,
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
        status: Status::Beta,
    },
];

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::Cell;

    use super::{APPS, MEMBERS, Status};

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
