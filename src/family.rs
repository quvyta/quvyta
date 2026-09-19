//! The members of the Quvyta family this program lists, opens and will install.

/// How settled a member is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Released, with an interface that is expected to stay.
    Released,
    /// Released as a beta: usable, but its interface and files may still change.
    Beta,
    /// Not released yet: listed so the family is known whole, but there is nothing on crates.io
    /// to install or update it from.
    Soon,
}

/// A program of the family. Names and descriptions live in the language files under
/// `family.<key>`.
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
    /// How settled it is.
    pub status: Status,
}

impl Member {
    /// Whether this member is the program that is running.
    pub(crate) fn is_self(&self) -> bool {
        self.package == env!("CARGO_PKG_NAME")
    }

    /// Whether crates.io has it, so quvyta may install and update it.
    pub(crate) fn published(&self) -> bool {
        self.status != Status::Soon
    }
}

/// Every member, in the order they are listed: the applications, the ones still to come after
/// the ones that can be installed, then the framework and quvyta itself, which are about the
/// family rather than members to work in. The framework library is not a program, so its
/// showcase stands for it and carries its `cargo add` line.
pub const FAMILY: [Member; 7] = [
    Member {
        key: "code",
        package: "quvyta-code",
        command: "qcode",
        icon: "prompt",
        repository: "https://github.com/quvyta/code",
        library: None,
        status: Status::Beta,
    },
    Member {
        key: "focus",
        package: "quvyta-focus",
        command: "qfocus",
        icon: "dot-outline",
        repository: "https://github.com/quvyta/focus",
        library: None,
        status: Status::Beta,
    },
    Member {
        key: "tools",
        package: "quvyta-tools",
        command: "qtools",
        icon: "settings",
        repository: "https://github.com/quvyta/tools",
        library: None,
        status: Status::Beta,
    },
    Member {
        key: "packages",
        package: "quvyta-packages",
        command: "qpac",
        icon: "inbox",
        repository: "https://github.com/quvyta/packages",
        library: None,
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
        status: Status::Soon,
    },
    Member {
        key: "framework",
        package: "quvyta-framework-showcase",
        command: "qframe",
        icon: "project",
        repository: "https://github.com/quvyta/framework",
        library: Some("cargo add quvyta-framework"),
        status: Status::Released,
    },
    Member {
        key: "quvyta",
        package: "quvyta",
        command: "quvyta",
        icon: "dot",
        repository: "https://github.com/quvyta/quvyta",
        library: None,
        status: Status::Beta,
    },
];
