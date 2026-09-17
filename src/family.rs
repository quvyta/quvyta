//! The members of the Quvyta family this program introduces.

/// Whether a member can be used today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Released and usable.
    Ready,
    /// Being built; not released yet.
    Coming,
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
    /// What a user runs to start using it, once it is released.
    pub install: Option<&'static str>,
    /// Whether it is released.
    pub status: Status,
}

/// Every member, in the order they are shown.
pub const FAMILY: [Member; 5] = [
    Member {
        key: "framework",
        package: "quvyta-framework",
        command: None,
        repository: "github.com/quvyta/framework",
        install: Some("cargo add quvyta-framework"),
        status: Status::Ready,
    },
    Member {
        key: "code",
        package: "quvyta-code",
        command: Some("qcode"),
        repository: "github.com/quvyta/code",
        install: None,
        status: Status::Coming,
    },
    Member {
        key: "focus",
        package: "quvyta-focus",
        command: Some("qfocus"),
        repository: "github.com/quvyta/focus",
        install: None,
        status: Status::Coming,
    },
    Member {
        key: "tools",
        package: "quvyta-tools",
        command: Some("qtools"),
        repository: "github.com/quvyta/tools",
        install: None,
        status: Status::Coming,
    },
    Member {
        key: "packages",
        package: "quvyta-packages",
        command: Some("qpackages"),
        repository: "github.com/quvyta/packages",
        install: None,
        status: Status::Coming,
    },
];
