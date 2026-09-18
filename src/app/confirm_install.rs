//! The dialogs of installing: the one that asks before anything starts, showing where the
//! program comes from and where it goes, the one that asks before stopping, and the one that
//! asks before removing.

use qframe::prelude::*;
use qframe::widgets::{Checkbox, CopyValue, Modal};

use super::installs::{Dialog, InstallMsg};
use super::{Msg, Quvyta};
use crate::checks::{self, Distro, Problem};
use crate::family::FAMILY;
use crate::install::{Job, Removal};

/// The dialogs' width, padding included; narrow screens shrink it.
const WIDTH: u16 = 72;

impl Quvyta {
    /// The dialogs that are open.
    pub(super) fn install_dialogs(&self, ui: &mut View<'_, Msg>) {
        if let Some(dialog) = &self.installs.dialog {
            self.confirm_dialog(dialog, ui);
        }
        if let Some(all) = self.installs.stop {
            self.stop_dialog(all, ui);
        }
        if let Some(index) = self.installs.remove {
            self.remove_dialog(index, ui);
        }
    }

    fn remove_dialog(&self, index: usize, ui: &mut View<'_, Msg>) {
        let member = &FAMILY[index];
        let keep = Msg::Install(InstallMsg::KeepMember);
        let remove = Msg::Install(InstallMsg::ConfirmRemove);
        let modal = Modal::new()
            .title(t!("remove.title", command = member.command))
            .variant("danger")
            .width(WIDTH)
            .on_close(keep.clone())
            .action(Button::new(t!("confirm.cancel")).on_press(keep))
            .action(Button::new(t!("remove.confirm")).variant("danger").on_press(remove));
        ui.add_with(modal, |ui| {
            ui.column(|ui| {
                let program = self.machine.show(&self.machine.cargo_bin().join(member.command));
                ui.add(Text::new(t!("remove.message", program = program))).fill_width();
                // The settings are the user's: named, so it is plain they stay.
                let kept = match &self.machine.settings_dir {
                    Some(folder) => t!("remove.settings-kept", folder = self.machine.show(folder)),
                    None => t!("remove.settings-kept-anywhere"),
                };
                ui.add(Text::new(kept).role("secondary")).fill_width();
                if let Some(removal) = Removal::new(&self.machine, member) {
                    copyable(ui, t!("confirm.command"), removal.command_line(), "remove-command");
                }
            })
            .gap(1)
            .fill_width();
        });
    }

    fn confirm_dialog(&self, dialog: &Dialog, ui: &mut View<'_, Msg>) {
        let member = &FAMILY[dialog.index];
        let job = Job::new(&self.machine, member, dialog.version.clone());
        let found = dialog.problems.as_deref();
        let ready = found.is_some_and(<[Problem]>::is_empty);
        let close = Msg::Install(InstallMsg::Close);
        let (title, go) = match (&dialog.from, &dialog.version) {
            (Some(from), Some(to)) => (
                t!("confirm.update-title", command = member.command, from = from.as_str(), to = to.as_str()),
                t!("confirm.update"),
            ),
            _ => (t!("confirm.title", command = member.command), t!("confirm.install")),
        };
        let mut modal = Modal::new()
            .title(title)
            .width(WIDTH)
            .on_close(close.clone())
            .action(Button::new(t!("confirm.cancel")).on_press(close));
        modal = if found.is_some_and(|found| !found.is_empty()) {
            modal.action(Button::new(t!("checks.again")).on_press(Msg::Install(InstallMsg::Recheck)))
        } else {
            // Disabled while the checks run: they take a moment, and nothing starts unchecked.
            let install = Button::new(go).variant("primary").disabled(!ready);
            modal.action(install.on_press(Msg::Install(InstallMsg::Confirm)))
        };
        ui.add_with(modal, |ui| {
            ui.column(|ui| {
                let version = match &dialog.version {
                    Some(version) => version.clone(),
                    None => t!("confirm.latest"),
                };
                let target = self.machine.show(&self.machine.cargo_bin().join(member.command));
                let fields = [
                    (t!("confirm.source"), t!("confirm.crates-io", package = member.package)),
                    (t!("confirm.version"), version),
                    (t!("confirm.target"), target),
                ];
                // The values start in one column, whatever the language makes of the labels.
                let width = fields.iter().map(|(label, _)| qframe::text::width(label)).max().unwrap_or(0);
                ui.column(|ui| {
                    for (label, value) in &fields {
                        field(ui, label, width, value);
                    }
                })
                .fill_width();
                match found {
                    // What is in the way takes the room of what the install would do.
                    Some(found) if !found.is_empty() => problems(found, true, ui),
                    _ => {
                        if let Some(job) = &job {
                            copyable(ui, t!("confirm.command"), job.command_line(), "command");
                        }
                        ui.column(|ui| {
                            ui.add(Text::new(t!("confirm.built-here")).role("secondary")).fill_width();
                            let home = self.machine.show(&self.machine.cargo_home);
                            ui.add(Text::new(t!("confirm.no-sudo", folder = home)).role("secondary")).fill_width();
                        })
                        .fill_width();
                    }
                }
                if ready && self.installs.other_window {
                    ui.add(Text::new(t!("install.other-window")).role("secondary")).fill_width();
                }
            })
            .gap(1)
            .fill_width();
        });
    }

    fn stop_dialog(&self, all: bool, ui: &mut View<'_, Msg>) {
        let Some(running) = &self.installs.running else { return };
        let command = FAMILY[running.index].command;
        let keep = Msg::Install(InstallMsg::KeepRunning);
        let modal = Modal::new()
            .title(t!("stop.title", command = command))
            .variant("danger")
            .width(WIDTH)
            .on_close(keep.clone())
            .action(Button::new(t!("stop.keep")).on_press(keep))
            .action(Button::new(t!("stop.confirm")).variant("danger").on_press(Msg::Install(InstallMsg::Stop)));
        ui.add_with(modal, |ui| {
            ui.column(|ui| {
                ui.add(Text::new(t!("stop.message")).role("secondary")).fill_width();
                if !self.installs.queue.is_empty() {
                    let toggle = |all| Msg::Install(InstallMsg::StopQueued(all));
                    ui.add(Checkbox::new(all).label(t!("stop.queued")).on_toggle(toggle)).id("stop-queued");
                }
            })
            .gap(1)
            .fill_width();
        });
    }
}

/// What is in the way of an install, each with its fix: copyable commands to run, or a button
/// when quvyta can run the fix itself. Without `headline` only the fixes are drawn, for a
/// failure that has already said what is wrong.
pub(super) fn problems(found: &[Problem], headline: bool, ui: &mut View<'_, Msg>) {
    for problem in found {
        ui.column(|ui| match problem {
            Problem::NoCargo => {
                if headline {
                    ui.add(Text::new(t!("checks.no-cargo")).color("warning")).fill_width();
                }
                copyable(ui, t!("checks.install-script"), checks::INSTALL_SCRIPT.to_owned(), "install-script");
                copyable(ui, t!("checks.rustup-site"), checks::RUSTUP_SITE.to_owned(), "rustup-site");
            }
            Problem::OldRust { version, rustup } => {
                let (major, minor) = checks::MIN_RUST;
                let needed = format!("{major}.{minor}");
                let text = t!("checks.old-rust", version = version.as_str(), needed = needed.as_str());
                if headline {
                    ui.add(Text::new(text).color("warning")).fill_width();
                }
                match rustup {
                    Some(rustup) => {
                        ui.add(Text::new(t!("checks.rustup-update")).role("secondary")).fill_width();
                        ui.row(|ui| {
                            let update = Msg::Install(InstallMsg::UpdateRust(rustup.clone()));
                            ui.add(Button::new(t!("checks.update-rust")).on_press(update)).id("update-rust");
                        });
                    }
                    None => {
                        ui.add(Text::new(t!("checks.update-rust-yourself")).role("secondary")).fill_width();
                    }
                }
            }
            Problem::NoLinker(distro) => {
                if headline {
                    ui.add(Text::new(t!("checks.no-linker")).color("warning")).fill_width();
                }
                let shown: Vec<Distro> = match distro {
                    Distro::Other => Distro::KNOWN.to_vec(),
                    known => vec![*known],
                };
                for (at, distro) in shown.into_iter().enumerate() {
                    if let Some(command) = distro.linker_command() {
                        let label = t!("checks.linker-on", distro = distro.name());
                        copyable(ui, label, command.to_owned(), &format!("linker-{at}"));
                    }
                }
                ui.add(Text::new(t!("checks.no-sudo")).role("secondary")).fill_width();
            }
        })
        .fill_width();
    }
}

/// A quiet label, padded to `width` columns, and its value on one line.
fn field(ui: &mut View<'_, Msg>, label: &str, width: u16, value: &str) {
    let pad = " ".repeat(usize::from(width.saturating_sub(qframe::text::width(label))) + 2);
    ui.add(Text::rich([Span::new(format!("{label}{pad}")).role("faint"), Span::new(value)])).fill_width();
}

/// A label with a copyable value under it, so a narrow screen never cuts the value short.
fn copyable(ui: &mut View<'_, Msg>, label: String, value: String, id: &str) {
    ui.column(|ui| {
        ui.add(Text::new(label).role("faint").no_wrap());
        ui.add(CopyValue::new(value)).id(id);
    })
    .fill_width();
}
