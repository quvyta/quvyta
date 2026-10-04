//! The Settings tab: the shared appearance and quvyta's own settings from
//! `launcher.conf`, changed in place.
//!
//! The appearance rows come from the framework, the same rows in the same order as in every other
//! Quvyta application, and each of them carries the box that says whether the change holds
//! everywhere or here only. quvyta is where someone installs the Quvyta apps, so this is where
//! their shared language, theme and icons are chosen.
//!
//! There is no Save button. A change applies to the running quvyta at once and is written in the
//! background; when the file cannot be written the value goes back and a notice says where and
//! why, so the screen never shows a setting the next start would not have.

use qframe::icons::nerd_font::{self, Archive, Install, Progress};
use qframe::icons::{GlyphMode, GlyphSample};
use qframe::prelude::*;
use qframe::storage::Shared;
use qframe::widget::NodeMut;
use qframe::widgets::{
    Appearance, AppearanceChange, Column, ColumnWidth, ContextItem, Modal, ProgressBar, ScrollView, Select, SettingRow,
    SettingsList, SettingsRows, Table, TableCell, TableRow, Toast,
};

use super::confirm_install::field;
use super::follow::{self, Following, MemberFollowing};
use super::path_notice::PathReach;
use super::{Msg, Quvyta};
use crate::ecosystem::APPS;
use crate::inventory::State;
use crate::launcher::{AfterClose, Launcher};
use crate::machine::Machine;

/// The widest the settings grow: beyond it the label and its control drift too far apart to be
/// read as one line.
const SECTION: u16 = 76;
/// From this many columns up the file's path stands to the right of the section title; below it
/// goes under the title.
const WIDE: u16 = 100;
/// The order of the choices after a member closes, as the select lists them.
const AFTER_CLOSE: [AfterClose; 2] = [AfterClose::Return, AfterClose::Shell];

/// The width of the bar a running font install shows in place of its button: wide enough to be
/// read as a bar, narrow enough to stay beside the sentence on a narrow screen.
const FONT_BAR: u16 = 16;

/// What the Settings tab knows about the Nerd Font: whether this machine has one, whether the
/// question before installing it is open, and how far the install is.
#[derive(Debug)]
pub(super) struct Font {
    /// Whether a Nerd Font lies in the font folders of this machine, read at start.
    installed: bool,
    /// Whether the question before installing is open.
    asking: bool,
    /// The last step of the install, once one has started.
    progress: Option<Progress>,
    /// Where the font is installed and whether the system is told, as the wizard does it.
    install: Install,
    /// What is downloaded: the framework's release, which the question names before anything runs.
    archive: Archive,
}

impl Font {
    /// What `machine` has, installed with `install` when it is asked to.
    pub(super) fn new(machine: &Machine, install: Install) -> Self {
        Self { installed: font_installed(machine), asking: false, progress: None, install, archive: Archive::release() }
    }

    /// Whether an install is running: a failed one may be tried again, so it is not.
    fn busy(&self) -> bool {
        matches!(self.progress, Some(Progress::Downloading { .. } | Progress::Verifying | Progress::Installing))
    }
}

/// Whether a Nerd Font is in the font folders of `machine`: the ones it names, which only a test or
/// a demo does, or else the framework's own.
fn font_installed(machine: &Machine) -> bool {
    match &machine.font_dirs {
        Some(dirs) => nerd_font::installed_in(dirs),
        None => nerd_font::installed(),
    }
}

/// A setting and its new value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
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
    /// A change on the appearance rows every Quvyta app shares.
    Appearance(AppearanceChange),
    /// How each installed member follows the shared values, read from their files.
    Followed(Vec<MemberFollowing>),
    /// Moves the keys to this row of the follow table.
    FollowSelect(usize),
    /// Puts the member at this index of [`APPS`] back on the shared value of the key.
    Follow(usize, Shared),
    /// The member's file was written, or why not.
    Follows(Result<(), String>),
    /// Opens the question before installing the Nerd Font.
    FontAsk,
    /// Closes the question without installing anything.
    FontCancel,
    /// The question was agreed to: the font is installed in the background.
    FontInstall,
    /// A step of the running install.
    FontStep(Progress),
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
            // The framework's rows write their own files, key by key, and keep the settings
            // quvyta holds in step, so nothing more is saved here.
            SettingMsg::Appearance(change) => self.appearance.update(change, &mut self.settings),
            SettingMsg::Followed(following) => self.followed(following),
            SettingMsg::FollowSelect(row) => {
                self.following_selected = Some(row);
                Command::none()
            }
            SettingMsg::Follow(index, key) => {
                let machine = self.machine.clone();
                Command::perform(move || {
                    let written = follow::follow(&machine, index, key).map_err(|error| error.to_string());
                    Msg::Setting(SettingMsg::Follows(written))
                })
            }
            // The table shows what the files say, so it is read again rather than changed here.
            SettingMsg::Follows(Ok(())) => self.read_following(),
            SettingMsg::Follows(Err(reason)) => {
                let folder = self.machine.settings_dir.as_deref().map(|folder| self.machine.show(folder));
                let body = folder.map_or(reason.clone(), |folder| format!("{folder}\n{reason}"));
                Command::batch([
                    Command::toast(Toast::warning(t!("settings.not-saved")).body(body)),
                    self.read_following(),
                ])
            }
            SettingMsg::FontAsk => {
                self.font.asking = true;
                Command::none()
            }
            SettingMsg::FontCancel => {
                self.font.asking = false;
                Command::none()
            }
            // Only the question's own button sends this, and the offer has no button while an
            // install runs, so one install never starts over another.
            SettingMsg::FontInstall => {
                self.font.asking = false;
                self.font.progress = Some(Progress::Downloading { fraction: None });
                let install = self.font.install.clone().archive(self.font.archive.clone());
                // The task is built here, where the language is known: its own thread has none.
                Command::task(install.task(|progress| Msg::Setting(SettingMsg::FontStep(progress))))
            }
            // A finished install is told in place of the offer for as long as quvyta runs, so the
            // machine is not asked again.
            SettingMsg::FontStep(progress) => {
                self.font.progress = Some(progress);
                Command::none()
            }
        }
    }

    /// Reads, in the background, how each installed member follows the shared values. Nothing is read
    /// before cargo has said what is installed: until then the table would list no one, which
    /// reads as "nothing is installed".
    pub(super) fn read_following(&self) -> Command<Msg> {
        let Some(inventory) = &self.inventory else { return Command::none() };
        // quvyta's own following is the boxes under the appearance rows.
        let members: Vec<usize> = (0..APPS.len())
            .filter(|index| matches!(inventory.state(*index), State::Cargo { .. } | State::Elsewhere { .. }))
            .collect();
        let machine = self.machine.clone();
        Command::perform(move || Msg::Setting(SettingMsg::Followed(follow::read(&machine, &members))))
    }

    /// Keeps what was read and tells, once, why a member's file could not be read.
    fn followed(&mut self, following: Vec<MemberFollowing>) -> Command<Msg> {
        let reasons: Vec<String> = following
            .iter()
            .filter_map(|member| match &member.following {
                Following::Unreadable(reasons) => Some(reasons.iter().cloned()),
                _ => None,
            })
            .flatten()
            .collect();
        let new = reasons.iter().any(|reason| !self.unreadable_told.contains(reason));
        self.following_selected = self.following_selected.filter(|row| *row < following.len());
        self.following = Some(following);
        self.unreadable_told = reasons;
        if !new {
            return Command::none();
        }
        // The reasons name the file only; the folder says where to find it.
        let folder = self.machine.settings_dir.as_deref().map(|folder| self.machine.show(folder));
        let lines: Vec<String> = folder.into_iter().chain(self.unreadable_told.iter().cloned()).collect();
        Command::toast(Toast::warning(t!("settings.follow-unreadable-told")).body(lines.join("\n")))
    }

    /// Makes `change` the running setting and returns the change that would restore the one
    /// before.
    fn apply(&mut self, change: Change) -> Change {
        match change {
            Change::AfterClose(after_close) => {
                Change::AfterClose(std::mem::replace(&mut self.launcher.after_close, after_close))
            }
        }
    }

    /// The Settings tab.
    pub(super) fn settings_page(&self, ui: &mut View<'_, Msg>) {
        // Two cells are the scrolling page's bar; the sections keep clear of it, so nothing on
        // them is cut short when the page is longer than the screen.
        let width = self.size.width.saturating_sub(2).min(SECTION);
        let page = |ui: &mut View<'_, Msg>| {
            ui.column(|ui| {
                self.appearance_settings(ui).width(Length::Cells(width)).id("appearance");
                self.following_section(width, ui);
                self.section_title(width, ui);
                self.own_settings(ui).width(Length::Cells(width)).id("settings");
                self.show_path_notice_within(width, ui);
            })
            .gap(1)
            .padding(Padding { top: 1, right: 0, bottom: 1, left: 0 })
            .fill_width();
        };
        // The shared appearance, quvyta's own settings and the PATH notice together outgrow a
        // short screen, so the page scrolls.
        ui.add_with(ScrollView::new(), page).fill().id("page");
        self.font_question(ui);
    }

    /// The appearance every Quvyta application shows the same way: language, theme and icons with
    /// their boxes, then reduced motion and the pillar, and the shared update notice. The rows
    /// are the framework's; quvyta adds none of its own, so a member's settings and quvyta's read
    /// alike.
    fn appearance_settings<'v>(&self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        SettingsList::show(ui, |list| {
            let message = |change| Msg::Setting(SettingMsg::Appearance(change));
            self.appearance.section(list, message);
            self.font_row(list);
            // quvyta asks crates.io at start, so it shows the shared switch for that, in the
            // framework's words: the same one every member that asks shows.
            self.appearance.updates(list, message);
        })
    }

    /// The Nerd Font, in the appearance section because it is what the icon set row needs: on a
    /// machine without one, the framework's sentence saying so and its Install button, then how far
    /// the install is, and once it is done what it did and what to look at. A machine that has one
    /// is told nothing.
    fn font_row(&self, list: &mut SettingsRows<'_, Msg>) {
        let font = &self.font;
        match &font.progress {
            Some(done @ Progress::Done { .. }) => {
                let row = SettingRow::new(done.text()).description(nerd_font::after_install_text());
                list.row(row, |_| {});
                // The samples the framework's note asks to look at: the Nerd glyphs, which need the
                // font, beside the Unicode ones every terminal draws.
                for (mode, name) in [(GlyphMode::Nerd, "nerd"), (GlyphMode::Unicode, "unicode")] {
                    let label = qframe::t!(&format!("quvyta.appearance.icons-{name}"));
                    list.row(SettingRow::new(label).nested(true), |ui| {
                        ui.add(GlyphSample::new(mode));
                    });
                }
            }
            _ if font.installed => {}
            Some(step) if font.busy() => {
                let row = SettingRow::new(nerd_font::status_text(false)).description(step.text());
                let bar = match step {
                    Progress::Downloading { fraction: Some(fraction) } => ProgressBar::new(*fraction).percent(true),
                    // The framework's bar keeps still by itself under reduced motion.
                    _ => ProgressBar::indeterminate(),
                };
                list.row(row, |ui| {
                    ui.add(bar).width(Length::Cells(FONT_BAR));
                });
            }
            step => {
                let row = SettingRow::new(nerd_font::status_text(false));
                // A reason is never told in colour alone: the warning mark stands in front of it.
                let row = match step {
                    Some(failed @ Progress::Failed(_)) => {
                        let mark = list.env().icons().glyph("warning").into_owned();
                        row.description(format!("{mark} {}", failed.text()))
                    }
                    _ => row,
                };
                list.row(row, |ui| {
                    ui.add(Button::new(qframe::t!("quvyta.setup.install")).on_press(Msg::Setting(SettingMsg::FontAsk)));
                });
            }
        }
    }

    /// The question before the font is installed: where it comes from, where it goes, that no sudo
    /// is needed and nothing outside the user's account is told, and that deleting that folder
    /// undoes it.
    fn font_question(&self, ui: &mut View<'_, Msg>) {
        if !self.font.asking {
            return;
        }
        let cancel = Msg::Setting(SettingMsg::FontCancel);
        let modal = Modal::new()
            .title(t!("confirm.title", command = nerd_font::FAMILY))
            .on_close(cancel.clone())
            .action(Button::new(t!("confirm.cancel")).on_press(cancel))
            .action(
                Button::new(t!("confirm.install")).variant("primary").on_press(Msg::Setting(SettingMsg::FontInstall)),
            );
        let folder = self.font.install.target_dir().map(|folder| self.machine.show(folder));
        ui.add_with(modal, |ui| {
            ui.column(|ui| {
                let mut fields = vec![
                    (t!("confirm.source"), self.font.archive.url().to_owned()),
                    (t!("confirm.version"), nerd_font::RELEASE.to_owned()),
                ];
                fields.extend(folder.clone().map(|folder| (t!("confirm.target"), folder)));
                let width = fields.iter().map(|(label, _)| qframe::text::width(label)).max().unwrap_or(0);
                ui.column(|ui| {
                    for (label, value) in &fields {
                        field(ui, label, width, value);
                    }
                })
                .fill_width();
                ui.column(|ui| {
                    let mut lines = vec![t!("font.checked")];
                    lines.extend(folder.as_deref().map(|folder| t!("confirm.no-sudo", folder = folder)));
                    lines.push(t!("font.registered"));
                    lines.push(t!("font.undo"));
                    for line in lines {
                        ui.add(Text::new(line).role("secondary")).fill_width();
                    }
                })
                .fill_width();
            })
            .gap(1)
            .fill_width();
        });
    }

    /// Whether each installed member follows the shared language, theme, icons and reduced motion:
    /// a table where its columns fit, one line per member where they do not. Nothing is drawn
    /// before what is installed is known.
    fn following_section(&self, width: u16, ui: &mut View<'_, Msg>) {
        let Some(following) = &self.following else { return };
        let words = Words::of(ui.env());
        ui.column(|ui| {
            ui.add(Text::new(t!("settings.follow-title")).role("heading")).padding(Padding::symmetric(0, 2));
            if following.is_empty() {
                ui.add(Text::new(t!("settings.follow-none")).role("faint")).padding(Padding::symmetric(0, 2));
                return;
            }
            // The table only where all its columns fit: four shared keys and a long language name
            // in some languages do not, even on a wide screen, and a column that scrolls sideways
            // hides a member's own value.
            if self.size.width >= WIDE && table_need(following, &words) <= width {
                self.following_table(following, &words, width, ui);
            } else {
                following_lines(following, &words, width, ui);
            }
            ui.add(Text::new(t!("settings.follow-next-start")).role("faint")).padding(Padding::symmetric(0, 2));
        })
        .width(Length::Cells(width))
        .id("following");
    }

    /// The follow table: a member per row, a shared key per column.
    fn following_table(&self, following: &[MemberFollowing], words: &Words, width: u16, ui: &mut View<'_, Msg>) {
        let columns = std::iter::once(Column::new(String::new()).width(ColumnWidth::Fit))
            .chain(words.keys.iter().map(|(_, name)| Column::new(name.clone()).width(ColumnWidth::Fit)));
        let rows: Vec<TableRow> = following
            .iter()
            .map(|member| {
                let command = TableCell::new(APPS[member.index].command);
                let cells = table_cells(member, words).into_iter().map(|(text, faint)| {
                    let cell = TableCell::new(text);
                    if faint { cell.role("faint") } else { cell }
                });
                TableRow::new(std::iter::once(command).chain(cells))
            })
            .collect();
        // A row is acted on only by putting its own keys back on the shared ones, so its menu is what
        // enter and a click open; a row that shares everything, or cannot be read, opens nothing.
        let menus: Vec<Vec<ContextItem<Msg>>> = following
            .iter()
            .map(|member| follow_choices(member).map(|(key, msg)| ContextItem::new(follow_label(key), msg)).collect())
            .collect();
        let select = |row: usize| Msg::Setting(SettingMsg::FollowSelect(row));
        let table = Table::new(columns, rows)
            .selected(self.following_selected)
            .on_select(select)
            .context_menu(move |row| menus.get(row).cloned().unwrap_or_default())
            .menu_on_activate(true);
        ui.add(table).width(Length::Cells(width)).id("following-table");
    }

    /// The title of quvyta's own section, drawn as the framework draws a heading, and, fainter,
    /// the file it is kept in: beside the title on a wide screen, under it on a narrow one,
    /// shortened in the middle when it does not fit, since the file name at the end matters as
    /// much as the folder at the start.
    fn section_title(&self, width: u16, ui: &mut View<'_, Msg>) {
        let words = t!("settings.title");
        let title = Text::new(words.clone()).role("heading").no_wrap();
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
                ui.add(Text::new(qframe::text::truncate_middle(&path, beside)).role("faint").no_wrap());
            })
            .padding(Padding::symmetric(0, 2))
            .width(Length::Cells(width));
        } else {
            ui.column(|ui| {
                ui.add(title);
                ui.add(Text::new(qframe::text::truncate_middle(&path, room)).role("faint").no_wrap());
            })
            .padding(Padding::symmetric(0, 2))
            .width(Length::Cells(width));
        }
    }

    /// The rows of `launcher.conf`, each written as soon as it changes.
    fn own_settings<'v>(&self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let launcher = &self.launcher;
        SettingsList::show(ui, |list| {
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

/// The choices that put one of `member`'s own keys back on the shared value, one per key it
/// does not share, with the message each sends. None for a member that shares everything, has
/// not been opened, or has a file quvyta could not read and so never writes.
fn follow_choices(member: &MemberFollowing) -> impl Iterator<Item = (Shared, Msg)> + '_ {
    let values = match &member.following {
        Following::Keys(values) => values.as_slice(),
        Following::NotOpened | Following::Unreadable(_) => &[],
    };
    values
        .iter()
        .filter(|(_, own)| own.is_some())
        .map(|(key, _)| (*key, Msg::Setting(SettingMsg::Follow(member.index, *key))))
}

/// The follow table's cells of `member` after its name, one per shared key, each with whether it
/// is drawn faint.
fn table_cells(member: &MemberFollowing, words: &Words) -> Vec<(String, bool)> {
    match &member.following {
        // One word for the row: it has no values to spread over the columns.
        Following::NotOpened => std::iter::once((t!("settings.follow-not-opened"), true))
            .chain(std::iter::repeat_with(|| (String::new(), false)))
            .take(words.keys.len())
            .collect(),
        Following::Unreadable(_) => words.keys.iter().map(|_| (t!("settings.follow-unreadable"), true)).collect(),
        // Each column finds its own key's value, so the order of what was read does not have to be
        // the order of the columns.
        Following::Keys(values) => words
            .keys
            .iter()
            .map(|(key, _)| match values.iter().find(|(read, _)| read == key).and_then(|(_, own)| own.as_ref()) {
                Some(value) => (words.value(*key, value), false),
                None => (t!("settings.follow-shared"), true),
            })
            .collect(),
    }
}

/// The cells the follow table needs to show every column whole, measured the way the framework's
/// table sizes a column that fits its content: its widest cell and one cell more, or its title,
/// with two cells between columns, and a few cells for the selection mark and the section's
/// sides.
fn table_need(following: &[MemberFollowing], words: &Words) -> u16 {
    let width = |text: &str| qframe::text::width(text);
    let mut columns: Vec<u16> =
        std::iter::once(0).chain(words.keys.iter().map(|(_, name)| width(name).saturating_sub(1))).collect();
    for member in following {
        let cells: Vec<u16> = std::iter::once(width(APPS[member.index].command))
            .chain(table_cells(member, words).iter().map(|(text, _)| width(text)))
            .collect();
        for (column, cell) in columns.iter_mut().zip(cells) {
            *column = (*column).max(cell);
        }
    }
    let gaps = 2 * u16::try_from(columns.len().saturating_sub(1)).unwrap_or(u16::MAX);
    columns.iter().fold(gaps, |sum, column| sum.saturating_add(column + 1)).saturating_add(8)
}

/// The menu entry that puts `key` back on the shared value.
fn follow_label(key: Shared) -> String {
    follow::SHOWN.iter().find(|(shown, ..)| *shown == key).map_or_else(|| key.key().to_owned(), |(.., label)| t!(label))
}

/// One line per member for a narrow screen: its command, then only the keys it does not share
/// with the others, or that it shares them all. When a member's own keys do not fit on one line
/// in `width` cells, each takes a line of its own under the first, so a line never breaks inside
/// a value.
fn following_lines(following: &[MemberFollowing], words: &Words, width: u16, ui: &mut View<'_, Msg>) {
    let name_width =
        following.iter().map(|member| qframe::text::width(APPS[member.index].command)).max().unwrap_or(0) + 2;
    // The section's sides and the name column leave this much for the keys.
    let room = width.saturating_sub(4 + name_width);
    for member in following {
        let (text, own) = match &member.following {
            Following::NotOpened => (t!("settings.follow-not-opened"), false),
            Following::Unreadable(_) => (t!("settings.follow-unreadable"), false),
            Following::Keys(values) => {
                let own: Vec<String> = values
                    .iter()
                    .filter_map(|(key, value)| {
                        let value = words.value(*key, value.as_deref()?);
                        Some(t!("settings.follow-own", key = words.key(*key), value = value))
                    })
                    .collect();
                let line = own.join("  ");
                if own.is_empty() {
                    (t!("settings.follow-all-shared"), false)
                } else if qframe::text::width(&line) <= room {
                    (line, true)
                } else {
                    (own.join("\n"), true)
                }
            }
        };
        ui.row(|ui| {
            ui.add(Text::new(APPS[member.index].command).no_wrap()).width(Length::Cells(name_width));
            // A long language name wraps under itself rather than being cut.
            let text = Text::new(text);
            ui.add(if own { text } else { text.role("faint") }).fill_width();
        })
        .padding(Padding::symmetric(0, 2));
        // The table's menu is not there on a narrow screen, so its choices stand under the member
        // as buttons, moving to the next line when they do not fit.
        let choices: Vec<(Shared, Msg)> = follow_choices(member).collect();
        if !choices.is_empty() {
            ui.row(|ui| {
                ui.add(Text::new(t!("settings.follow-narrow")).role("faint").no_wrap());
                for (key, msg) in choices {
                    ui.add(Button::new(words.key(key)).on_press(msg));
                }
            })
            .gap(1)
            .wrap(true)
            .padding(Padding { top: 0, right: 2, bottom: 0, left: 2 + name_width });
        }
    }
}

/// The words the follow table shows for keys and values, in the language on screen.
struct Words {
    /// Each shared key the table shows, in its order, with its name as the appearance rows name
    /// it.
    keys: Vec<(Shared, String)>,
    /// Each language code with its name in that language.
    languages: Vec<(String, String)>,
    /// Each theme id with its name.
    themes: Vec<(String, String)>,
    /// Each icon set with its name, as the appearance rows name it.
    icons: Vec<(IconMode, String)>,
}

impl Words {
    fn of(env: &qframe::env::Env) -> Self {
        Self {
            keys: follow::SHOWN
                .iter()
                .map(|(key, name, _)| (*key, name.map_or_else(|| Appearance::label(env.i18n(), *key), |name| t!(name))))
                .collect(),
            icons: IconMode::ALL.iter().map(|mode| (*mode, mode.label(env.i18n()))).collect(),
            languages: env.i18n().list(),
            themes: env.themes(),
        }
    }

    fn key(&self, key: Shared) -> &str {
        self.keys.iter().find(|(shown, _)| *shown == key).map_or_else(|| key.key(), |(_, name)| name.as_str())
    }

    /// `value` of `key` as the appearance rows would name it: a language in its own name, a
    /// theme's name, an icon set's name. A value this quvyta does not know, such as a theme a
    /// newer member brought, is shown as it is written.
    fn value(&self, key: Shared, value: &str) -> String {
        let named = |names: &[(String, String)]| names.iter().find(|(id, _)| id == value).map(|(_, name)| name.clone());
        // Compared, not matched: a shared setting the framework adds later is shown as written.
        let name = if key == Shared::Language {
            named(&self.languages)
        } else if key == Shared::Theme {
            named(&self.themes)
        } else if key == Shared::Icons {
            IconMode::from_name(value)
                .and_then(|mode| self.icons.iter().find(|(shown, _)| *shown == mode).map(|(_, name)| name.clone()))
        } else if key == Shared::ReducedMotion {
            value
                .parse::<bool>()
                .ok()
                .map(|reduced| t!(if reduced { "settings.motion-reduced" } else { "settings.motion-full" }))
        } else {
            None
        };
        name.unwrap_or_else(|| value.to_owned())
    }
}

#[cfg(test)]
pub(super) mod tests;

#[cfg(test)]
pub(super) mod font_tests;
