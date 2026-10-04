//! Updates as the screen keeps them: the newest versions crates.io named, which apps they
//! make out of date, and the line under the list that counts them.
//!
//! Being up to date says nothing: no "everything is up to date" line, silence is the answer.
//! A check that fails says so faintly and the last answer, when there is one, still stands.

use qframe::prelude::*;

use super::installs::{Also, Dialog, InstallMsg};
use super::{Msg, Quvyta};
use crate::ecosystem::APPS;
use crate::updates::{self, Check, Latest};

/// Everything that happens to updates.
#[derive(Debug, Clone)]
pub enum UpdateMsg {
    /// Asks crates.io again now, however young the kept answer is.
    Check,
    /// What a check found.
    Checked(Check),
    /// Asks, in one question, whether to update every app that has an update, in the order
    /// of the list.
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

    /// The newest version of the app at `index` on crates.io, when known: what an install
    /// pins, so the version the dialog shows is the one installed. An app not released yet has
    /// none, whatever crates.io says: a crate under its name is not the app.
    pub(super) fn latest_version(&self, index: usize) -> Option<String> {
        let app = APPS.get(index).filter(|app| app.published())?;
        Some(self.updates.latest.as_ref()?.version(app.package)?.to_owned())
    }

    /// The newer version the app at `index` can be updated to: one cargo installed, with a
    /// newer version on crates.io. Apps installed some other way are left to what installed
    /// them.
    pub(super) fn update_to(&self, index: usize) -> Option<String> {
        let installed = self.state(index)?.cargo_version()?;
        self.latest_version(index).filter(|latest| updates::newer(latest, installed))
    }

    /// Whether the app at `index` has an update that is not on its way yet.
    pub(super) fn updatable(&self, index: usize) -> bool {
        self.update_to(index).is_some() && !self.installs.has(index)
    }

    /// The apps with an update not on its way yet, in the order of the list.
    fn outdated(&self) -> impl Iterator<Item = usize> + '_ {
        (0..APPS.len()).filter(|index| self.updatable(*index))
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
            // Every update is asked about first, in one question, the way one app's update is:
            // where each comes from, the versions, the exact commands, and the checks before the
            // button can be pressed. Nothing starts before it is agreed to.
            UpdateMsg::InstallAll => {
                let mut all = self.outdated().filter_map(|index| {
                    let from = self.state(index)?.cargo_version()?.to_owned();
                    Some(Also { index, from, to: self.update_to(index)? })
                });
                let Some(first) = all.next() else { return Command::none() };
                let more: Vec<Also> = all.collect();
                for index in std::iter::once(first.index).chain(more.iter().map(|also| also.index)) {
                    self.installs.ended.remove(&index);
                }
                self.installs.dialog = Some(Dialog {
                    index: first.index,
                    version: Some(first.to),
                    from: Some(first.from),
                    problems: None,
                    more,
                });
                return self.check();
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
            // Side by side when both fit the column, one above the other when not: the row wraps,
            // so the two decide it from the buttons the way they are drawn rather than from a
            // second copy of the width a button takes.
            ui.row(|ui| {
                ui.add(Button::new(label.clone()).on_press(Msg::Updates(UpdateMsg::Check))).id("update-count");
                let all = Msg::Updates(UpdateMsg::InstallAll);
                ui.add(Button::new(install.clone()).variant("primary").on_press(all)).id("install-updates");
            })
            .gap(GAP)
            .wrap(true)
            .line_gap(1)
            .fill_width();
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

    /// Asks for an update of the app at `index`, when it has one.
    pub(super) fn ask_update(&self, index: usize) -> Option<Msg> {
        self.updatable(index).then_some(Msg::Install(InstallMsg::Ask(index)))
    }
}

/// The columns between the count and the button beside it.
const GAP: u16 = 2;
