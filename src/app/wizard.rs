//! The first start: the framework's setup wizard, with quvyta's own step asking which Quvyta apps
//! to install.
//!
//! It opens while quvyta has no `launcher.conf` of its own, and only then: someone who has used
//! an earlier version keeps their file and never sees it. The framework owns the appearance step,
//! the buttons and the two files; nothing at all is written until Finish, so a quvyta closed
//! half-way leaves the settings folder exactly as it was and the wizard comes again next start.
//!
//! quvyta's step is the list of Quvyta apps as checkboxes, nothing checked. A member that runs on
//! Arch Linux only is faint and cannot be checked on another system, and so is one that is not
//! released yet; the distribution comes from the same place the install checks read it
//! ([`crate::checks::distro`]). On Finish the checked members go through the install dialog and
//! the queue the Apps tab uses, one after another: quvyta installs nothing without being told.

use qframe::prelude::*;
use qframe::widgets::{Checkbox, ScrollView, SetupWizard, Toast};

use super::{Msg, Quvyta};
use crate::ecosystem::{APPS, Member, Status};

/// Rows the wizard takes around its page: the padding, the name, the blank line under it, the
/// steps, the blank lines around the page and the row of buttons.
const AROUND_PAGE: u16 = 8;

/// The fewest rows the page keeps, however short the terminal is.
const LEAST_PAGE_ROWS: u16 = 8;

/// Cells the app list keeps from the page's left edge, so a member's line stands under its
/// checkbox rather than under its name.
const LINE_INDENT: u16 = 4;

/// Something on quvyta's own step of the wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardMsg {
    /// Checks or unchecks the member at this index of [`APPS`].
    Pick(usize, bool),
    /// quvyta's own settings were written into the file the wizard made, or why not.
    Stored(Result<(), String>),
}

impl Quvyta {
    /// Whether the first-run wizard has the screen.
    pub(super) fn setting_up(&self) -> bool {
        self.setup.as_ref().is_some_and(qframe::widgets::Setup::needed)
    }

    /// Whether the member at `index` can be chosen on the wizard's own step: it is released, it
    /// runs on this system, and it is not quvyta itself, which is running already.
    pub(super) fn choosable(&self, index: usize) -> bool {
        APPS.get(index)
            .is_some_and(|member| member.published() && !member.is_self() && (!member.arch_only || self.arch))
    }

    pub(super) fn update_wizard(&mut self, msg: WizardMsg) -> Command<Msg> {
        match msg {
            WizardMsg::Pick(index, on) => {
                if self.choosable(index)
                    && let Some(picked) = self.picked.get_mut(index)
                {
                    *picked = on;
                }
            }
            WizardMsg::Stored(Ok(())) => {}
            WizardMsg::Stored(Err(reason)) => {
                // The appearance is already in its own file; only quvyta's own keys were lost.
                return Command::toast(Toast::warning(t!("settings.not-saved")).body(reason));
            }
        }
        Command::none()
    }

    /// The wizard wrote both files: quvyta's own settings go in beside the shared keys, the
    /// screen takes the appearance that was chosen, and the members that were checked start
    /// their way through the install dialog.
    pub(super) fn finish_setup(&mut self) -> Command<Msg> {
        let Some(setup) = self.setup.take() else { return Command::none() };
        // The rows of the Settings tab carry on from what the wizard chose; its own Appearance
        // wrote nothing but held the values.
        self.appearance = super::appearance_of(&self.machine, setup.preferences().clone());
        self.launcher = crate::launcher::Launcher::default();
        crate::launcher::Launcher::write_defaults(&mut self.settings);
        let saved = self.settings.save_command(|result| Msg::Wizard(WizardMsg::Stored(result)));
        self.asked = (0..APPS.len()).filter(|index| self.picked.get(*index) == Some(&true)).collect();
        // Which members are there already is only known once cargo has answered; when it has
        // not, the first answer opens the dialogs instead.
        let asking = if self.inventory.is_some() { self.ask_next() } else { Command::none() };
        Command::batch([saved, Command::focus("apps"), asking])
    }

    /// The wizard, while it is wanted: the framework's appearance step, then quvyta's own.
    pub(super) fn setup_wizard(&self, ui: &mut View<'_, Msg>) {
        let Some(setup) = &self.setup else { return };
        ui.column(|ui| {
            self.wizard_name(ui);
            ui.spacer().height(Length::Cells(1));
            // Every step is given the rows that are left, so the buttons stand at the bottom
            // wherever the user is and a page longer than the screen scrolls instead of pushing
            // them off it.
            SetupWizard::new(setup)
                .step(t!("wizard.step-apps"), |ui| self.apps_step(ui))
                .page_height(self.size.height.saturating_sub(AROUND_PAGE).max(LEAST_PAGE_ROWS))
                .show(ui)
                .fill_width();
        })
        .padding(Padding::symmetric(1, 2))
        .fill();
    }

    /// The name over the wizard, with the tagline when there is room for it whole: cut short it
    /// would read worse than not being there.
    fn wizard_name(&self, ui: &mut View<'_, Msg>) {
        let tagline = format!("  {}", t!("header.tagline"));
        let mut spans = vec![Span::new("quvyta").color("accent").bold()];
        if qframe::text::width("quvyta") + qframe::text::width(&tagline) + 4 <= self.size.width {
            spans.push(Span::new(tagline).role("faint"));
        }
        ui.add(Text::rich(spans).no_wrap()).fill_width();
    }

    /// quvyta's own step: the Quvyta apps, each with a checkbox and one line saying what it is.
    fn apps_step(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new(t!("wizard.apps-intro")).role("secondary")).fill_width();
        ui.spacer().height(Length::Cells(1));
        let page = |ui: &mut View<'_, Msg>| {
            for (index, member) in APPS.iter().enumerate() {
                // quvyta is the program asking; it is not one of the choices.
                if member.is_self() {
                    continue;
                }
                self.app_row(index, member, ui);
            }
        };
        // Six members with a line each outgrow a short screen; the page scrolls rather than
        // cutting the last of them off.
        ui.add_with(ScrollView::new(), page).fill().id("wizard-apps");
    }

    /// One member: its checkbox and, under it, the one line that says what it is and why it
    /// cannot be chosen when it cannot.
    fn app_row(&self, index: usize, member: &Member, ui: &mut View<'_, Msg>) {
        let choosable = self.choosable(index);
        let checked = self.picked.get(index) == Some(&true);
        let box_ = Checkbox::new(checked)
            .label(member.command)
            .disabled(!choosable)
            .on_toggle(move |on| Msg::Wizard(WizardMsg::Pick(index, on)));
        ui.add(box_).id(format!("wizard-{}", member.key));
        let line = t!(&format!("wizard.line-{}", member.key));
        let line = match (member.status, member.arch_only && !self.arch) {
            (Status::Soon, _) => t!("wizard.soon", line = line),
            (_, true) => t!("wizard.arch-only", line = line),
            _ => line,
        };
        let role = if choosable { "secondary" } else { "faint" };
        ui.add(Text::new(line).role(role)).padding(Padding { left: LINE_INDENT, ..Padding::default() }).fill_width();
    }
}

#[cfg(test)]
#[path = "wizard_tests.rs"]
pub(super) mod tests;
