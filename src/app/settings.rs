//! The Settings tab: quvyta's own settings from `launcher.conf`, changed in place.
//!
//! There is no Save button. A change applies to the running quvyta at once and is written in the
//! background; when the file cannot be written the value goes back and a notice says where and
//! why, so the screen never shows a setting the next start would not have.

use qframe::prelude::*;
use qframe::widget::NodeMut;
use qframe::widgets::{ScrollView, Select, SettingRow, SettingsList, Switch, Toast};

use super::path_notice::PathReach;
use super::{Msg, Quvyta};
use crate::launcher::{AfterClose, Launcher};

/// The widest the settings grow: beyond it the label and its control drift too far apart to be
/// read as one line.
const SECTION: u16 = 76;
/// From this many columns up the file's path stands to the right of the section title; below it
/// goes under the title.
const WIDE: u16 = 100;
/// The order of the choices after a member closes, as the select lists them.
const AFTER_CLOSE: [AfterClose; 2] = [AfterClose::Return, AfterClose::Shell];

/// A setting and its new value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Whether crates.io is asked for newer versions at start.
    CheckUpdates(bool),
    /// What happens when a member opened from quvyta closes.
    AfterClose(AfterClose),
}

/// Everything that happens on the Settings tab.
#[derive(Debug, Clone)]
pub enum SettingMsg {
    /// Applies the change and writes it to `launcher.conf`.
    Change(Change),
    /// The change was written, or why not; `undo` puts the value back as it was before.
    Saved {
        /// The change that restores the earlier value.
        undo: Change,
        /// Nothing, or the reason the file could not be written.
        result: Result<(), String>,
    },
    /// Shows the PATH notice, the same one quvyta offers on its own.
    AddToPath,
}

impl Quvyta {
    pub(super) fn update_setting(&mut self, msg: SettingMsg) -> Command<Msg> {
        match msg {
            SettingMsg::Change(change) => {
                let undo = self.apply(change);
                if undo == change {
                    return Command::none();
                }
                // Without a settings folder there is nowhere to keep it: it lasts until quvyta
                // quits, as the defaults do.
                let Some(path) = self.machine.launcher_conf.clone() else { return Command::none() };
                Command::perform(move || {
                    let written = match change {
                        Change::CheckUpdates(on) => Launcher::save_check_updates(&path, on),
                        Change::AfterClose(after_close) => Launcher::save_after_close(&path, after_close),
                    };
                    Msg::Setting(SettingMsg::Saved { undo, result: written.map_err(|error| error.to_string()) })
                })
            }
            SettingMsg::Saved { result: Ok(()), .. } => Command::none(),
            SettingMsg::Saved { undo, result: Err(reason) } => {
                self.apply(undo);
                // The folder is what could not be written; the reason alone does not say where.
                let folder = self.machine.launcher_conf.as_deref().and_then(std::path::Path::parent);
                let body = folder.map_or(reason.clone(), |folder| format!("{}\n{reason}", self.machine.show(folder)));
                Command::toast(Toast::warning(t!("settings.not-saved")).body(body))
            }
            SettingMsg::AddToPath => self.ask_path(),
        }
    }

    /// Makes `change` the running setting and returns the change that would restore the one
    /// before. Turning updates off changes what the next start does, as the file does; `r` still
    /// asks when someone wants to know now.
    fn apply(&mut self, change: Change) -> Change {
        match change {
            Change::CheckUpdates(on) => Change::CheckUpdates(std::mem::replace(&mut self.launcher.check_updates, on)),
            Change::AfterClose(after_close) => {
                Change::AfterClose(std::mem::replace(&mut self.launcher.after_close, after_close))
            }
        }
    }

    /// The Settings tab.
    pub(super) fn settings_page(&self, ui: &mut View<'_, Msg>) {
        let width = self.size.width.min(SECTION);
        let page = |ui: &mut View<'_, Msg>| {
            ui.column(|ui| {
                self.section_title(width, ui);
                self.own_settings(ui).width(Length::Cells(width)).id("settings");
                self.show_path_notice_within(width, ui);
            })
            .gap(1)
            .padding(Padding { top: 1, right: 0, bottom: 1, left: 0 })
            .fill_width();
        };
        // The settings alone fit any screen quvyta draws on; only the PATH notice under them can
        // outgrow a short one. A scroll view takes a stop of its own in the tab order, so it is
        // there only when it may be needed.
        if self.path_notice.is_some() {
            ui.add_with(ScrollView::new(), page).fill();
        } else {
            ui.column(page).fill();
        }
    }

    /// The faint title of quvyta's own section and, fainter, the file it is kept in: beside the
    /// title on a wide screen, under it on a narrow one, shortened in the middle when it does
    /// not fit, since the file name at the end matters as much as the folder at the start.
    fn section_title(&self, width: u16, ui: &mut View<'_, Msg>) {
        let words = t!("settings.title");
        let title = Text::new(words.clone()).role("secondary").no_wrap();
        let Some(path) = self.machine.launcher_conf.as_deref().map(|path| self.machine.show(path)) else {
            ui.add(title).padding(Padding::symmetric(0, 2));
            return;
        };
        let room = width.saturating_sub(4);
        if self.size.width >= WIDE {
            // Beside the title the path keeps two cells from it.
            let beside = room.saturating_sub(qframe::text::width(&words) + 2);
            ui.row(|ui| {
                ui.add(title).fill_width();
                ui.add(Text::new(shorten_middle(&path, beside)).role("faint").no_wrap());
            })
            .padding(Padding::symmetric(0, 2))
            .width(Length::Cells(width));
        } else {
            ui.column(|ui| {
                ui.add(title);
                ui.add(Text::new(shorten_middle(&path, room)).role("faint").no_wrap());
            })
            .padding(Padding::symmetric(0, 2))
            .width(Length::Cells(width));
        }
    }

    /// The rows of `launcher.conf`, each written as soon as it changes.
    fn own_settings<'v>(&self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let launcher = &self.launcher;
        SettingsList::show(ui, |list| {
            list.row(SettingRow::new(t!("settings.check-updates")), |ui| {
                let toggle = |on| Msg::Setting(SettingMsg::Change(Change::CheckUpdates(on)));
                ui.add(Switch::new(launcher.check_updates).on_toggle(toggle));
            });
            list.row(SettingRow::new(t!("settings.after-close")), |ui| {
                let options = [t!("settings.after-close-return"), t!("settings.after-close-shell")];
                let selected = AFTER_CLOSE.iter().position(|choice| *choice == launcher.after_close);
                let choose = |index: usize| {
                    let after_close = AFTER_CLOSE.get(index).copied().unwrap_or_default();
                    Msg::Setting(SettingMsg::Change(Change::AfterClose(after_close)))
                };
                ui.add(Select::new(options).selected(selected).on_select(choose));
            });
            let folder = self.machine.show(&self.machine.cargo_bin());
            let path = SettingRow::new(t!("settings.path", folder = folder));
            let reach = self.path_reach();
            // Only a folder nothing adds yet has something to do; enter on the row presses Add.
            let path =
                if reach == PathReach::Missing { path.on_activate(Msg::Setting(SettingMsg::AddToPath)) } else { path };
            list.row(path, |ui| match reach {
                PathReach::Here => {
                    ui.add(Text::new(t!("settings.path-yes")).no_wrap());
                }
                PathReach::NewTerminals => {
                    ui.add(Text::new(t!("settings.path-new-terminals")).no_wrap());
                }
                PathReach::Missing => {
                    ui.add(Text::new(t!("settings.path-no")).no_wrap());
                    ui.spacer().width(Length::Cells(2));
                    ui.add(Button::new(t!("settings.path-add")).on_press(Msg::Setting(SettingMsg::AddToPath)));
                }
            });
        })
    }

    /// The PATH notice the Add of the settings opened, under them and as wide as they are.
    fn show_path_notice_within(&self, width: u16, ui: &mut View<'_, Msg>) {
        // Its pillar stands in the rows' pillar column, so the notice reads as the rows' own.
        if self.path_notice.is_some() {
            ui.column(|ui| self.show_path_notice(ui)).width(Length::Cells(width));
        }
    }
}

/// `text` in at most `width` columns: whole when it fits, else its start and its end with `…`
/// between them, the end getting the extra column.
fn shorten_middle(text: &str, width: u16) -> String {
    if qframe::text::width(text) <= width || width == 0 {
        return text.to_owned();
    }
    let room = width - 1;
    let (head_room, tail_room) = (room / 2, room - room / 2);
    let char_width = |c: char| qframe::text::width(c.encode_utf8(&mut [0; 4]));
    let mut head = String::new();
    let mut used = 0;
    for c in text.chars() {
        if used + char_width(c) > head_room {
            break;
        }
        used += char_width(c);
        head.push(c);
    }
    let mut tail = Vec::new();
    let mut used = 0;
    for c in text.chars().rev() {
        if used + char_width(c) > tail_room {
            break;
        }
        used += char_width(c);
        tail.push(c);
    }
    format!("{head}…{}", tail.into_iter().rev().collect::<String>())
}

#[cfg(test)]
pub(super) mod tests;
