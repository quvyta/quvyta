//! The screen: the Quvyta apps as a list and the chosen member's details beside it, or on a page of
//! their own when the terminal is narrow.

mod confirm_install;
mod detail;
mod follow;
mod install_view;
mod installs;
mod open;
mod path_notice;
mod settings;
mod updates;
mod whats_new;
mod wizard;

use std::collections::VecDeque;

use qframe::icons::nerd_font::Install;
use qframe::prelude::*;
use qframe::runtime::{HandoffOutcome, Termination};
use qframe::storage::{Family, Preferences, Settings};
use qframe::widgets::{Appearance, Markdown, Panel, ScrollView, Setup, SetupMsg, Splitter, Tabs, Toast};

use crate::ecosystem::{APPS, Member, NotHere, Status};
use crate::inventory::{Inventory, State};
use crate::launcher::{AfterClose, Launcher};
use crate::machine::{LAUNCHER, Machine};
pub use follow::{Following, MemberFollowing};
pub use installs::InstallMsg;
use installs::Installs;
use open::Opening;
pub use path_notice::PathMsg;
use path_notice::PathNotice;
pub use settings::{Change, SettingMsg};
pub use updates::UpdateMsg;
use updates::Updates;
pub use wizard::WizardMsg;

/// The fewest columns the split takes: under this the list and the details take turns on the
/// screen, however little the words of the day's language ask for.
const WIDE: u16 = 60;
/// The fewest columns the details keep beside the list.
const DETAIL_MIN: u16 = 28;
/// The fewest columns the list takes.
const LIST_MIN: u16 = 24;

/// The tabs in the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// The Quvyta apps: the list and the chosen member's details.
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
    /// The member shown, an index of [`APPS`].
    selected: usize,
    size: Size,
    /// On a narrow screen, whether the details have the screen instead of the list.
    detail_page: bool,
    /// Whether the page of what changed in the running version is over the list; `true` on the
    /// first start of a version that is not the one the data folder holds.
    whats_new: bool,
    /// quvyta's own settings, read when it starts.
    launcher: Launcher,
    /// The same file as the framework reads it, `launcher.conf` in the shared folder: what the
    /// appearance rows write their own settings into.
    settings: Settings,
    /// The shared appearance, language, theme and icons, and the rows that change it.
    appearance: Appearance,
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
    /// How each installed member follows the shared settings, in the order of [`APPS`];
    /// `None` until it has been read, which waits for what is installed.
    following: Option<Vec<follow::MemberFollowing>>,
    /// The row of the follow table the keys are on.
    following_selected: Option<usize>,
    /// The reasons members' files could not be read that were already told, so reading them again
    /// does not tell them twice.
    unreadable_told: Vec<String>,
    /// The first-run wizard, while quvyta has no settings file of its own; `None` once it is
    /// over, and from the start for someone who has quvyta's file already.
    setup: Option<Setup<Msg>>,
    /// Which members are checked on the wizard's own step, by index of [`APPS`].
    picked: [bool; APPS.len()],
    /// Whether this is Arch Linux, which is where the members marked `arch_only` run. Read once
    /// at start, since the app list offers members on every screen and not only on the wizard's.
    arch: bool,
    /// The Nerd Font the Settings tab offers while this machine has none.
    font: settings::Font,
}

/// Everything that can happen.
#[derive(Debug, Clone)]
pub enum Msg {
    /// The terminal has this size now.
    Resized(Size),
    /// Selects the member at this index of [`APPS`].
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
        /// The member, an index of [`APPS`].
        index: usize,
        /// How it ended.
        outcome: HandoffOutcome,
    },
    /// Something about installing.
    Install(InstallMsg),
    /// The member at this index does not run on this machine, and the screen says so instead of
    /// offering to install it.
    NotInstallable(usize),
    /// The PATH notice.
    Path(PathMsg),
    /// Something about updates.
    Updates(UpdateMsg),
    /// Shows this tab.
    Tab(Tab),
    /// Something on the Settings tab.
    Setting(SettingMsg),
    /// Something on the first step of the setup wizard, which the framework answers.
    Setup(SetupMsg),
    /// Something on quvyta's own step of the setup wizard.
    Wizard(WizardMsg),
    /// The wizard wrote what it was told and is over.
    SetUp,
}

/// The shared appearance of `machine`, with the rows writing into the same folder the
/// preferences were read from: a machine rooted in a folder of its own never touches the user's
/// settings.
fn appearance_of(machine: &Machine, preferences: Preferences) -> Appearance {
    let appearance = Appearance::new(Family::QUVYTA, LAUNCHER, preferences);
    match machine.settings_dir.as_deref() {
        Some(folder) => appearance.in_folder(folder),
        None => appearance,
    }
}

/// The first-run wizard when quvyta has no settings file of its own yet, and nothing when it has
/// one or when this platform names no settings folder at all, where there would be nowhere to
/// write what the wizard asks.
fn setup(machine: &Machine, i18n: &qframe::i18n::I18n) -> Option<Setup<Msg>> {
    let folder = machine.settings_dir.as_deref()?;
    let mut setup = Setup::new_in(folder, Family::QUVYTA, LAUNCHER, i18n, Msg::Setup).on_finish(Msg::SetUp);
    // A machine with font folders of its own is a test or a demo: no real font is looked at, and
    // none is installed or registered.
    if let Some(dirs) = &machine.font_dirs {
        setup = setup.install(font_install(machine)).font_dirs(dirs.clone());
    }
    setup.needed().then_some(setup)
}

/// How a Nerd Font is installed on `machine`, the same for the wizard and the Settings tab: as the
/// framework does it for this user, unless the machine names font folders of its own. That is a
/// test or a demo, whose folders no terminal reads, so the font goes into the first of them and
/// the system is not told.
fn font_install(machine: &Machine) -> Install {
    match &machine.font_dirs {
        Some(dirs) => {
            let fonts = dirs.first().cloned().unwrap_or_else(|| machine.home.join("fonts"));
            Install::new().target(fonts.join("QuvytaNerdFont")).register(false)
        }
        None => Install::new(),
    }
}

impl Quvyta {
    /// The application for `machine`.
    ///
    /// The shared appearance is resolved here, before the first frame: quvyta's own file
    /// when it names a language, theme or icon set of its own, else the shared file,
    /// else what the machine asks for. [`Quvyta::preferences`] and [`Quvyta::settings`] hand it
    /// to the runtime, so quvyta opens the way every Quvyta app looks.
    pub fn new(machine: Machine) -> Self {
        // Before anything is read, so the shared switch already says what quvyta's old one did.
        if let (Some(path), Some(folder)) = (machine.launcher_conf.as_deref(), machine.settings_dir.as_deref()) {
            // A file that cannot be written keeps its line, and the next start tries again.
            let _ = crate::launcher::hand_over_check_updates(path, folder);
        }
        let settings = crate::launcher::open(machine.launcher_conf.as_deref());
        let i18n = crate::cli::i18n(|name| std::env::var(name).ok());
        let setup = setup(&machine, &i18n);
        // With the wizard open nothing may be written yet, not even the shared file, so the
        // preferences are the ones it resolved without saving.
        let preferences = match &setup {
            Some(setup) => setup.preferences().clone(),
            None => match machine.settings_dir.as_deref() {
                Some(folder) => Family::QUVYTA.preferences_in(folder, LAUNCHER, &i18n),
                None => Family::QUVYTA.preferences(LAUNCHER, &i18n),
            },
        };
        let appearance = appearance_of(&machine, preferences);
        let arch = crate::checks::distro(&machine) == crate::checks::Distro::Arch;
        let font = settings::Font::new(&machine, font_install(&machine));
        Self {
            setup,
            font,
            picked: [false; APPS.len()],
            arch,
            machine,
            inventory: None,
            selected: 0,
            size: Size::default(),
            detail_page: false,
            whats_new: false,
            launcher: Launcher::default(),
            settings,
            appearance,
            installs: Installs::default(),
            path_notice: None,
            updates: Updates::default(),
            asked: VecDeque::new(),
            tab: Tab::default(),
            following: None,
            following_selected: None,
            unreadable_told: Vec::new(),
        }
    }

    /// Opens on the install dialogs of `members`, indexes of [`APPS`], one after another, as
    /// soon as it is known which of them are installed. A member not released yet is left out:
    /// it has no dialog, and it is not there already either.
    #[must_use]
    pub fn asking(mut self, members: Vec<usize>) -> Self {
        self.asked = members.into_iter().filter(|index| APPS.get(*index).is_some_and(Member::published)).collect();
        self
    }

    /// Opens on the page of the member at `index` of [`APPS`]: selected in the list with its
    /// details beside it, or on a narrow screen its details alone, as a first `enter` shows them,
    /// with `esc` back to the list. The size is not known yet, so the page is chosen for both.
    #[must_use]
    pub fn showing(mut self, index: usize) -> Self {
        if index < APPS.len() {
            self.selected = index;
            self.detail_page = true;
        }
        self
    }

    /// The shared language, theme and icons as quvyta resolved them for itself, for the
    /// runtime to start with.
    #[must_use]
    pub fn preferences(&self) -> &Preferences {
        self.appearance.preferences()
    }

    /// quvyta's own `launcher.conf` as the framework reads it, for the runtime to start with what
    /// it says about reduced motion and the pillar.
    #[must_use]
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Opens the dialog of the next member asked for that can be installed. The ones that are
    /// there already are not installed again: the first of them is shown with its details,
    /// where its version and any update are, and a toast names them all. One that does not run
    /// on this machine is neither of those: it is refused at once, with the reason the keyboard
    /// gives.
    fn ask_next(&mut self) -> Command<Msg> {
        let mut there = Vec::new();
        let mut nowhere = Vec::new();
        while let Some(index) = self.asked.pop_front() {
            if self.installable(index) {
                // The dialog belongs to the list, which shows what it starts.
                self.tab = Tab::Apps;
                self.selected = index;
                return Command::batch([
                    already_installed(&there),
                    self.refused(&nowhere),
                    self.install_update(InstallMsg::Ask(index)),
                ]);
            }
            // Not installable is two different things: it is there already, or it does not run on
            // this machine at all. Only the second one is a refusal, and it is said at once.
            if self.not_here(index).is_some() {
                nowhere.push(index);
            } else {
                there.push(index);
            }
        }
        let refused = self.refused(&nowhere);
        if let Some(first) = there.first().copied()
            && self.installs.dialog.is_none()
        {
            let shown = self.update(if self.wide() { Msg::Select(first) } else { Msg::ShowDetail(first) });
            return Command::batch([already_installed(&there), refused, shown]);
        }
        Command::batch([already_installed(&there), refused])
    }

    fn wide(&self) -> bool {
        self.size.width >= WIDE.max(self.list_width(true) + self.detail_need())
    }

    /// The columns the details need so nothing in them is cut, with the air around them.
    ///
    /// It is the widest any Quvyta app asks for and not the chosen member's own need: a need
    /// that changed with the selection would move the split while walking down the list.
    ///
    /// `wide` asks for this from `update` and `action` as well as from the view, where the
    /// environment and with it the icon set are out of reach, so it is measured from the
    /// language files alone and the badge's dot counts as one cell, the way `list_width` counts
    /// a row's icon.
    fn detail_need(&self) -> u16 {
        let widest = (0..APPS.len()).map(|index| self.detail_content(index)).max().unwrap_or(0);
        // The row under a running install counts whether or not one is running: were it counted
        // only while cargo works, the split would move the moment somebody pressed Install. The
        // longer rows of a failure and of the queue are not here; they lay themselves out down
        // the screen instead, since holding the split shut until they fit would cost every
        // language a third of the screen.
        DETAIL_MIN.max(widest.max(install_view::running_row_width()) + 4)
    }

    /// The widest line the details of the member at `index` cannot break: its buttons in a row,
    /// its title, or its badge. The title and the badge stand alone because `detail::show` puts
    /// the badge on its own line when they do not fit beside each other.
    fn detail_content(&self, index: usize) -> u16 {
        let member = &APPS[index];
        let title = qframe::text::width(&t!(&format!("apps.{}.title", member.key)));
        let badge = detail::badge_width(&match member.status() {
            Status::Released => t!("status.released"),
            Status::Beta => t!("status.beta"),
            Status::Alpha => t!("status.alpha"),
            Status::Soon => t!("status.soon"),
        });
        self.buttons_width(index).max(title).max(badge)
    }

    /// The columns the buttons of the member at `index` take, as `main_action` lays them out:
    /// each label with the button's air around it, and two cells between neighbours. The spacer
    /// that holds Remove apart is a neighbour of its own, so it costs two cells even when it is
    /// squeezed to nothing.
    fn buttons_width(&self, index: usize) -> u16 {
        let button = |label: String| qframe::text::width(&label) + 4;
        let mut row = Vec::new();
        if self.opening(index).is_some() {
            row.push(button(t!("detail.open")));
            if self.updatable(index) {
                row.push(button(t!("detail.update")));
            }
            if self.removable(index) {
                row.push(0);
                row.push(button(t!("detail.remove")));
            }
        } else if self.updatable(index) {
            row.push(button(t!("detail.update")));
        } else if self.installable(index) {
            row.push(button(t!("detail.install")));
        }
        let gaps = 2 * u16::try_from(row.len().saturating_sub(1)).unwrap_or(0);
        row.iter().sum::<u16>() + gaps
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

    /// Why the member at `index` cannot be installed from this machine, as the screen names it;
    /// `None` when quvyta may offer to install it here.
    fn not_here(&self, index: usize) -> Option<NotHere> {
        APPS.get(index).and_then(|member| member.not_here(self.arch))
    }

    /// The one line that tells the member at `index` cannot be installed from here, naming it:
    /// what the keyboard and the command line are both given. `None` when there is no reason, so
    /// an answer that arrives for a member that turned out to be here changes nothing.
    fn refusal(&self, index: usize) -> Option<String> {
        let member = APPS.get(index)?;
        Some(match self.not_here(index)? {
            NotHere::ArchOnly => t!("platform.arch-only-toast", command = member.command),
            NotHere::UnixOnly => t!("platform.unix-only-toast", command = member.command),
        })
    }

    /// The one toast that tells every member of `indexes` cannot be installed from this machine,
    /// each with its reason: the command line refuses the way the keyboard does, rather than
    /// leaving the person to read a list that quietly does not offer what they asked for.
    fn refused(&self, indexes: &[usize]) -> Command<Msg> {
        let lines: Vec<String> = indexes.iter().filter_map(|index| self.refusal(*index)).collect();
        if lines.is_empty() {
            return Command::none();
        }
        Command::toast(Toast::warning(lines.join("\n")))
    }

    /// Whether quvyta should step aside for good once a member it opened closes.
    fn leaves_with_member(&self) -> bool {
        self.launcher.after_close == AfterClose::Shell
    }

    /// Tells what was wrong in `launcher.conf`, once, as it starts.
    fn settings_problems(&self) -> Command<Msg> {
        // A broken shared file is told here too: it is read at start like quvyta's own, and its
        // keys decide how quvyta looks.
        let shared = self.appearance.preferences().diagnostics();
        if self.launcher.diagnostics.is_empty() && shared.is_empty() {
            return Command::none();
        }
        // The diagnostics name the file only; the full path says where to find it.
        let path = self.machine.launcher_conf.as_deref().map(|path| self.machine.show(path));
        let lines: Vec<String> = path
            .into_iter()
            .chain(self.launcher.diagnostics.iter().map(ToString::to_string))
            .chain(shared.iter().map(ToString::to_string))
            .collect();
        Command::toast(Toast::warning(t!("launcher.problems")).body(lines.join("\n")))
    }

    /// Opens the page of what changed in the running version, on the first start of a version
    /// the data folder does not hold, and keeps that version so it opens once. The wizard has the
    /// screen on a first start, and a changelog is no way to welcome someone.
    fn offer_whats_new(&mut self) -> Command<Msg> {
        if self.setting_up() {
            // Nothing to say if this cannot be kept: the page then opens once after the wizard,
            // with its own quiet note that it cannot remember.
            whats_new::first_start(&self.machine).ok();
            return Command::none();
        }
        let Some(kept) = whats_new::offer(&self.machine) else { return Command::none() };
        self.whats_new = true;
        // A version that cannot be kept opens the page at the next start as well; saying so
        // quietly is kinder than a page that keeps coming with no reason given.
        if kept.is_err() {
            return Command::toast(Toast::warning(t!("whats-new.forgotten")));
        }
        Command::none()
    }
}

impl App for Quvyta {
    type Msg = Msg;

    fn init(&mut self) -> Command<Msg> {
        self.launcher = Launcher::from_settings(&self.settings);
        // The shared update notice: turned off in any member, nothing asks crates.io unasked;
        // `r` still does, since that is someone asking.
        let updates = if self.preferences().update_notice() { self.check_updates(false) } else { Command::none() };
        // On the first start the wizard has the screen, so the appearance rows take the keys.
        let first = if self.setting_up() { "setup-appearance" } else { "apps" };
        Command::batch([
            Command::focus(first),
            self.read_inventory(),
            self.settings_problems(),
            self.clear_leftover(),
            updates,
            self.offer_whats_new(),
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
        // While the wizard asks, the app list's keys have nothing to act on: its list is not there
        // and nothing may be installed before Finish.
        if self.setting_up() {
            return None;
        }
        // The page of what changed has one way out, the Back button; the list it is over is not
        // there to act on.
        if self.whats_new {
            return (name == "back").then_some(Msg::Back);
        }
        // The app list's keys act on the list; on the Settings tab they would act on a member
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
            Msg::Select(index) if index < APPS.len() => self.selected = index,
            Msg::ShowDetail(index) if index < APPS.len() => {
                self.selected = index;
                self.detail_page = true;
            }
            Msg::Select(_) | Msg::ShowDetail(_) => {}
            // The page of what changed is over the list and closes before anything else: while it
            // is there, `esc` means leaving it and nothing else.
            Msg::Back if self.whats_new => {
                self.whats_new = false;
                return Command::focus("apps");
            }
            // From the settings esc goes back to the app list as it was left, a member's page
            // included; the keys go to the list when it is on screen.
            Msg::Back if self.tab == Tab::Settings => {
                self.tab = Tab::Apps;
                if self.wide() || !self.detail_page {
                    return Command::focus("apps");
                }
            }
            Msg::Back => {
                self.detail_page = false;
                return Command::focus("apps");
            }
            Msg::Inventory(inventory) => {
                let first = self.inventory.replace(inventory).is_none();
                // What is installed decides which members the follow table lists.
                let following = self.read_following();
                if first {
                    return Command::batch([self.check_path_at_start(), self.ask_next(), following]);
                }
                return following;
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
            // There is nothing to open, so nothing to answer an install with either: the reason
            // is told and the list is left as it was.
            Msg::Primary if self.opening(self.selected).is_none() => {
                return self.update(Msg::NotInstallable(self.selected));
            }
            Msg::Primary => return self.update(Msg::Open(self.selected)),
            Msg::Open(index) => {
                if let Some(opening) = self.opening(index) {
                    return opening.handoff(index, &APPS[index]);
                }
            }
            Msg::Closed { index, outcome } => {
                let Some(member) = APPS.get(index) else { return Command::none() };
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
            // What a member that does not run here is told, whether the keyboard or the command
            // line asked for it. The list keeps the member, and offers nothing.
            Msg::NotInstallable(index) => {
                if let Some(refusal) = self.refusal(index) {
                    return Command::toast(Toast::warning(refusal));
                }
            }
            Msg::Path(msg) => return self.update_path(msg),
            Msg::Updates(msg) => return self.update_update(msg),
            // The header's keys and a click leave the keys on the tabs, where they were.
            Msg::Tab(tab) => {
                self.tab = tab;
                // A member opened since the last look may have written its file.
                if tab == Tab::Settings {
                    return self.read_following();
                }
            }
            Msg::Setting(msg) => return self.update_setting(msg),
            // The framework owns its step: it applies the change, writes the two files when the
            // wizard finishes, and answers with `Msg::SetUp`.
            Msg::Setup(msg) => {
                if let Some(mut setup) = self.setup.take() {
                    let done = setup.update(msg, &mut self.settings);
                    self.setup = Some(setup);
                    return done;
                }
            }
            Msg::Wizard(msg) => return self.update_wizard(msg),
            Msg::SetUp => return self.finish_setup(),
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        // The first start asks before it shows the app list: the wizard has the screen to itself.
        if self.setting_up() {
            self.setup_wizard(ui);
            return;
        }
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
        // The page of what changed is over the list, which is still there, whole, behind it. A
        // version whose entry the changelog that ships does not have is not shown at all.
        if self.whats_new
            && let Some(entry) = whats_new::entry()
        {
            self.whats_new_page(entry, ui);
            return;
        }
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

    fn list(&self, ui: &mut View<'_, Msg>) {
        let list = List::new(self.list_items(self.compact())).selected(Some(self.selected)).on_select(Msg::Select);
        // On a narrow screen a row opens the details; on a wide one they are already beside it.
        let list = if self.wide() { list } else { list.on_activate(Msg::ShowDetail) };
        ui.add(list).fill().id("apps");
    }

    /// The rows of the list, as [`List`] takes them, `compact` or not.
    fn list_items(&self, compact: bool) -> Vec<ListItem> {
        APPS.iter()
            .enumerate()
            .map(|(index, member)| {
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
            })
            .collect()
    }

    /// The width the list needs for its widest row, `compact` or not, as the list measures it:
    /// the pillar and the icon with their air, the label, the state with its air and a margin.
    fn list_width(&self, compact: bool) -> u16 {
        let row = |index: usize| {
            let state = self.row_text(index, compact).map_or(0, |text| qframe::text::width(&text) + 2);
            qframe::text::width(APPS[index].command) + state + 7
        };
        (0..APPS.len()).map(row).max().unwrap_or(0)
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
                (None, state) => row_state(&APPS[index], state, compact, self.not_here(index)),
            })
        })
    }

    /// The columns the list may take: beside the details, or the whole screen but a margin.
    fn list_room(&self) -> u16 {
        self.size.width.saturating_sub(if self.wide() { self.detail_need() } else { 1 })
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
        let member = &APPS[self.selected];
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
                let room = self.detail_width().saturating_sub(4);
                detail::show(member, self.state(self.selected), latest.as_deref(), main, &self.machine, room, ui);
            })
            .gap(1)
            .padding(Padding { top: 0, right: 2, bottom: 1, left: 2 })
            .fill_width()
            .id(member.key);
        })
        .fill();
    }

    /// What changed in the running version: the title, the entry as the changelog that ships
    /// writes it, and the way back. Drawn as `detail` draws the details of a member, over a
    /// surface that answers the pointer like the key does, since a click is `esc` here too.
    fn whats_new_page(&self, entry: &str, ui: &mut View<'_, Msg>) {
        ui.add_with(ScrollView::new(), |ui| {
            ui.add_with(Panel::new().selected(true).on_press(Msg::Back), |ui| {
                ui.column(|ui| {
                    ui.add(Button::new(t!("nav.back")).icon("arrow-left").on_press(Msg::Back)).id("back");
                    ui.add(
                        Text::new(t!("whats-new.title", version = env!("CARGO_PKG_VERSION"))).role("title").no_wrap(),
                    );
                    // The entry as it is written: the framework's document draws the marks it has.
                    ui.add(Markdown::new(entry));
                })
                .gap(1)
                .fill_width();
            })
            .fill_width();
        })
        .fill();
    }

    /// The hint bar. The key of every hint is the one the keymap binds to the action, and what the
    /// key does is said in the label, which changes with the screen: both come from
    /// [`KeyHints::action_labelled`], so the bar cannot name a key the keys no longer do. The
    /// framework's own actions keep both from the keymap. What enter does is read first and goes
    /// last when the bar narrows ([`KeyHints::action_labelled_first`]): it is the one thing on the
    /// screen the arrows cannot say.
    fn footer(&self, ui: &mut View<'_, Msg>) {
        let hints = if self.whats_new {
            // The page of what changed has one way out, so that is the only key the bar names.
            KeyHints::new().action_labelled(Scope::App, "back", t!("hints.back"))
        } else if self.tab == Tab::Settings {
            KeyHints::new()
                .hint("↑↓", t!("hints.choose"))
                .action_labelled(Scope::App, "back", t!("hints.apps"))
                .action(Scope::Global, "focus-next")
        } else if self.wide() || self.detail_page {
            let hints = KeyHints::new();
            let hints = if self.installable(self.selected) {
                hints.action_labelled_first(Scope::App, "primary", t!("hints.install"))
            } else if self.opening(self.selected).is_some() {
                hints.action_labelled_first(Scope::App, "primary", t!("hints.open"))
            } else if self.updatable(self.selected) {
                hints.action_labelled_first(Scope::App, "primary", t!("hints.update"))
            } else {
                hints
            };
            // Where enter updates already, `u` would say it twice.
            let hints = if self.updatable(self.selected) && self.opening(self.selected).is_some() {
                hints.action_labelled_first(Scope::App, "update", t!("hints.update"))
            } else {
                hints
            };
            let hints = if self.wide() {
                hints.hint("↑↓", t!("hints.choose"))
            } else {
                hints.action_labelled(Scope::App, "back", t!("hints.back"))
            };
            hints.action(Scope::Global, "focus-next").action_labelled(Scope::App, "refresh", t!("hints.refresh"))
        } else {
            KeyHints::new()
                .action_labelled_first(Scope::App, "primary", t!("hints.details"))
                .hint("↑↓", t!("hints.choose"))
                .action_labelled(Scope::App, "refresh", t!("hints.refresh"))
        };
        ui.add(hints.action_right(Scope::Global, "quit")).fill_width();
    }
}

/// The columns the tabs of the header take, measured by the framework's own strip, so the name
/// and the tagline are kept clear of the strip it draws, whatever air it gives each label.
fn tabs_width(env: &qframe::env::Env, labels: &[String]) -> u16 {
    qframe::widget::natural_size(&Tabs::<Msg>::new(labels.to_vec()), env, Size::MAX).width
}

/// What a list row says about how its member is installed: in words, never in colour alone.
/// Says which of the members at `indexes` were asked for but are installed already.
fn already_installed(indexes: &[usize]) -> Command<Msg> {
    if indexes.is_empty() {
        return Command::none();
    }
    let commands: Vec<&str> = indexes.iter().map(|index| APPS[*index].command).collect();
    Command::toast(Toast::info(t!("cli.already-installed", commands = commands.join(", "))))
}

/// `compact` leaves out quvyta's own version, which its details show anyway.
fn row_state(member: &Member, state: &State, compact: bool, not_here: Option<NotHere>) -> String {
    match state {
        // Not there for a reason of its own, so never "not installed": that invites the install.
        // A member that is not out yet keeps its own line, and one that does not run here says
        // which platform it is for.
        State::Missing => match not_here {
            Some(reason) => quiet(reason),
            None if member.status() == Status::Soon => t!("row.soon"),
            None => t!("row.missing"),
        },
        State::Cargo { version, .. } => version.clone(),
        State::Elsewhere { .. } => t!("row.unknown"),
        State::This { .. } if compact => t!("row.this-short"),
        State::This { version, .. } => t!("row.this", version = *version),
    }
}

/// Why a member cannot be installed from here, in the few words a list row has room for. The
/// first run says the same claim in a whole sentence; the page and the row both say this.
pub(in crate::app) fn quiet(reason: NotHere) -> String {
    match reason {
        NotHere::ArchOnly => t!("platform.arch-only"),
        NotHere::UnixOnly => t!("platform.unix-only"),
    }
}

impl Quvyta {
    /// The name, the tagline when there is room for it whole, and the tabs at the right.
    fn header(&self, ui: &mut View<'_, Msg>) {
        let labels = [t!("tabs.apps"), t!("tabs.settings")];
        let tabs_width = tabs_width(ui.env(), &labels);
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
mod alpha_tests;
#[cfg(test)]
mod cli_tests;
#[cfg(test)]
mod framework_tests;
#[cfg(test)]
mod install_tests;
#[cfg(test)]
mod platform_tests;
#[cfg(test)]
mod remove_tests;
#[cfg(test)]
mod soon_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod update_all_tests;
#[cfg(test)]
mod update_tests;
#[cfg(test)]
mod whats_new_tests;
