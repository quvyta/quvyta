//! The question that stands between **Open** and an app this machine cannot run yet: what the
//! app needs from the machine, the one line that puts it right, and the ways out of it.
//!
//! It is the install dialog's own problem painter, the install dialog's own **Install here** and
//! the install dialog's own **Check again**; only the answer is different, because an app that
//! is already installed is not installed again — it opens, as soon as what it needs is there.

use qframe::prelude::*;
use qframe::widgets::Modal;

use super::confirm_install::{problem_commands, problems, width_for};
use super::installs::InstallMsg;
use super::{Msg, Quvyta};
use crate::checks::Problem;
use crate::ecosystem::APPS;

/// Everything that can happen to the question.
#[derive(Debug, Clone, Copy)]
pub enum NeedsMsg {
    /// Closes the question without opening the app: what it needs stays missing, and the
    /// app is left alone. Esc and the close mark mean the same thing.
    NotNow,
}

/// The question about the app at an index of [`APPS`], while it is open.
#[derive(Debug, Clone)]
pub(super) struct Needs {
    /// The app the question is about.
    pub(super) index: usize,
    /// What is in the way; `None` while the checks run.
    pub(super) problems: Option<Vec<Problem>>,
}

impl Quvyta {
    /// Asks about what the app at `index` needs from the machine, instead of handing the
    /// terminal to a program that is about to fail. The checks it waits for are the ones the
    /// install dialog runs, asked with this app in mind, and they run in the background.
    pub(super) fn ask_need(&mut self, index: usize) -> Command<Msg> {
        if self.needs.is_some() {
            return Command::none();
        }
        self.needs = Some(Needs { index, problems: None });
        self.checked(index)
    }

    /// What the checks found for the question: an app whose requirement is met opens by itself,
    /// since opening it is what the person asked for a moment ago, and one still short of it
    /// keeps the question with what is missing.
    pub(super) fn needs_checked(&mut self, index: usize, problems: Vec<Problem>) -> Command<Msg> {
        if self.needs.as_ref().is_none_or(|needs| needs.index != index) {
            return Command::none();
        }
        if problems.is_empty() {
            self.needs = None;
            return match self.opening(index) {
                Some(opening) => opening.handoff(index, &APPS[index]),
                None => Command::none(),
            };
        }
        if let Some(needs) = self.needs.as_mut() {
            needs.problems = Some(problems);
        }
        Command::none()
    }

    /// What the question answers.
    pub(super) fn update_needs(&mut self, msg: NeedsMsg) -> Command<Msg> {
        match msg {
            // Not now leaves the app exactly as it was, and quvyta gets the screen back.
            NeedsMsg::NotNow => self.needs = None,
        }
        Command::none()
    }

    /// The question, while it is open.
    pub(super) fn needs_dialog(&self, ui: &mut View<'_, Msg>) {
        let Some(needs) = &self.needs else { return };
        let app = &APPS[needs.index];
        let found = needs.problems.as_deref().unwrap_or_default();
        let width = width_for(found.iter().flat_map(problem_commands));
        let not_now = Msg::Needs(NeedsMsg::NotNow);
        let mut modal = Modal::new()
            .title(t!("needs.title", command = app.command))
            .width(width)
            .on_close(not_now.clone())
            .action(Button::new(t!("needs.not-now")).on_press(not_now));
        // A line run by hand is only found by looking again, as in the install dialog.
        if !found.is_empty() {
            modal = modal.action(Button::new(t!("checks.again")).on_press(Msg::Install(InstallMsg::Recheck)));
        }
        ui.add_with(modal, |ui| {
            let body = |ui: &mut View<'_, Msg>| match &needs.problems {
                // The checks run `rustc --version`, so there is a moment with nothing to say yet.
                None => {
                    ui.add(Text::new(t!("needs.checking", command = app.command)).role("secondary")).fill_width();
                }
                Some(found) => {
                    // The same problem, painted as the install dialog paints it.
                    problems(found, true, ui);
                    ui.add(Text::new(t!("needs.then-open", command = app.command)).role("secondary")).fill_width();
                }
            };
            ui.column(body).gap(1).fill_width();
        });
    }
}
