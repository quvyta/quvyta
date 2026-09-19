//! What the screen shows of installs: a row's state in the list, and in the details the main
//! button, the progress of a running install, a place in the queue, or why the last one failed.

use qframe::prelude::*;
use qframe::widgets::{LogLine, LogView, ProgressBar};

use super::confirm_install::problems;
use super::installs::{Action, Ended, InstallMsg, Phase, Running};
use super::{Msg, Quvyta};
use crate::cargo::Failure;
use crate::family::FAMILY;
use crate::install::{self, Outcome};

/// Rows of cargo's output shown under "Details".
const LOG_ROWS: u16 = 12;
/// Lines of a failure shown without opening the details.
const LAST_LINES: usize = 5;

impl Quvyta {
    /// What a list row says about an install of the member at `index`, when one runs, waits or
    /// failed; `None` leaves the row to say how the member is installed.
    pub(super) fn install_row(&self, index: usize) -> Option<String> {
        if let Some(running) = self.installs.running(index) {
            if running.action == Action::Remove {
                return Some(t!("row.removing"));
            }
            return Some(match running.progress.fraction() {
                Some(fraction) => t!("row.installing-share", percent = percent(fraction)),
                None => t!("row.installing"),
            });
        }
        if self.installs.is_queued(index) {
            return Some(t!("row.queued"));
        }
        match self.installs.ended.get(&index) {
            Some(Ended::Failed { action: Action::Remove, .. }) => Some(t!("row.not-removed")),
            Some(Ended::Failed { .. }) => Some(t!("row.failed")),
            _ => None,
        }
    }

    /// Whether the member at `index` shows its failure mark.
    pub(super) fn install_failed(&self, index: usize) -> bool {
        matches!(self.installs.ended.get(&index), Some(Ended::Failed { .. })) && !self.installs.has(index)
    }

    /// The main part of the details of the member at `index`: Open, Install, or the install
    /// running, waiting or failed.
    pub(super) fn main_action(&self, index: usize, ui: &mut View<'_, Msg>) {
        if let Some(running) = self.installs.running(index) {
            if running.action == Action::Remove {
                // Quick enough that a bar would only flash; the title says what happens.
                let command = FAMILY[index].command;
                ui.add(Text::new(t!("remove.removing", command = command)).role("title").no_wrap());
                return;
            }
            return self.progress(running, ui);
        }
        if self.installs.is_queued(index) {
            return self.queued(index, ui);
        }
        match self.installs.ended.get(&index) {
            Some(Ended::Failed { action, outcome, problems: found, log }) => {
                let lines: Vec<&str> = log.iter().map(LogLine::text).collect();
                return self.failure(index, action, outcome, found, &lines, ui);
            }
            Some(Ended::Stopped) if self.installable(index) => {
                ui.add(Text::new(t!("install.stopped")).role("secondary")).fill_width();
            }
            _ => {}
        }
        let update = self.updatable(index).then_some(Msg::Install(InstallMsg::Ask(index)));
        if self.opening(index).is_some() {
            ui.row(|ui| {
                ui.add(Button::new(t!("detail.open")).variant("primary").on_press(Msg::Open(index))).id("open");
                if let Some(update) = update {
                    ui.add(Button::new(t!("detail.update")).on_press(update)).id("update");
                }
                // Apart from the others, so it is never pressed on the way to them.
                if self.removable(index) {
                    ui.spacer();
                    let ask = Msg::Install(InstallMsg::AskRemove(index));
                    ui.add(Button::new(t!("detail.remove")).on_press(ask)).id("remove");
                }
            })
            .gap(2)
            .fill_width();
        } else if let Some(update) = update {
            // quvyta itself: nothing to open, so the update is the main button.
            ui.row(|ui| {
                ui.add(Button::new(t!("detail.update")).variant("primary").on_press(update)).id("update");
            });
        } else if self.installable(index) {
            ui.row(|ui| {
                let ask = Msg::Install(InstallMsg::Ask(index));
                ui.add(Button::new(t!("detail.install")).variant("primary").on_press(ask)).id("install");
            });
            if self.installs.other_window {
                ui.add(Text::new(t!("install.other-window")).role("secondary")).fill_width();
            }
        }
    }

    fn progress(&self, running: &Running, ui: &mut View<'_, Msg>) {
        let command = FAMILY[running.index].command;
        ui.add(Text::new(t!("install.installing", command = command)).role("title").no_wrap());
        let progress = &running.progress;
        ui.column(|ui| {
            // Every phase takes the width of the longest, so the counts beside it never move.
            let width = PHASES.iter().map(|phase| qframe::text::width(&phase_label(*phase))).max().unwrap_or(0);
            let label = phase_label(progress.phase);
            let pad = " ".repeat(usize::from(width.saturating_sub(qframe::text::width(&label))));
            let counts = progress.counted.map(|(done, total)| format!("   {done} / {total}")).unwrap_or_default();
            ui.add(Text::rich([Span::new(format!("{label}{pad}")), Span::new(counts).role("secondary")]).no_wrap())
                .fill_width();
            let bar = match progress.fraction() {
                Some(fraction) => ProgressBar::new(fraction).percent(false),
                None => ProgressBar::indeterminate(),
            };
            ui.add(bar).fill_width().id("progress");
            // The line keeps its place while no crate is named, so nothing below it jumps.
            ui.add(Text::new(progress.krate.clone().unwrap_or_default()).role("faint").no_wrap()).fill_width();
        })
        .fill_width();
        ui.row(|ui| {
            self.details_button(ui);
            ui.spacer();
            ui.add(Button::new(t!("install.stop")).on_press(Msg::Install(InstallMsg::AskStop))).id("stop");
        })
        .gap(2)
        .fill_width();
        self.details(running.index, ui);
    }

    fn queued(&self, index: usize, ui: &mut View<'_, Msg>) {
        let command = FAMILY[index].command;
        ui.add(Text::new(t!("install.queued-title", command = command)).role("title").no_wrap());
        let first = self.installs.queue.front().is_some_and(|(queued, _)| *queued == index);
        let note = match self.installs.running.as_ref() {
            Some(running) => t!("install.queued-after", command = FAMILY[running.index].command),
            None if first && self.installs.other_window => t!("install.waiting-other-window"),
            None => t!("install.queued"),
        };
        ui.add(Text::new(note).role("secondary")).fill_width();
        ui.row(|ui| {
            let dequeue = Msg::Install(InstallMsg::Dequeue(index));
            ui.add(Button::new(t!("install.dequeue")).on_press(dequeue)).id("dequeue");
        });
    }

    fn failure(
        &self,
        index: usize,
        action: &Action,
        outcome: &Outcome,
        found: &[crate::checks::Problem],
        lines: &[&str],
        ui: &mut View<'_, Msg>,
    ) {
        let member = &FAMILY[index];
        let removing = *action == Action::Remove;
        let title = if removing {
            t!("remove.failed", command = member.command)
        } else {
            t!("install.failed", command = member.command)
        };
        ui.add(Text::new(title).role("title").color("danger")).fill_width();
        let reason = match outcome {
            Outcome::Failed(failure) => failure_text(*failure, member.package),
            Outcome::NotRemoved => t!("remove.failed-reason"),
            Outcome::NotStarted(reason) => t!("install.not-started", reason = reason.as_str()),
            Outcome::Installed { .. } | Outcome::Removed | Outcome::Cancelled => String::new(),
        };
        ui.add(Text::new(reason)).fill_width();
        problems(found, false, self.detail_width(), ui);
        // cargo builds before it touches anything, so a failed install changed nothing; a failed
        // removal may have got part of the way, and says only what cargo said.
        if !removing {
            ui.add(Text::new(t!("install.nothing-changed")).role("secondary")).fill_width();
        }

        let last: Vec<&str> =
            lines.iter().rev().filter(|line| !line.trim().is_empty()).take(LAST_LINES).copied().collect();
        if !last.is_empty() {
            ui.column(|ui| {
                ui.add(Text::new(t!("install.last-lines")).role("faint").no_wrap());
                for line in last.iter().rev() {
                    ui.add(Text::new(line.trim_end()).role("secondary").no_wrap()).fill_width();
                }
            })
            .selectable(true)
            .fill_width();
        }
        if let Some(path) = install::log_path(&self.machine, member).filter(|_| !removing) {
            ui.add(Text::new(t!("install.log-file", path = self.machine.show(&path)))).selectable(true).fill_width();
        }
        ui.row(|ui| {
            self.details_button(ui);
            ui.spacer();
            ui.add(Button::new(t!("install.copy-log")).on_press(Msg::Install(InstallMsg::CopyLog(index))))
                .id("copy-log");
            ui.add(Button::new(t!("install.close")).on_press(Msg::Install(InstallMsg::Dismiss(index)))).id("dismiss");
            let retry = Msg::Install(InstallMsg::Retry(index));
            ui.add(Button::new(t!("install.retry")).variant("primary").on_press(retry)).id("retry");
        })
        .gap(2)
        .fill_width();
        self.details(index, ui);
    }

    fn details_button(&self, ui: &mut View<'_, Msg>) {
        let icon = if self.installs.details { "chevron-down" } else { "chevron-right" };
        ui.add(Button::new(t!("install.details")).icon(icon).on_press(Msg::Install(InstallMsg::ToggleDetails)))
            .id("details");
    }

    /// cargo's own output, when the details are open.
    fn details(&self, index: usize, ui: &mut View<'_, Msg>) {
        if !self.installs.details {
            return;
        }
        if let Some(log) = self.log(index) {
            ui.add(LogView::new(log)).height(Length::Cells(LOG_ROWS)).fill_width().id("log");
        }
    }
}

/// The phases in order, for the width of the longest.
const PHASES: [Phase; 4] = [Phase::Starting, Phase::Downloading, Phase::Compiling, Phase::Placing];

fn phase_label(phase: Phase) -> String {
    match phase {
        Phase::Starting => t!("install.phase.starting"),
        Phase::Downloading => t!("install.phase.downloading"),
        Phase::Compiling => t!("install.phase.compiling"),
        Phase::Placing => t!("install.phase.placing"),
    }
}

/// A whole percentage of `fraction`.
fn percent(fraction: f32) -> String {
    format!("{:.0}", (fraction * 100.0).clamp(0.0, 100.0).floor())
}

/// The plain sentence for a failure.
fn failure_text(failure: Failure, package: &str) -> String {
    match failure {
        Failure::NoLinker => t!("install.failure.linker"),
        Failure::OldRust => t!("install.failure.rust"),
        Failure::Network => t!("install.failure.network"),
        Failure::DiskFull => t!("install.failure.disk"),
        Failure::NotFound => t!("install.failure.not-found", package = package),
        Failure::Build => t!("install.failure.build"),
    }
}
