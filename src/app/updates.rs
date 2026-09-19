//! Updates as the screen keeps them: the newest versions crates.io named, which members they
//! make out of date, and the line under the list that counts them.
//!
//! Being up to date says nothing: no "everything is up to date" line, silence is the answer.
//! A check that fails says so faintly and the last answer, when there is one, still stands.

use qframe::prelude::*;

use super::installs::{Action, InstallMsg};
use super::{Msg, Quvyta};
use crate::family::FAMILY;
use crate::updates::{self, Check, Latest};

/// Everything that happens to updates.
#[derive(Debug, Clone)]
pub enum UpdateMsg {
    /// Asks crates.io again now, however young the kept answer is.
    Check,
    /// What a check found.
    Checked(Check),
    /// Queues every member that has an update, in the order of the list.
    InstallAll,
}

/// What the screen knows about updates.
#[derive(Debug, Default)]
pub(super) struct Updates {
    /// The newest versions, once known.
    latest: Option<Latest>,
    /// Whether the last check failed.
    failed: bool,
    /// Whether a check someone asked for runs.
    checking: bool,
}

impl Quvyta {
    /// Looks for newer versions in the background; `again` asks crates.io even when the kept
    /// answer is fresh.
    pub(super) fn check_updates(&self, again: bool) -> Command<Msg> {
        let machine = self.machine.clone();
        Command::perform(move || Msg::Updates(UpdateMsg::Checked(updates::check(&machine, again, updates::now()))))
    }

    /// The newest version of the member at `index` on crates.io, when known: what an install
    /// pins, so the version the dialog shows is the one installed. A member not released yet has
    /// none, whatever crates.io says: a crate under its name is not the member.
    pub(super) fn latest_version(&self, index: usize) -> Option<String> {
        let member = FAMILY.get(index).filter(|member| member.published())?;
        Some(self.updates.latest.as_ref()?.version(member.package)?.to_owned())
    }

    /// The newer version the member at `index` can be updated to: one cargo installed, with a
    /// newer version on crates.io. Members installed some other way are left to what installed
    /// them.
    pub(super) fn update_to(&self, index: usize) -> Option<String> {
        let installed = self.state(index)?.cargo_version()?;
        self.latest_version(index).filter(|latest| updates::newer(latest, installed))
    }

    /// Whether the member at `index` has an update that is not on its way yet.
    pub(super) fn updatable(&self, index: usize) -> bool {
        self.update_to(index).is_some() && !self.installs.has(index)
    }

    /// The members with an update not on its way yet, in the order of the list.
    fn outdated(&self) -> impl Iterator<Item = usize> + '_ {
        (0..FAMILY.len()).filter(|index| self.updatable(*index))
    }

    pub(super) fn update_update(&mut self, msg: UpdateMsg) -> Command<Msg> {
        match msg {
            UpdateMsg::Check => {
                self.updates.checking = true;
                return self.check_updates(true);
            }
            UpdateMsg::Checked(check) => {
                self.updates.checking = false;
                match check {
                    Check::Known(latest) => {
                        self.updates.latest = Some(latest);
                        self.updates.failed = false;
                    }
                    Check::Failed(kept) => {
                        self.updates.failed = true;
                        if self.updates.latest.is_none() {
                            self.updates.latest = kept;
                        }
                    }
                }
            }
            UpdateMsg::InstallAll => {
                let all: Vec<(usize, Action)> =
                    self.outdated().map(|index| (index, Action::Install(self.update_to(index)))).collect();
                if all.is_empty() {
                    return Command::none();
                }
                self.installs.queue.extend(all);
                return self.start_next();
            }
        }
        Command::none()
    }

    /// Under the list: how many updates there are with the button that installs them all, or
    /// that the check failed. A click on the count asks again.
    pub(super) fn update_bar(&self, ui: &mut View<'_, Msg>) {
        let count = self.outdated().count();
        if count > 0 {
            let label = t!("updates.count", n = count);
            let install = t!("updates.install-all");
            // Side by side when both fit the column, one above the other when not.
            let width = qframe::text::width(&label) + qframe::text::width(&install) + 2 * BUTTON_PADDING + GAP;
            let buttons = |ui: &mut View<'_, Msg>| {
                ui.add(Button::new(label.clone()).on_press(Msg::Updates(UpdateMsg::Check))).id("update-count");
                let all = Msg::Updates(UpdateMsg::InstallAll);
                ui.add(Button::new(install.clone()).variant("primary").on_press(all)).id("install-updates");
            };
            if width <= self.list_area_width() {
                ui.row(buttons).gap(GAP);
            } else {
                ui.column(buttons).gap(1);
            }
        }
        let note = if self.updates.checking {
            t!("updates.checking")
        } else if self.updates.failed {
            t!("updates.failed")
        } else {
            return;
        };
        // In line with the rows' icons, as the buttons' labels are.
        let indent = Padding { top: 0, right: 0, bottom: 0, left: 2 };
        ui.add(Text::new(note).role("faint")).padding(indent).fill_width();
    }

    /// Asks for an update of the member at `index`, when it has one.
    pub(super) fn ask_update(&self, index: usize) -> Option<Msg> {
        self.updatable(index).then_some(Msg::Install(InstallMsg::Ask(index)))
    }
}

/// The columns a button adds to its label.
const BUTTON_PADDING: u16 = 4;
/// The columns between the count and the button beside it.
const GAP: u16 = 2;
