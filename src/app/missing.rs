//! The apps this machine does not have: the quiet count under the list, the button that asks for
//! them one at a time, and the key that does what the button does.
//!
//! Nothing is installed without the question it gets today. The button and the key fill the same
//! `asked` queue that `quvyta install NAME…` and the first-run wizard fill, and `ask_next` opens
//! those questions one after another: agreeing to one says nothing about the next.
//!
//! The count asks the question [`Quvyta::installable`] asks, so the line under the list and the
//! buttons of the list itself can never disagree about who can be installed from here.

use qframe::prelude::*;

use super::updates::GAP;
use super::{Msg, Quvyta};
use crate::ecosystem::APPS;

/// The fewest rows the screen has for the line to be drawn. Below it, the line and the update
/// line under it would take every row the list has, so the line gives its rows back to the list:
/// each app's own row still offers its install, and the key still asks for all of them.
const LEAST_ROWS: u16 = 12;

impl Quvyta {
    /// The apps this machine does not have and can have, in the order of the list.
    ///
    /// An app installed some other way is not missing, an app not out yet is not missing, quvyta
    /// itself is not missing, and one that cannot run on this machine is not one this list can
    /// offer. That is one question, [`Quvyta::installable`], the same the rows' own Install
    /// buttons and the first-run wizard ask.
    pub(super) fn missing(&self) -> impl Iterator<Item = usize> + '_ {
        (0..APPS.len()).filter(|index| self.installable(*index))
    }

    /// Whether any app is missing and can be installed here, which is what the line under the
    /// list says and what the key hint waits for.
    pub(super) fn anything_missing(&self) -> bool {
        self.missing().next().is_some()
    }

    /// Asks for the apps that are missing, one after another, the way the first-run wizard leaves
    /// the ones that were checked there. `None` when there is nothing to install, and while a
    /// question of its own is open: that one is the person's business until it is answered or
    /// closed.
    pub(super) fn ask_missing(&self) -> Option<Msg> {
        (self.installs.dialog.is_none() && self.anything_missing()).then_some(Msg::InstallMissing)
    }

    /// Under the list, above the update count: how many apps are missing and can be installed
    /// here, with the button that asks for them one by one. With every one of them installed
    /// there is nothing to say and nothing is drawn: the list keeps the room it had. On a screen
    /// too short for the list and both lines, the list keeps it too.
    pub(super) fn missing_bar(&self, ui: &mut View<'_, Msg>) {
        let count = self.missing().count();
        if count == 0 || self.size.height < LEAST_ROWS {
            return;
        }
        let label = t!("missing.count", n = count);
        // Side by side when the two fit the list column, the button on its own line under the
        // count when they do not, as the update line below draws the same pair. No empty line
        // between them: they are one thing said in two parts.
        ui.row(|ui| {
            ui.add(Text::new(label).role("faint"));
            let install = t!("missing.install");
            ui.add(Button::new(install).variant("primary").on_press(Msg::InstallMissing)).id("install-missing");
        })
        .gap(GAP)
        .wrap(true)
        .fill_width();
    }
}
