//! The screen: the family as a list and the chosen member's details beside it, or on a page of
//! their own when the terminal is narrow.

mod confirm_install;
mod detail;
mod install_view;
mod installs;
mod open;
mod path_notice;
mod settings;
mod updates;

use std::collections::VecDeque;

use qframe::prelude::*;
use qframe::runtime::{HandoffOutcome, Termination};
use qframe::widgets::{ScrollView, Splitter, Tabs, Toast};

use crate::family::{FAMILY, Member, Status};
use crate::inventory::{Inventory, State};
use crate::launcher::{AfterClose, Launcher};
use crate::machine::Machine;
pub use installs::InstallMsg;
use installs::Installs;
use open::Opening;
pub use path_notice::PathMsg;
use path_notice::PathNotice;
pub use settings::{Change, SettingMsg};
pub use updates::UpdateMsg;
use updates::Updates;

/// Below this many columns the list and the details take turns on the screen.
const WIDE: u16 = 60;
/// The fewest columns the details keep beside the list.
const DETAIL_MIN: u16 = 28;
/// The fewest columns the list takes.
const LIST_MIN: u16 = 24;

/// The tabs in the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// The family: the list and the chosen member's details.
    #[default]
    Apps,
    /// quvyta's own settings.
    Settings,
}

/// The tabs in the order the header shows them.
const TABS: [Tab; 2] = [Tab::Apps, Tab::Settings];

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
    /// Members the command line asked to install whose dialog has not opened yet, in order.
    asked: VecDeque<usize>,
    /// The tab shown.
    tab: Tab,
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
    /// Shows this tab.
    Tab(Tab),
    /// Something on the Settings tab.
    Setting(SettingMsg),
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
            asked: VecDeque::new(),
            tab: Tab::default(),
        }
    }

    /// Opens on the install dialogs of `members`, indexes of [`FAMILY`], one after another, as
    /// soon as it is known which of them are installed. A member not released yet is left out:
    /// it has no dialog, and it is not there already either.
    #[must_use]
    pub fn asking(mut self, members: Vec<usize>) -> Self {
        self.asked = members.into_iter().filter(|index| FAMILY.get(*index).is_some_and(Member::published)).collect();
        self
    }

    /// Opens on the page of the member at `index` of [`FAMILY`]: selected in the list with its
    /// details beside it, or on a narrow screen its details alone, as a first `enter` shows them,
    /// with `esc` back to the list. The size is not known yet, so the page is chosen for both.
    #[must_use]
    pub fn showing(mut self, index: usize) -> Self {
        if index < FAMILY.len() {
            self.selected = index;
            self.detail_page = true;
        }
        self
    }

    /// Opens the dialog of the next member asked for that can be installed. The ones that are
    /// there already are not installed again: the first of them is shown with its details,
    /// where its version and any update are, and a toast names them all.
    fn ask_next(&mut self) -> Command<Msg> {
        let mut there = Vec::new();
        while let Some(index) = self.asked.pop_front() {
            if self.installable(index) {
                // The dialog belongs to the list, which shows what it starts.
                self.tab = Tab::Apps;
                self.selected = index;
                return Command::batch([already_installed(&there), self.install_update(InstallMsg::Ask(index))]);
            }
            there.push(index);
        }
        if let Some(first) = there.first().copied()
            && self.installs.dialog.is_none()
        {
            let shown = self.update(if self.wide() { Msg::Select(first) } else { Msg::ShowDetail(first) });
            return Command::batch([already_installed(&there), shown]);
        }
        already_installed(&there)
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
        // The family's keys act on the list; on the Settings tab they would act on a member
        // nobody sees.
        if self.tab == Tab::Settings {
            return (name == "back").then_some(Msg::Back);
        }
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
            // From the settings esc goes back to the family as it was left, a member's page
            // included; the keys go to the list when it is on screen.
            Msg::Back if self.tab == Tab::Settings => {
                self.tab = Tab::Apps;
                if self.wide() || !self.detail_page {
                    return Command::focus("family");
                }
            }
            Msg::Back => {
                self.detail_page = false;
                return Command::focus("family");
            }
            Msg::Inventory(inventory) => {
                let first = self.inventory.replace(inventory).is_none();
                if first {
                    return Command::batch([self.check_path_at_start(), self.ask_next()]);
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
            Msg::Install(msg) => {
                // Answering one dialog the command line asked for opens the next.
                let answered = matches!(msg, InstallMsg::Close | InstallMsg::Confirm);
                let done = self.install_update(msg);
                if answered && self.installs.dialog.is_none() {
                    return Command::batch([done, self.ask_next()]);
                }
                return done;
            }
            Msg::Path(msg) => return self.update_path(msg),
            Msg::Updates(msg) => return self.update_update(msg),
            // The header's keys and a click leave the keys on the tabs, where they were.
            Msg::Tab(tab) => self.tab = tab,
            Msg::Setting(msg) => return self.update_setting(msg),
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        AppShell::new()
            .header(|ui| self.header(ui))
            .body(|ui| match self.tab {
                Tab::Apps => self.body(ui),
                Tab::Settings => self.settings_page(ui),
            })
            .footer(|ui| self.footer(ui))
            .show(ui);
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
                (None, state) => row_state(&FAMILY[index], state, compact),
            })
        })
    }

    /// The columns the list may take: beside the details, or the whole screen but a margin.
    fn list_room(&self) -> u16 {
        self.size.width.saturating_sub(if self.wide() { DETAIL_MIN } else { 1 })
    }

    /// The columns the details have: beside the list, or the screen.
    fn detail_width(&self) -> u16 {
        if self.wide() { self.size.width.saturating_sub(self.list_column()) } else { self.size.width }
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
        let hints = if self.tab == Tab::Settings {
            KeyHints::new()
                .hint("↑↓", t!("hints.choose"))
                .hint("esc", t!("hints.apps"))
                .action(Scope::Global, "focus-next")
        } else if self.wide() || self.detail_page {
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
/// Says which of the members at `indexes` were asked for but are installed already.
fn already_installed(indexes: &[usize]) -> Command<Msg> {
    if indexes.is_empty() {
        return Command::none();
    }
    let commands: Vec<&str> = indexes.iter().map(|index| FAMILY[*index].command).collect();
    Command::toast(Toast::info(t!("cli.already-installed", commands = commands.join(", "))))
}

/// `compact` leaves out quvyta's own version, which its details show anyway.
fn row_state(member: &Member, state: &State, compact: bool) -> String {
    match state {
        // Not there because it is not out yet: "not installed" would suggest installing it.
        State::Missing if member.status == Status::Soon => t!("row.soon"),
        State::Missing => t!("row.missing"),
        State::Cargo { version, .. } => version.clone(),
        State::Elsewhere { .. } => t!("row.unknown"),
        State::This { .. } if compact => t!("row.this-short"),
        State::This { version, .. } => t!("row.this", version = *version),
    }
}

impl Quvyta {
    /// The name, the tagline when there is room for it whole, and the tabs at the right.
    fn header(&self, ui: &mut View<'_, Msg>) {
        let labels = [t!("tabs.apps"), t!("tabs.settings")];
        // Each tab pads its label with two cells on both sides; one cell parts them.
        let tabs_width: u16 = labels.iter().map(|label| qframe::text::width(label) + 4).sum::<u16>() + 1;
        let tagline = format!("  {}", t!("header.tagline"));
        // The name and the tagline keep at least four cells from the tabs, so the first tab does
        // not read as the tagline's last word.
        let room = self.size.width.saturating_sub(4 + tabs_width + 4);
        let mut spans = vec![Span::new("quvyta").color("accent").bold()];
        // A tagline cut short reads worse than none.
        if qframe::text::width("quvyta") + qframe::text::width(&tagline) <= room {
            spans.push(Span::new(tagline).role("faint"));
        }
        ui.row(|ui| {
            ui.add(Text::rich(spans).no_wrap()).fill_width();
            let active = TABS.iter().position(|tab| *tab == self.tab).unwrap_or(0);
            let open = |index: usize| Msg::Tab(TABS.get(index).copied().unwrap_or_default());
            ui.add(Tabs::new(labels).active(active).on_select(open)).id("tabs");
        })
        .padding(Padding::symmetric(0, 2))
        .fill_width();
    }
}

#[cfg(test)]
mod cli_tests;
#[cfg(test)]
mod install_tests;
#[cfg(test)]
mod remove_tests;
#[cfg(test)]
mod soon_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod update_tests;
