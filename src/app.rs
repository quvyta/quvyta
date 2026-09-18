//! The screen: the family as a list and the chosen member's details beside it, or on a page of
//! their own when the terminal is narrow.

mod confirm_install;
mod detail;
mod install_view;
mod installs;
mod open;
mod path_notice;
mod updates;

use qframe::prelude::*;
use qframe::runtime::{HandoffOutcome, Termination};
use qframe::widgets::{ScrollView, Splitter, Toast};

use crate::family::FAMILY;
use crate::inventory::{Inventory, State};
use crate::launcher::{AfterClose, Launcher};
use crate::machine::Machine;
use installs::{InstallMsg, Installs};
use open::Opening;
pub use path_notice::PathMsg;
use path_notice::PathNotice;
pub use updates::UpdateMsg;
use updates::Updates;

/// Below this many columns the list and the details take turns on the screen.
const WIDE: u16 = 60;
/// The fewest columns the details keep beside the list.
const DETAIL_MIN: u16 = 28;
/// The fewest columns the list takes.
const LIST_MIN: u16 = 24;

/// The application's state.
#[derive(Debug)]
pub struct Quvyta {
    machine: Machine,
    /// What is installed; `None` until cargo has answered.
    inventory: Option<Inventory>,
    /// The member shown, an index of [`FAMILY`].
    selected: usize,
    size: Size,
    /// On a narrow screen, whether the details have the screen instead of the list.
    detail_page: bool,
    /// quvyta's own settings, read when it starts.
    launcher: Launcher,
    /// Installs running, waiting and ended.
    installs: Installs,
    /// What the detail area says about starting members by name; `None` when there is nothing
    /// to say.
    path_notice: Option<PathNotice>,
    /// The newest versions on crates.io and how the last check went.
    updates: Updates,
}

/// Everything that can happen.
#[derive(Debug, Clone)]
pub enum Msg {
    /// The terminal has this size now.
    Resized(Size),
    /// Selects the member at this index of [`FAMILY`].
    Select(usize),
    /// On a narrow screen, shows the details of the member at this index.
    ShowDetail(usize),
    /// Leaves the narrow details for the list.
    Back,
    /// What is installed, read in the background.
    Inventory(Inventory),
    /// Runs the main button of the member shown: opens it when it is installed, asks to install
    /// it when it is not.
    Primary,
    /// Opens the member at this index.
    Open(usize),
    /// The member at `index` closed and quvyta has the screen back.
    Closed {
        /// The member, an index of [`FAMILY`].
        index: usize,
        /// How it ended.
        outcome: HandoffOutcome,
    },
    /// Something about installing.
    Install(InstallMsg),
    /// The PATH notice.
    Path(PathMsg),
    /// Something about updates.
    Updates(UpdateMsg),
}

impl Quvyta {
    /// The application for `machine`.
    pub fn new(machine: Machine) -> Self {
        Self {
            machine,
            inventory: None,
            selected: 0,
            size: Size::default(),
            detail_page: false,
            launcher: Launcher::default(),
            installs: Installs::default(),
            path_notice: None,
            updates: Updates::default(),
        }
    }

    fn wide(&self) -> bool {
        self.size.width >= WIDE
    }

    /// Reads what is installed without holding up the screen: cargo takes a moment to answer.
    fn read_inventory(&self) -> Command<Msg> {
        let machine = self.machine.clone();
        Command::perform(move || Msg::Inventory(Inventory::read(&machine)))
    }

    fn state(&self, index: usize) -> Option<&State> {
        self.inventory.as_ref().map(|inventory| inventory.state(index))
    }

    /// How the member at `index` would be opened; `None` when it cannot be.
    fn opening(&self, index: usize) -> Option<Opening> {
        let program = self.state(index)?.program()?.clone();
        Some(Opening { program, dir: self.machine.home.clone() })
    }

    /// Whether quvyta should step aside for good once a member it opened closes.
    fn leaves_with_member(&self) -> bool {
        self.launcher.after_close == AfterClose::Shell
    }

    /// Tells what was wrong in `launcher.conf`, once, as it starts.
    fn settings_problems(&self) -> Command<Msg> {
        if self.launcher.diagnostics.is_empty() {
            return Command::none();
        }
        // The diagnostics name the file only; the full path says where to find it.
        let path = self.machine.launcher_conf.as_deref().map(|path| self.machine.show(path));
        let lines: Vec<String> =
            path.into_iter().chain(self.launcher.diagnostics.iter().map(ToString::to_string)).collect();
        Command::toast(Toast::warning(t!("launcher.problems")).body(lines.join("\n")))
    }
}

impl App for Quvyta {
    type Msg = Msg;

    fn init(&mut self) -> Command<Msg> {
        self.launcher = Launcher::load(self.machine.launcher_conf.as_deref());
        // Turned off, nothing asks crates.io unasked; `r` still does, since that is asking.
        let updates = if self.launcher.check_updates { self.check_updates(false) } else { Command::none() };
        Command::batch([
            Command::focus("family"),
            self.read_inventory(),
            self.settings_problems(),
            self.clear_leftover(),
            updates,
        ])
    }

    fn before_quit(&self) -> Option<Msg> {
        self.ask_before_quit()
    }

    fn terminating(&self, cause: Termination) -> Option<Msg> {
        match cause {
            // The terminal is gone and nobody can answer; cargo is stopped rather than left
            // building unseen.
            Termination::Hangup => self.installs.busy().then_some(Msg::Install(InstallMsg::Quit)),
            _ => self.before_quit(),
        }
    }

    fn resized(&self, size: Size) -> Option<Msg> {
        Some(Msg::Resized(size))
    }

    fn action(&self, name: &str) -> Option<Msg> {
        match name {
            "back" if self.detail_page && !self.wide() => Some(Msg::Back),
            "primary" => Some(Msg::Primary),
            "refresh" => Some(Msg::Updates(UpdateMsg::Check)),
            "update" => self.ask_update(self.selected),
            _ => None,
        }
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Resized(size) => self.size = size,
            Msg::Select(index) if index < FAMILY.len() => self.selected = index,
            Msg::ShowDetail(index) if index < FAMILY.len() => {
                self.selected = index;
                self.detail_page = true;
            }
            Msg::Select(_) | Msg::ShowDetail(_) => {}
            Msg::Back => {
                self.detail_page = false;
                return Command::focus("family");
            }
            Msg::Inventory(inventory) => {
                let first = self.inventory.replace(inventory).is_none();
                if first {
                    return self.check_path_at_start();
                }
            }
            // On a narrow list the details come first; the button is on their page.
            Msg::Primary if !self.wide() && !self.detail_page => return self.update(Msg::ShowDetail(self.selected)),
            // quvyta has nothing to open, so its update is its main button.
            Msg::Primary
                if self.installable(self.selected)
                    || (self.opening(self.selected).is_none() && self.updatable(self.selected)) =>
            {
                return self.update(Msg::Install(InstallMsg::Ask(self.selected)));
            }
            Msg::Primary => return self.update(Msg::Open(self.selected)),
            Msg::Open(index) => {
                if let Some(opening) = self.opening(index) {
                    return opening.handoff(index, &FAMILY[index]);
                }
            }
            Msg::Closed { index, outcome } => {
                let Some(member) = FAMILY.get(index) else { return Command::none() };
                // An install keeps quvyta here: stopping it unasked would lose the build.
                if self.leaves_with_member()
                    && matches!(outcome, HandoffOutcome::Finished { .. })
                    && !self.installs.busy()
                {
                    return Command::quit();
                }
                // The member may have changed what is installed, even itself.
                return Command::batch([open::report(member, &outcome), self.read_inventory()]);
            }
            Msg::Install(msg) => return self.install_update(msg),
            Msg::Path(msg) => return self.update_path(msg),
            Msg::Updates(msg) => return self.update_update(msg),
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        AppShell::new().header(header).body(|ui| self.body(ui)).footer(|ui| self.footer(ui)).show(ui);
        self.install_dialogs(ui);
    }
}

impl Quvyta {
    fn body(&self, ui: &mut View<'_, Msg>) {
        if self.wide() {
            Splitter::columns(self.list_column())
                .first(|ui| {
                    self.list_area(ui);
                })
                .second(|ui| self.detail(ui))
                .show(ui)
                .padding(Padding { top: 1, right: 0, bottom: 0, left: 0 });
        } else if self.detail_page {
            ui.column(|ui| self.detail(ui)).padding(Padding { top: 1, right: 0, bottom: 0, left: 0 }).fill();
        } else {
            ui.column(|ui| self.list_area(ui)).padding(Padding { top: 1, right: 1, bottom: 0, left: 0 }).fill();
        }
    }

    /// The list, and under it what there is to say about updates.
    fn list_area(&self, ui: &mut View<'_, Msg>) {
        ui.column(|ui| {
            self.list(ui);
            self.update_bar(ui);
        })
        .gap(1)
        .fill();
    }

    /// The columns the list area has: its column beside the details, or the screen.
    fn list_area_width(&self) -> u16 {
        if self.wide() { self.list_column() } else { self.list_room() }
    }

    fn list(&self, ui: &mut View<'_, Msg>) {
        let compact = self.compact();
        let items = FAMILY.iter().enumerate().map(|(index, member)| {
            // A failure keeps its mark in the list until it is dismissed; the word says it too.
            let item = if self.install_failed(index) {
                ListItem::new(member.command).icon("error", Some("danger"))
            } else {
                ListItem::new(member.command).icon(member.icon, None)
            };
            match self.row_text(index, compact) {
                Some(text) => item.detail(text),
                None => item,
            }
        });
        let list = List::new(items).selected(Some(self.selected)).on_select(Msg::Select);
        // On a narrow screen a row opens the details; on a wide one they are already beside it.
        let list = if self.wide() { list } else { list.on_activate(Msg::ShowDetail) };
        ui.add(list).fill().id("family");
    }

    /// The width the list needs for its widest row, `compact` or not, as the list measures it:
    /// the pillar and the icon with their air, the label, the state with its air and a margin.
    fn list_width(&self, compact: bool) -> u16 {
        let row = |index: usize| {
            let state = self.row_text(index, compact).map_or(0, |text| qframe::text::width(&text) + 2);
            qframe::text::width(FAMILY[index].command) + state + 7
        };
        (0..FAMILY.len()).map(row).max().unwrap_or(0)
    }

    /// What the row of the member at `index` says: an install under way, or how it is installed
    /// and the newer version when there is one.
    fn row_text(&self, index: usize, compact: bool) -> Option<String> {
        self.install_row(index).or_else(|| {
            let state = self.state(index)?;
            Some(match (self.update_to(index), state) {
                (Some(latest), State::This { version, .. }) => t!("row.update", version = *version, latest = latest),
                (Some(latest), _) => {
                    t!("row.update", version = state.cargo_version().unwrap_or_default(), latest = latest)
                }
                (None, state) => row_state(state, compact),
            })
        })
    }

    /// The columns the list may take: beside the details, or the whole screen but a margin.
    fn list_room(&self) -> u16 {
        self.size.width.saturating_sub(if self.wide() { DETAIL_MIN } else { 1 })
    }

    /// Whether the rows are too wide for their room and should say less.
    fn compact(&self) -> bool {
        self.list_width(false) > self.list_room()
    }

    /// The list column: as wide as its rows, leaving the details room to read.
    fn list_column(&self) -> u16 {
        self.list_width(self.compact()).clamp(LIST_MIN, self.list_room().max(LIST_MIN))
    }

    fn detail(&self, ui: &mut View<'_, Msg>) {
        let member = &FAMILY[self.selected];
        let narrow = !self.wide();
        // Keyed by member so copy confirmations do not carry over to the next one.
        ui.add_with(ScrollView::new(), |ui| {
            ui.column(|ui| {
                if narrow {
                    ui.add(Button::new(t!("nav.back")).icon("arrow-left").on_press(Msg::Back)).id("back");
                }
                self.show_path_notice(ui);
                let main = |ui: &mut View<'_, Msg>| self.main_action(self.selected, ui);
                let latest = self.update_to(self.selected);
                detail::show(member, self.state(self.selected), latest.as_deref(), main, &self.machine, ui);
            })
            .gap(1)
            .padding(Padding { top: 0, right: 2, bottom: 1, left: 2 })
            .fill_width()
            .id(member.key);
        })
        .fill();
    }

    fn footer(&self, ui: &mut View<'_, Msg>) {
        let hints = if self.wide() || self.detail_page {
            let hints = KeyHints::new();
            let hints = if self.installable(self.selected) {
                hints.hint("enter", t!("hints.install"))
            } else if self.opening(self.selected).is_some() {
                hints.hint("enter", t!("hints.open"))
            } else if self.updatable(self.selected) {
                hints.hint("enter", t!("hints.update"))
            } else {
                hints
            };
            // Where enter updates already, `u` would say it twice.
            let hints = if self.updatable(self.selected) && self.opening(self.selected).is_some() {
                hints.hint("u", t!("hints.update"))
            } else {
                hints
            };
            let hints = if self.wide() {
                hints.hint("↑↓", t!("hints.choose"))
            } else {
                hints.hint("esc", t!("hints.back"))
            };
            hints.action(Scope::Global, "focus-next").hint("r", t!("hints.refresh"))
        } else {
            KeyHints::new()
                .hint("enter", t!("hints.details"))
                .hint("↑↓", t!("hints.choose"))
                .hint("r", t!("hints.refresh"))
        };
        ui.add(hints.action_right(Scope::Global, "quit")).fill_width();
    }
}

/// What a list row says about how its member is installed: in words, never in colour alone.
/// `compact` leaves out quvyta's own version, which its details show anyway.
fn row_state(state: &State, compact: bool) -> String {
    match state {
        State::Missing => t!("row.missing"),
        State::Cargo { version, .. } => version.clone(),
        State::Elsewhere { .. } => t!("row.unknown"),
        State::This { .. } if compact => t!("row.this-short"),
        State::This { version, .. } => t!("row.this", version = *version),
    }
}

fn header(ui: &mut View<'_, Msg>) {
    ui.add(
        Text::rich([
            Span::new("quvyta").color("accent").bold(),
            Span::new(format!("  {}", t!("header.tagline"))).role("faint"),
        ])
        .no_wrap(),
    )
    .padding(Padding::symmetric(0, 2))
    .fill_width();
}

#[cfg(test)]
mod install_tests;
#[cfg(test)]
mod remove_tests;
#[cfg(test)]
mod scenes;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod update_tests;
