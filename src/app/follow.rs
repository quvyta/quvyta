//! Whether each installed member follows the shared language, theme, icons and reduced motion, as
//! the Settings tab shows it.
//!
//! Each member keeps its own settings file in the shared folder. A shared key it does not name,
//! or names as the shared id, follows the shared value; any other value is the member's own. quvyta
//! only reads these files here: it never writes to a member's file it could not read, since
//! rewriting a broken file could lose what someone wrote in it by hand.

use std::path::Path;

use qframe::diagnostics::Severity;
use qframe::icons::IconMode;
use qframe::storage::{Family, Settings, Shared};

use crate::ecosystem::APPS;
use crate::machine::Machine;

/// How one member's settings file stands with the shared settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Following {
    /// The member has no settings file yet: it has never been opened, and when it is, it starts
    /// on the shared values.
    NotOpened,
    /// The file cannot be read as settings; the reasons, each at its file, line and column.
    Unreadable(Vec<String>),
    /// The member's own value of each shared setting the follow table shows, in its order; `None` where
    /// it follows the shared value.
    Keys(Vec<(Shared, Option<String>)>),
}

/// The shared settings the follow table shows, in its order, each with the locale key of its
/// name, as the appearance rows name it, and of the choice that puts it back on the shared value.
/// Reduced motion's row is named by what it does, "Reduce motion", which is too long for a column
/// of a table that must fit four of them in every language; its column says what it is about.
///
/// A list of its own rather than every shared setting the framework knows: the framework may add
/// one, and the table then keeps showing what it knows how to name and to put back, instead of a
/// column it would have no words for.
pub(crate) const SHOWN: [(Shared, &str, &str); 4] = [
    (Shared::Language, "quvyta.appearance.language", "settings.follow-language"),
    (Shared::Theme, "quvyta.appearance.theme", "settings.follow-theme"),
    (Shared::Icons, "quvyta.appearance.icons", "settings.follow-icons"),
    (Shared::ReducedMotion, "settings.follow-motion", "settings.follow-reduced-motion"),
];

/// A Quvyta app, an index of [`APPS`], and how it follows the shared values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberFollowing {
    /// The member, an index of [`APPS`].
    pub index: usize,
    /// How its file stands.
    pub following: Following,
}

/// Reads the settings file of each member at `members`, indexes of [`APPS`], in that order.
pub(crate) fn read(machine: &Machine, members: &[usize]) -> Vec<MemberFollowing> {
    members
        .iter()
        .filter_map(|index| {
            let member = APPS.get(*index)?;
            let following = match machine.member_conf(member.settings_id()) {
                Some(path) => read_file(&path),
                // Without a settings folder no member could have written a file either.
                None => Following::NotOpened,
            };
            Some(MemberFollowing { index: *index, following })
        })
        .collect()
}

/// Puts the member at `index` of [`APPS`] back on the shared value of `key`: only that key
/// of its own file changes, to the shared id, and the shared file is neither read nor
/// written. The framework refuses a file it cannot read and leaves it exactly as it was.
///
/// # Errors
///
/// The framework's: a broken file, no settings folder, or a file that cannot be written.
pub(crate) fn follow(machine: &Machine, index: usize, key: Shared) -> std::io::Result<()> {
    let member = APPS.get(index).ok_or_else(|| std::io::Error::other("no such member"))?;
    match &machine.member_settings {
        Some(folder) => Family::QUVYTA.follow_in(folder, member.settings_id(), key),
        None => Family::QUVYTA.follow(member.settings_id(), key),
    }
}

/// How the settings file at `path` stands with the shared values.
fn read_file(path: &Path) -> Following {
    if !path.exists() {
        return Following::NotOpened;
    }
    // Opening reads and never writes: nothing heals the file without a schema of its own.
    let settings = Settings::open(path).member_of(&Family::QUVYTA);
    let errors: Vec<String> = settings
        .diagnostics()
        .iter()
        .filter(|problem| problem.severity == Severity::Error)
        .map(ToString::to_string)
        .collect();
    if !errors.is_empty() {
        return Following::Unreadable(errors);
    }
    Following::Keys(SHOWN.iter().map(|(key, ..)| (*key, own_value(&settings, *key))).collect())
}

/// The member's own value of `key`, or `None` when it follows the shared value. A value the framework
/// would not use, such as an icon set it does not know, falls back to the shared one the way the
/// framework resolves it, so the table says what the member will really draw with.
fn own_value(settings: &Settings, key: Shared) -> Option<String> {
    // Reduced motion of its own is written as a boolean; following, as the shared id in text.
    if key == Shared::ReducedMotion
        && let Some(flag) = settings.get::<bool>(key.key())
    {
        return Some(flag.to_string());
    }
    let text = settings.get::<String>(key.key())?;
    let text = text.trim();
    // Compared, not matched, so a shared setting the framework adds later needs nothing here.
    let usable = if key == Shared::Icons {
        IconMode::from_name(text).is_some()
    } else if key == Shared::ReducedMotion {
        text.parse::<bool>().is_ok()
    } else {
        !text.is_empty()
    };
    (usable && text != Family::QUVYTA.id()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(text: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let folder = tempfile::tempdir().expect("temp");
        let path = folder.path().join("code.conf");
        std::fs::write(&path, text).expect("written");
        (folder, path)
    }

    #[test]
    fn a_key_named_or_left_out_follows_the_shared_value_and_any_other_value_is_the_member_s_own() {
        let (_folder, path) = file("language = \"quvyta\"\ntheme = \"amber\"\nreduced_motion = true\n");
        let expected = vec![
            (Shared::Language, None),
            (Shared::Theme, Some("amber".to_owned())),
            (Shared::Icons, None),
            (Shared::ReducedMotion, None),
        ];
        assert_eq!(read_file(&path), Following::Keys(expected));
    }

    #[test]
    fn reduced_motion_of_its_own_is_a_boolean_and_following_it_is_the_shared_id() {
        let motion = |text: &str| {
            let (_folder, path) = file(text);
            let Following::Keys(values) = read_file(&path) else { panic!("unreadable: {text}") };
            values.into_iter().find(|(key, _)| *key == Shared::ReducedMotion).expect("shown").1
        };
        assert_eq!(motion("reduced-motion = true\n"), Some("true".to_owned()));
        assert_eq!(motion("reduced-motion = false\n"), Some("false".to_owned()), "off is a value of its own too");
        assert_eq!(motion("reduced-motion = \"quvyta\"\n"), None);
        assert_eq!(motion("theme = \"amber\"\n"), None, "left out, it follows");
    }

    #[test]
    fn an_icon_set_the_framework_does_not_know_follows_the_shared_value_as_the_framework_reads_it() {
        let (_folder, path) = file("icons = \"sparkly\"\n");
        let expected =
            vec![(Shared::Language, None), (Shared::Theme, None), (Shared::Icons, None), (Shared::ReducedMotion, None)];
        assert_eq!(read_file(&path), Following::Keys(expected));
    }

    #[test]
    fn a_missing_file_has_not_been_opened_and_a_broken_one_is_unreadable_and_left_alone() {
        let folder = tempfile::tempdir().expect("temp");
        assert_eq!(read_file(&folder.path().join("code.conf")), Following::NotOpened);
        let broken = "theme = \"amber\nlanguage = \n";
        let (folder, path) = file(broken);
        let Following::Unreadable(reasons) = read_file(&path) else { panic!("readable: {:?}", read_file(&path)) };
        assert!(reasons.iter().all(|reason| reason.starts_with("code.conf:")), "{reasons:?}");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), broken, "reading writes nothing");
        let names: Vec<_> =
            std::fs::read_dir(folder.path()).expect("list").map(|entry| entry.expect("entry").file_name()).collect();
        assert_eq!(names, ["code.conf"], "no backup is left beside it");
    }
}
