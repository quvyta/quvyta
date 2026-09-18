//! The members of the Quvyta family this program introduces.

/// How settled a released member is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Released, with an interface that is expected to stay.
    Released,
    /// Released as a beta: usable, but its interface and files may still change.
    Beta,
}

/// A library or application of the family. Names and descriptions live in the language files
/// under `family.<key>`.
#[derive(Debug)]
pub struct Member {
    /// The segment of its language keys.
    pub key: &'static str,
    /// The package name on crates.io.
    pub package: &'static str,
    /// The short terminal command, for applications.
    pub command: Option<&'static str>,
    /// Where the source lives.
    pub repository: &'static str,
    /// What a user runs to start using it: `cargo add` for the library, `cargo install` for an
    /// application.
    pub install: &'static str,
    /// How settled it is.
    pub status: Status,
}

/// Every member, in the order they are shown.
pub const FAMILY: [Member; 5] = [
    Member {
        key: "framework",
        package: "quvyta-framework",
        command: None,
        repository: "https://github.com/quvyta/framework",
        install: "cargo add quvyta-framework",
        status: Status::Released,
    },
    Member {
        key: "code",
        package: "quvyta-code",
        command: Some("qcode"),
        repository: "https://github.com/quvyta/code",
        install: "cargo install quvyta-code",
        status: Status::Beta,
    },
    Member {
        key: "focus",
        package: "quvyta-focus",
        command: Some("qfocus"),
        repository: "https://github.com/quvyta/focus",
        install: "cargo install quvyta-focus",
        status: Status::Beta,
    },
    Member {
        key: "tools",
        package: "quvyta-tools",
        command: Some("qtools"),
        repository: "https://github.com/quvyta/tools",
        install: "cargo install quvyta-tools",
        status: Status::Beta,
    },
    Member {
        key: "packages",
        package: "quvyta-packages",
        command: Some("qpackages"),
        repository: "https://github.com/quvyta/packages",
        install: "cargo install quvyta-packages",
        status: Status::Beta,
    },
];
