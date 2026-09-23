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
                ui.add(Text::new(t!("remove.removing", command = command)).role("title"));
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
        ui.add(Text::new(t!("install.installing", command = command)).role("title"));
        let progress = &running.progress;
        ui.column(|ui| {
            // Every phase takes the width of the longest, so the counts beside it never move.
            let width = PHASES.iter().map(|phase| qframe::text::width(&phase_label(*phase))).max().unwrap_or(0);
            let label = phase_label(progress.phase);
            let pad = " ".repeat(usize::from(width.saturating_sub(qframe::text::width(&label))));
            let counts = progress.counted.map(|(done, total)| format!("   {done} / {total}")).unwrap_or_default();
            // The counts climb while the install runs, so a wrapping line would reflow the
            // screen mid-install; it stays on one line, and since the phase comes first in a
            // column as wide as the longest phase, what a narrow screen cuts is the counts.
            ui.add(Text::rich([Span::new(format!("{label}{pad}")), Span::new(counts).role("secondary")]).no_wrap())
                .fill_width();
            let bar = match progress.fraction() {
                Some(fraction) => ProgressBar::new(fraction).percent(false),
                None => ProgressBar::indeterminate(),
            };
            ui.add(bar).fill_width().id("progress");
            // The line keeps its place while no crate is named, so nothing below it jumps, and
            // it names a new crate every few frames: it holds nothing but cargo's own word, so
            // a narrow screen shortens it rather than reflowing everything under it.
            ui.add(Text::new(progress.krate.clone().unwrap_or_default()).role("faint").no_wrap()).fill_width();
        })
        .fill_width();
        actions(ui, |ui| {
            self.details_button(ui);
            ui.spacer();
            ui.add(Button::new(t!("install.stop")).on_press(Msg::Install(InstallMsg::AskStop))).id("stop");
        });
        self.details(running.index, ui);
    }

    fn queued(&self, index: usize, ui: &mut View<'_, Msg>) {
        let command = FAMILY[index].command;
        ui.add(Text::new(t!("install.queued-title", command = command)).role("title"));
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
        actions(ui, |ui| {
            self.details_button(ui);
            ui.spacer();
            ui.add(Button::new(t!("install.copy-log")).on_press(Msg::Install(InstallMsg::CopyLog(index))))
                .id("copy-log");
            ui.add(Button::new(t!("install.close")).on_press(Msg::Install(InstallMsg::Dismiss(index)))).id("dismiss");
            let retry = Msg::Install(InstallMsg::Retry(index));
            ui.add(Button::new(t!("install.retry")).variant("primary").on_press(retry)).id("retry");
        });
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

/// Buttons side by side, the ones that do not fit moving to the next line. A button cannot
/// shorten its words, and a row wider than its room would lose the buttons at its end off the
/// screen, which after a failure would be Close and Try again.
fn actions<M: Clone + 'static>(ui: &mut View<'_, M>, build: impl FnOnce(&mut View<'_, M>)) {
    ui.row(build).gap(2).wrap(true).line_gap(1).fill_width();
}

/// The columns a button takes: its words with the theme's air on both sides.
fn button_width(label: &str) -> u16 {
    qframe::text::width(label) + 4
}

/// The details button carries a chevron before its words: a glyph and a space, and the glyph
/// is one cell in every glyph set, the way a badge's dot is.
fn details_button_width() -> u16 {
    button_width(&t!("install.details")) + 2
}

/// The columns buttons side by side take: each of them, two cells between neighbours, and the
/// spacer that holds a group apart counted as a neighbour of its own.
fn row_width(buttons: &[u16], spacer: bool) -> u16 {
    let neighbours = buttons.len() + usize::from(spacer);
    let gaps = 2 * u16::try_from(neighbours.saturating_sub(1)).unwrap_or(0);
    buttons.iter().sum::<u16>() + gaps
}

/// The columns the row under a running install asks for: the details button, the spacer that
/// holds Stop apart, and Stop. Nothing about it depends on which member is installing, so the
/// split can keep room for it without moving the moment an install starts.
pub(super) fn running_row_width() -> u16 {
    row_width(&[details_button_width(), button_width(&t!("install.stop"))], true)
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
