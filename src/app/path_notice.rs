//! The PATH notice: when the programs cargo installs cannot be started by name in a terminal,
//! quvyta shows the start-up file and the line that fixes it, and adds the line on request.
//!
//! The check runs at start once the inventory shows something cargo installed, and after every
//! successful install. It reads start-up files, and adding writes one, so both run off the
//! drawing thread.

use qframe::prelude::*;
use qframe::widgets::{CopyValue, Toast};

use super::{Msg, Quvyta};
use crate::ecosystem::APPS;
use crate::inventory::State;
use crate::launcher::{Launcher, PathPrompt};
use crate::machine::Machine;
use crate::shell_path::{self, PathAction};

/// What the notice hears.
#[derive(Debug, Clone)]
pub enum PathMsg {
    /// What the check of `PATH` after an install, or at start, decided in the background.
    Checked {
        /// The member just installed, an index of [`APPS`]; `None` at start.
        installed: Option<usize>,
        /// What it takes.
        action: PathAction,
        /// Whether the user asked for the notice, from the Settings tab: then it shows even
        /// after Not now, which only silences the offers quvyta makes on its own.
        asked: bool,
    },
    /// Appends the line to the start-up file.
    Add,
    /// The line was appended, or why it was not: the path as shown, and the reason.
    Added(Result<(), String>),
    /// Hides the offer and remembers not to make it again.
    NotNow,
    /// `launcher.conf` remembered the answer, or why it could not.
    Dismissed(Result<(), String>),
}

/// How far cargo's folder is from `PATH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PathReach {
    /// It is on this terminal's `PATH`.
    Here,
    /// A start-up file adds it, so terminals opened from now on have it.
    NewTerminals,
    /// Nothing adds it, as far as quvyta knows.
    Missing,
}

/// The notice shown in the detail area, or under the settings.
#[derive(Debug, Clone)]
pub(super) struct PathNotice {
    /// The member the text names; `None` for the general wording.
    command: Option<&'static str>,
    action: PathAction,
    stage: Stage,
}

/// How far the offer got.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Stage {
    /// Waiting for Add or Not now.
    Offered,
    /// The line is being written.
    Adding,
    /// The line is in the file.
    Added,
    /// Writing failed, for this reason.
    Failed(String),
}

impl Quvyta {
    /// Checks whether members can be started by name and shows the notice when they cannot.
    ///
    /// This is the entry point for installing: after a successful install of the member at
    /// `index`, `update` returns `self.check_path(Some(index))`, and the notice names that
    /// member. At start it runs with `None`, and the wording is general.
    pub(super) fn check_path(&self, installed: Option<usize>) -> Command<Msg> {
        self.decide_path(installed, false)
    }

    /// The check the Settings tab's Add asks for: the same notice, shown even after Not now.
    pub(super) fn ask_path(&self) -> Command<Msg> {
        self.decide_path(None, true)
    }

    fn decide_path(&self, installed: Option<usize>, asked: bool) -> Command<Msg> {
        let machine = self.machine.clone();
        Command::perform(move || {
            let action = shell_path::decide(&machine.shell_env(), shell_path::read_file);
            Msg::Path(PathMsg::Checked { installed, action, asked })
        })
    }

    /// How far cargo's folder is from `PATH`, for the Settings tab.
    pub(super) fn path_reach(&self) -> PathReach {
        if self.machine.path.contains(&self.machine.cargo_bin()) {
            return PathReach::Here;
        }
        match &self.path_notice {
            Some(notice)
                if matches!(notice.action, PathAction::AlreadyConfigured { .. }) || notice.stage == Stage::Added =>
            {
                PathReach::NewTerminals
            }
            _ => PathReach::Missing,
        }
    }

    /// The check at start: only when something is installed by cargo, quvyta itself included,
    /// since only then is there a program the user may want to type by name.
    pub(super) fn check_path_at_start(&self) -> Command<Msg> {
        let cargo_bin = self.machine.cargo_bin();
        let by_cargo = APPS.iter().enumerate().any(|(index, member)| match self.state(index) {
            Some(State::Cargo { .. }) => true,
            // quvyta's own row says it is running, not where from; its file in cargo's folder
            // says cargo put it there.
            Some(State::This { .. }) => cargo_bin.join(member.command).is_file(),
            _ => false,
        });
        if by_cargo { self.check_path(None) } else { Command::none() }
    }

    pub(super) fn update_path(&mut self, msg: PathMsg) -> Command<Msg> {
        match msg {
            PathMsg::Checked { installed, action, asked } => {
                let asks = matches!(action, PathAction::Add { .. } | PathAction::Unknown { .. });
                // Asked for, the notice takes the keys, so its Add is one enter away.
                let focus = if asked && matches!(action, PathAction::Add { .. }) {
                    Command::focus("path-add")
                } else {
                    Command::none()
                };
                self.path_notice = match action {
                    PathAction::OnPath => None,
                    // Not now silences the offer; saying that a new terminal already works is not
                    // an offer, so it still shows.
                    _ if asks && !asked && self.launcher.path_prompt == PathPrompt::Dismissed => None,
                    action => Some(PathNotice {
                        command: installed.and_then(|index| APPS.get(index)).map(|member| member.command),
                        action,
                        stage: Stage::Offered,
                    }),
                };
                return focus;
            }
            PathMsg::Add => {
                let Some(notice) = self.path_notice.as_mut() else { return Command::none() };
                let PathAction::Add { file, line, .. } = &notice.action else { return Command::none() };
                if matches!(notice.stage, Stage::Adding | Stage::Added) {
                    return Command::none();
                }
                notice.stage = Stage::Adding;
                let (file, line, home) = (file.clone(), line.clone(), self.machine.home.clone());
                return Command::perform(move || {
                    let result = shell_path::apply(&file, &line)
                        .map_err(|error| format!("{}: {}", shell_path::shown(&error.path, &home), error.error));
                    Msg::Path(PathMsg::Added(result))
                });
            }
            PathMsg::Added(result) => {
                if let Some(notice) = self.path_notice.as_mut() {
                    notice.stage = match result {
                        Ok(()) => Stage::Added,
                        Err(reason) => Stage::Failed(reason),
                    };
                }
            }
            PathMsg::NotNow => {
                self.path_notice = None;
                self.launcher.path_prompt = PathPrompt::Dismissed;
                let Some(path) = self.machine.launcher_conf.clone() else { return Command::none() };
                return Command::perform(move || {
                    let saved = Launcher::dismiss_path_prompt(&path).map_err(|error| error.to_string());
                    Msg::Path(PathMsg::Dismissed(saved))
                });
            }
            PathMsg::Dismissed(Ok(())) => {}
            PathMsg::Dismissed(Err(reason)) => {
                let path = self.machine.launcher_conf.as_deref().map(|path| self.machine.show(path));
                let body = path.map_or(reason.clone(), |path| format!("{path}\n{reason}"));
                return Command::toast(Toast::warning(t!("path.not-remembered")).body(body));
            }
        }
        Command::none()
    }

    /// Draws the notice, when there is one, at the top of the detail area.
    pub(super) fn show_path_notice(&self, ui: &mut View<'_, Msg>) {
        if let Some(notice) = &self.path_notice {
            show(notice, &self.machine, ui);
        }
    }
}

/// A text of the notice: the one naming `command`, or its general form under `key-any`.
fn say(key: &str, command: Option<&str>) -> String {
    match command {
        Some(command) => t!(&format!("path.{key}"), command = command),
        None => t!(&format!("path.{key}-any")),
    }
}

fn show(notice: &PathNotice, machine: &Machine, ui: &mut View<'_, Msg>) {
    let command = notice.command;
    let (file, line) = match &notice.action {
        PathAction::OnPath => return,
        PathAction::AlreadyConfigured { .. } => {
            ui.add(Text::new(say("ready", command)).role("secondary")).fill_width();
            return;
        }
        PathAction::Add { file, line, .. } => (Some(file), line),
        PathAction::Unknown { line } => (None, line),
    };
    if notice.stage == Stage::Added {
        ui.column(|ui| {
            ui.add(Text::new(say("added", command)).color("success")).fill_width();
            ui.add(Text::new(t!("path.open-terminals")).role("secondary")).fill_width();
        })
        .fill_width();
        return;
    }

    let folder = shell_path::shown(&machine.shell_env().bin_dir(), &machine.home);
    let panel = Panel::new().title(t!("path.title", folder = folder)).selected(true);
    ui.add_with(panel, |ui| {
        let why = if file.is_some() { "why" } else { "unknown" };
        ui.add(Text::new(say(why, command))).fill_width();
        ui.column(|ui| {
            if let Some(file) = file {
                ui.add(Text::rich([
                    Span::new(format!("{}  ", t!("path.file"))).role("faint"),
                    Span::new(shell_path::shown(file, &machine.home)),
                ]))
                .fill_width();
            }
            ui.add(Text::new(t!("path.line")).role("faint").no_wrap());
            ui.add(CopyValue::new(line.as_str())).id("path-line");
        })
        .fill_width();
        if let Stage::Failed(reason) = &notice.stage {
            ui.column(|ui| {
                ui.add(Text::new(t!("path.failed")).color("danger")).fill_width();
                ui.add(Text::new(reason.as_str()).role("secondary")).fill_width();
            })
            .fill_width();
        }
        // After a failed write the line stays to copy; trying the same write again would fail the
        // same way.
        let offers_add = file.is_some() && matches!(notice.stage, Stage::Offered | Stage::Adding);
        ui.row(|ui| {
            ui.add(Button::new(t!("path.not-now")).on_press(Msg::Path(PathMsg::NotNow))).id("path-not-now");
            if offers_add {
                let add = Button::new(t!("path.add")).variant("primary").loading(notice.stage == Stage::Adding);
                ui.add(add.on_press(Msg::Path(PathMsg::Add))).id("path-add");
            }
        })
        .gap(2)
        .justify(Align::End)
        .fill_width();
    })
    .fill_width();
}

#[cfg(test)]
pub(super) mod tests;
