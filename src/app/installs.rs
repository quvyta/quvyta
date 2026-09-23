//! The install queue as the screen keeps it: the dialog that asks first, the member being
//! installed with its progress and log, the ones waiting, and how the last install of each
//! member ended.
//!
//! One install runs at a time: two builds at once would take twice the machine and wait for
//! each other on cargo's own lock anyway, and a queue tells the user honestly what happens next.
//! Removals wait in the same queue: `cargo uninstall` rewrites the same record of installed
//! packages an install writes last, so one cargo job at a time keeps the two from racing.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::time::Duration;

use qframe::prelude::*;
use qframe::runtime::{Handoff, HandoffOutcome, Task, TaskEvent, TaskId, TaskOutcome};
use qframe::storage::AppLock;
use qframe::widgets::{LogBuffer, LogLevel, LogLine, Toast};

use super::{Msg, Quvyta};
use crate::cargo::{self, Step};
use crate::checks::{self, Problem};
use crate::ecosystem::APPS;
use crate::install::{self, Claim, Job, Outcome, Removal};
use crate::inventory::State;

/// Lines of cargo's output kept for the details and the copied log; far more than an install
/// of any member writes.
const LOG_LINES: usize = 20_000;

/// How often a window waiting for another one looks whether the build folder is free.
const WAIT: Duration = Duration::from_secs(3);

/// Everything that happens to installs.
#[derive(Debug, Clone)]
pub enum InstallMsg {
    /// Opens the install dialog for the member at this index.
    Ask(usize),
    /// What the checks found for the member of the dialog, and whether another window installs.
    Checked {
        /// The member.
        index: usize,
        /// What is in the way; empty when the install can go ahead.
        problems: Vec<Problem>,
        /// Whether another quvyta window holds the build folder.
        other_window: bool,
    },
    /// Runs the checks of the open dialog again.
    Recheck,
    /// Closes the dialog without installing.
    Close,
    /// Puts the member of the dialog in the queue.
    Confirm,
    /// Hands the terminal to rustup to bring Rust up to date.
    UpdateRust(PathBuf),
    /// rustup is done.
    RustUpdated(HandoffOutcome),
    /// Hands the terminal to the line that puts this problem right, which the person has read
    /// beside the button they pressed.
    RunFix(Problem),
    /// That line has ended.
    FixRan(HandoffOutcome),
    /// A line cargo wrote while installing the member at this index.
    Line(usize, String),
    /// A frame of cargo's progress line, which it redraws in place, while installing the member
    /// at this index. It says where the build is and belongs in no log.
    Frame(usize, String),
    /// The install of the member at this index ended.
    Finished {
        /// The member.
        index: usize,
        /// How it ended.
        outcome: Outcome,
        /// What the checks find after a failure they explain, for its fix.
        problems: Vec<Problem>,
    },
    /// The task of the member at this index started, progressed or ended.
    Event(usize, TaskEvent),
    /// Asks whether to stop the running install.
    AskStop,
    /// Whether stopping also empties the queue.
    StopQueued(bool),
    /// Keeps the install running.
    KeepRunning,
    /// Stops the running install, and the queue too when that was chosen.
    Stop,
    /// Takes the member at this index out of the queue.
    Dequeue(usize),
    /// Asks whether to quit while an install runs.
    AskQuit,
    /// Stops everything and quits.
    Quit,
    /// Shows or hides cargo's own output.
    ToggleDetails,
    /// Copies the whole log of the member at this index.
    CopyLog(usize),
    /// Clears the failure of the member at this index.
    Dismiss(usize),
    /// Tries the member at this index again.
    Retry(usize),
    /// Asks whether to remove the member at this index.
    AskRemove(usize),
    /// Closes the remove question without removing.
    KeepMember,
    /// Puts the removal of the member of the question in the queue.
    ConfirmRemove,
    /// Whether another window held the build folder when quvyta started.
    Leftover(bool),
    /// The build folder another window held is free.
    BuildFree,
    /// The build folder of an ended queue is deleted and released.
    Released,
}

/// The install state of the screen.
#[derive(Debug, Default)]
pub(super) struct Installs {
    /// The dialog, while it is open.
    pub(super) dialog: Option<Dialog>,
    /// Members waiting, in order, with what is to be done to each.
    pub(super) queue: VecDeque<(usize, Action)>,
    /// The member the remove question is about, while it is open.
    pub(super) remove: Option<usize>,
    /// The install running now.
    pub(super) running: Option<Running>,
    /// How the last install of a member ended, while that is worth showing.
    pub(super) ended: BTreeMap<usize, Ended>,
    /// The build folder, held while the queue has work.
    lock: Option<AppLock>,
    /// Whether another quvyta window holds the build folder.
    pub(super) other_window: bool,
    /// The task that waits for the other window.
    waiting: Option<TaskId>,
    /// While the stop question is open: whether the queue stops too.
    pub(super) stop: Option<bool>,
    /// Quitting once the running install has stopped.
    quitting: bool,
    /// Whether cargo's own output is shown.
    pub(super) details: bool,
}

/// The install dialog of one member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Dialog {
    /// The member, an index of [`APPS`].
    pub(super) index: usize,
    /// The version to install; `None` for the latest on crates.io.
    pub(super) version: Option<String>,
    /// The version installed now, when the dialog asks to update it.
    pub(super) from: Option<String>,
    /// What the checks found; `None` while they run.
    pub(super) problems: Option<Vec<Problem>>,
}

/// What a job of the queue does to its member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Action {
    /// `cargo install`, at this version, or the latest one when `None`. An update is an install
    /// of the newer version: cargo replaces the one there.
    Install(Option<String>),
    /// `cargo uninstall`.
    Remove,
}

/// The install that runs.
#[derive(Debug)]
pub(super) struct Running {
    /// The member, an index of [`APPS`].
    pub(super) index: usize,
    /// What is done to it.
    pub(super) action: Action,
    task: TaskId,
    /// Where it is.
    pub(super) progress: Progress,
    /// What cargo wrote.
    pub(super) log: LogBuffer,
}

/// Where a running install is, from cargo's lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Progress {
    /// The phase.
    pub(super) phase: Phase,
    /// Units built and in the build, when cargo's progress line gave them.
    pub(super) counted: Option<(u32, u32)>,
    /// The crate being built.
    pub(super) krate: Option<String>,
}

/// The phases of an install, in order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Phase {
    /// cargo has not said anything yet.
    #[default]
    Starting,
    /// Updating the index and fetching sources.
    Downloading,
    /// Building.
    Compiling,
    /// Putting the program in place.
    Placing,
}

impl Progress {
    fn read(&mut self, line: &str) {
        match cargo::step(line) {
            Some(Step::Downloading) => self.phase = Phase::Downloading,
            Some(Step::Compiling { krate }) => {
                self.phase = Phase::Compiling;
                self.krate = Some(krate);
            }
            Some(Step::Counted { done, total, krate }) => {
                self.phase = Phase::Compiling;
                self.counted = Some((done, total));
                if !krate.is_empty() {
                    self.krate = Some(krate);
                }
            }
            Some(Step::Placing) => self.phase = Phase::Placing,
            None => {}
        }
    }

    /// The share done, when cargo counted.
    pub(super) fn fraction(&self) -> Option<f32> {
        let (done, total) = self.counted?;
        // Counts stay far below where `f32` loses whole numbers.
        #[expect(clippy::cast_precision_loss, reason = "unit counts are small")]
        Some(done as f32 / total.max(1) as f32)
    }
}

/// How the last install of a member ended.
#[derive(Debug, Clone)]
pub(super) enum Ended {
    /// It failed.
    Failed {
        /// What failed, so trying again does the same.
        action: Action,
        /// Why, as far as cargo's output tells.
        outcome: Outcome,
        /// What the checks found afterwards, for the fix.
        problems: Vec<Problem>,
        /// What cargo wrote.
        log: LogBuffer,
    },
    /// It was stopped.
    Stopped,
}

impl Installs {
    /// Whether an install runs or waits, which keeps quvyta from quitting unasked.
    pub(super) fn busy(&self) -> bool {
        self.running.is_some() || !self.queue.is_empty()
    }

    /// Whether the member at `index` runs or waits.
    pub(super) fn has(&self, index: usize) -> bool {
        self.running.as_ref().is_some_and(|running| running.index == index)
            || self.queue.iter().any(|(queued, _)| *queued == index)
    }

    /// Whether the member at `index` waits in the queue.
    pub(super) fn is_queued(&self, index: usize) -> bool {
        self.queue.iter().any(|(queued, _)| *queued == index)
    }

    /// The install of the member at `index`, when it runs.
    pub(super) fn running(&self, index: usize) -> Option<&Running> {
        self.running.as_ref().filter(|running| running.index == index)
    }
}

impl Quvyta {
    /// Whether the member at `index` can be installed from here: it is not there, it is out on
    /// crates.io, it is not quvyta itself, and it is not already on its way.
    pub(super) fn installable(&self, index: usize) -> bool {
        matches!(self.state(index), Some(State::Missing))
            && APPS.get(index).is_some_and(|member| member.published() && !member.is_self())
            && !self.installs.has(index)
    }

    /// Whether the member at `index` can be removed from here: cargo installed it, so cargo can
    /// remove it, and it is not quvyta, which does not remove itself from under its own feet.
    pub(super) fn removable(&self, index: usize) -> bool {
        matches!(self.state(index), Some(State::Cargo { .. }))
            && APPS.get(index).is_some_and(|member| !member.is_self())
            && !self.installs.has(index)
    }

    /// Whether `action` can be queued for the member at `index` as things are now.
    fn can(&self, index: usize, action: &Action) -> bool {
        match action {
            Action::Install(_) => self.installable(index) || self.updatable(index),
            Action::Remove => self.removable(index),
        }
    }

    /// Looks once, in the background, for a build folder a crashed window left behind.
    pub(super) fn clear_leftover(&self) -> Command<Msg> {
        let machine = self.machine.clone();
        Command::perform(move || Msg::Install(InstallMsg::Leftover(install::clear_leftover(&machine))))
    }

    /// Asks before quitting while an install runs or waits.
    pub(super) fn ask_before_quit(&self) -> Option<Msg> {
        self.installs.busy().then_some(Msg::Install(InstallMsg::AskQuit))
    }

    pub(super) fn install_update(&mut self, msg: InstallMsg) -> Command<Msg> {
        match msg {
            InstallMsg::Ask(index) if self.installable(index) => {
                self.installs.ended.remove(&index);
                let version = self.latest_version(index);
                self.installs.dialog = Some(Dialog { index, version, from: None, problems: None });
                return self.check();
            }
            InstallMsg::Ask(index) if self.updatable(index) => {
                self.installs.ended.remove(&index);
                let from = self.state(index).and_then(State::cargo_version).map(ToOwned::to_owned);
                self.installs.dialog = Some(Dialog { index, version: self.update_to(index), from, problems: None });
                return self.check();
            }
            InstallMsg::Ask(_) => {}
            InstallMsg::Checked { index, problems, other_window } => {
                if let Some(dialog) = self.installs.dialog.as_mut().filter(|dialog| dialog.index == index) {
                    dialog.problems = Some(problems);
                    // This window's own lock reads as taken too; only a lock it does not hold counts.
                    self.installs.other_window = other_window && self.installs.lock.is_none();
                }
            }
            InstallMsg::Recheck => {
                if let Some(dialog) = self.installs.dialog.as_mut() {
                    dialog.problems = None;
                    return self.check();
                }
            }
            InstallMsg::Close => self.installs.dialog = None,
            InstallMsg::Confirm => {
                if let Some(dialog) = self.installs.dialog.take()
                    && dialog.problems.as_ref().is_some_and(Vec::is_empty)
                    && if dialog.from.is_some() { self.updatable(dialog.index) } else { self.installable(dialog.index) }
                {
                    self.installs.queue.push_back((dialog.index, Action::Install(dialog.version)));
                    return self.start_next();
                }
            }
            InstallMsg::UpdateRust(rustup) => {
                let handoff = Handoff::new(rustup, |outcome| Msg::Install(InstallMsg::RustUpdated(outcome)))
                    .args(install::rustup_update())
                    .dir(self.machine.home.clone())
                    .notice(t!("checks.updating-rust"));
                return Command::handoff(handoff);
            }
            InstallMsg::RustUpdated(outcome) => {
                let report = match outcome {
                    HandoffOutcome::Finished { code: Some(0) } => Command::none(),
                    HandoffOutcome::Finished { .. } => Command::toast(Toast::warning(t!("checks.rustup-failed"))),
                    HandoffOutcome::Failed(reason) => {
                        Command::toast(Toast::danger(t!("checks.rustup-failed")).body(reason))
                    }
                };
                return Command::batch([report, self.install_update(InstallMsg::Recheck)]);
            }
            InstallMsg::RunFix(problem) => {
                let Some(line) = problem.fix() else { return Command::none() };
                // The very line shown, run by the shell: nothing is added to it. Whatever it asks,
                // sudo's password included, it asks on the terminal quvyta has stepped away from.
                let handoff = Handoff::new("sh", |outcome| Msg::Install(InstallMsg::FixRan(outcome)))
                    .args(["-c", line])
                    .dir(self.machine.home.clone())
                    .notice(t!("checks.running", command = line))
                    .pause(true);
                return Command::handoff(handoff);
            }
            InstallMsg::FixRan(outcome) => {
                let report = match outcome {
                    HandoffOutcome::Finished { code: Some(0) } => Command::none(),
                    HandoffOutcome::Finished { .. } => Command::toast(Toast::warning(t!("checks.fix-failed"))),
                    HandoffOutcome::Failed(reason) => {
                        Command::toast(Toast::danger(t!("checks.fix-failed")).body(reason))
                    }
                };
                // Whether it worked is for the checks to say, not the exit code: they look again.
                return Command::batch([report, self.install_update(InstallMsg::Recheck)]);
            }
            InstallMsg::Line(index, line) => {
                if let Some(running) = self.installs.running.as_mut().filter(|running| running.index == index) {
                    running.progress.read(&line);
                    running.log.push(LogLine::new(level(&line), line));
                }
            }
            InstallMsg::Frame(index, frame) => {
                if let Some(running) = self.installs.running.as_mut().filter(|running| running.index == index) {
                    running.progress.read(&frame);
                }
            }
            InstallMsg::Finished { index, outcome, problems } => return self.finished(index, outcome, problems),
            InstallMsg::Event(index, TaskEvent::Finished { outcome, .. }) => match outcome {
                TaskOutcome::Done => {}
                TaskOutcome::Cancelled => {
                    if self.installs.running.as_ref().is_some_and(|running| running.index == index) {
                        self.installs.running = None;
                        self.installs.ended.insert(index, Ended::Stopped);
                    }
                    return self.start_next();
                }
                TaskOutcome::Failed(reason) => return self.finished(index, Outcome::NotStarted(reason), Vec::new()),
            },
            InstallMsg::Event(..) => {}
            InstallMsg::AskStop if self.installs.running.is_some() => self.installs.stop = Some(false),
            InstallMsg::AskStop => {}
            InstallMsg::StopQueued(all) => {
                if self.installs.stop.is_some() {
                    self.installs.stop = Some(all);
                }
            }
            InstallMsg::KeepRunning => self.installs.stop = None,
            InstallMsg::Stop => {
                if let Some(all) = self.installs.stop.take() {
                    if all {
                        self.installs.queue.clear();
                    }
                    if let Some(running) = &self.installs.running {
                        return Command::cancel_task(running.task);
                    }
                }
            }
            InstallMsg::Dequeue(index) => {
                self.installs.queue.retain(|(queued, _)| *queued != index);
                return self.start_next();
            }
            InstallMsg::AskQuit => {
                let confirm = Confirm::new(t!("quit.title"), Msg::Install(InstallMsg::Quit))
                    .message(t!("quit.message"))
                    .confirm_label(t!("quit.confirm"))
                    .danger();
                return Command::confirm(confirm);
            }
            InstallMsg::Quit => {
                self.installs.quitting = true;
                self.installs.queue.clear();
                return match &self.installs.running {
                    // Quitting waits for cargo to be killed, or it would build on unseen.
                    Some(running) => Command::cancel_task(running.task),
                    None => self.start_next(),
                };
            }
            InstallMsg::ToggleDetails => self.installs.details = !self.installs.details,
            InstallMsg::CopyLog(index) => {
                if let Some(log) = self.log(index) {
                    let text: Vec<&str> = log.iter().map(LogLine::text).collect();
                    return Command::copy(text.join("\n"));
                }
            }
            InstallMsg::Dismiss(index) => {
                self.installs.ended.remove(&index);
            }
            InstallMsg::Retry(index) => {
                let Some(Ended::Failed { action, .. }) = self.installs.ended.remove(&index) else {
                    return Command::none();
                };
                // Again at the version known now, which the failure may have come before.
                let action = match action {
                    Action::Install(_) if self.installable(index) => Action::Install(self.latest_version(index)),
                    Action::Install(_) => Action::Install(self.update_to(index)),
                    Action::Remove => Action::Remove,
                };
                if self.can(index, &action) {
                    self.installs.queue.push_back((index, action));
                    return self.start_next();
                }
            }
            InstallMsg::AskRemove(index) if self.removable(index) => {
                self.installs.ended.remove(&index);
                self.installs.remove = Some(index);
            }
            InstallMsg::AskRemove(_) => {}
            InstallMsg::KeepMember => self.installs.remove = None,
            InstallMsg::ConfirmRemove => {
                if let Some(index) = self.installs.remove.take().filter(|index| self.removable(*index)) {
                    self.installs.queue.push_back((index, Action::Remove));
                    return self.start_next();
                }
            }
            // An install asked for before the answer came holds the folder itself.
            InstallMsg::Leftover(_) if self.installs.lock.is_some() => {}
            InstallMsg::Leftover(other_window) => {
                self.installs.other_window = other_window;
                if other_window {
                    return self.wait_for_other_window();
                }
            }
            InstallMsg::BuildFree => {
                self.installs.other_window = false;
                self.installs.waiting = None;
                return self.start_next();
            }
            InstallMsg::Released => {
                if self.installs.quitting {
                    return Command::quit();
                }
            }
        }
        Command::none()
    }

    /// Runs the checks of the open dialog in the background.
    fn check(&self) -> Command<Msg> {
        let Some(index) = self.installs.dialog.as_ref().map(|dialog| dialog.index) else { return Command::none() };
        let machine = self.machine.clone();
        let mine = self.installs.lock.is_some();
        Command::perform(move || {
            let problems = checks::run(&machine);
            let other_window = !mine && install::held_elsewhere(&machine);
            Msg::Install(InstallMsg::Checked { index, problems, other_window })
        })
    }

    /// The whole log of the member at `index`: of the install running or of its failure.
    pub(super) fn log(&self, index: usize) -> Option<&LogBuffer> {
        match (self.installs.running(index), self.installs.ended.get(&index)) {
            (Some(running), _) => Some(&running.log),
            (None, Some(Ended::Failed { log, .. })) => Some(log),
            _ => None,
        }
    }

    fn finished(&mut self, index: usize, outcome: Outcome, problems: Vec<Problem>) -> Command<Msg> {
        let Some(running) = self.installs.running.take_if(|running| running.index == index) else {
            return Command::none();
        };
        let Some(member) = APPS.get(index) else { return Command::none() };
        let report = match &outcome {
            Outcome::Installed { version } => {
                let toast = match version {
                    Some(version) => t!("install.done", command = member.command, version = version.as_str()),
                    None => t!("install.done-unknown", command = member.command),
                };
                let mut toast = Toast::success(toast).key("installed");
                // cargo replaced quvyta's file; the process running is still the old one.
                if member.is_self() {
                    toast = toast.body(t!("install.next-time"));
                }
                // Installed is not yet startable by name when cargo's folder is not on PATH.
                Command::batch([Command::toast(toast), self.read_inventory(), self.check_path(Some(index))])
            }
            Outcome::Removed => Command::batch([
                Command::toast(Toast::success(t!("remove.done", command = member.command)).key("removed")),
                self.read_inventory(),
            ]),
            Outcome::Cancelled => {
                self.installs.ended.insert(index, Ended::Stopped);
                Command::none()
            }
            Outcome::Failed(_) | Outcome::NotRemoved | Outcome::NotStarted(_) => {
                let failed = Ended::Failed { action: running.action, outcome, problems, log: running.log };
                self.installs.ended.insert(index, failed);
                Command::none()
            }
        };
        Command::batch([report, self.start_next()])
    }

    /// Starts the next member of the queue once nothing runs, taking the build folder first;
    /// deletes the folder once the queue is empty.
    pub(super) fn start_next(&mut self) -> Command<Msg> {
        if self.installs.running.is_some() || self.installs.waiting.is_some() {
            return Command::none();
        }
        let Some((index, action)) = self.installs.queue.front().cloned() else {
            return self.release();
        };
        if self.installs.lock.is_none() {
            match install::claim(&self.machine) {
                Claim::Held(lock) => self.installs.lock = Some(lock),
                Claim::Busy => {
                    self.installs.other_window = true;
                    return self.wait_for_other_window();
                }
                Claim::Unshared => {}
            }
        }
        self.installs.queue.pop_front();
        let member = &APPS[index];
        let task = match &action {
            Action::Install(version) => {
                Job::new(&self.machine, member, version.clone()).map(|job| self.install_task(index, job))
            }
            Action::Remove => Removal::new(&self.machine, member).map(|removal| remove_task(index, removal)),
        };
        let Some(task) = task else {
            // The checks found cargo; it went away since.
            let failed = Ended::Failed {
                action,
                outcome: Outcome::NotStarted(t!("checks.no-cargo")),
                problems: vec![Problem::NoCargo],
                log: LogBuffer::new(1),
            };
            self.installs.ended.insert(index, failed);
            return self.start_next();
        };
        let (task_id, log) = (task.id(), LogBuffer::new(LOG_LINES));
        self.installs.running = Some(Running { index, action, task: task_id, progress: Progress::default(), log });
        self.installs.ended.remove(&index);
        Command::task(task)
    }

    /// The background task of installing the member at `index` with `job`.
    fn install_task(&self, index: usize, job: Job) -> Task<Msg> {
        let machine = self.machine.clone();
        Task::new(APPS[index].command, move |cx| {
            // cargo redraws its progress line many times a second, and most redraws only move the
            // bar inside it. Only a frame that says something new about the build is worth a frame
            // of the screen's own.
            let mut last: Option<Step> = None;
            let outcome = job.run(
                &|| cx.is_cancelled(),
                &mut |line| cx.send(Msg::Install(InstallMsg::Line(index, line))),
                &mut |frame| {
                    let Some(step) = cargo::step(&frame) else { return };
                    if last.as_ref() != Some(&step) {
                        last = Some(step);
                        cx.send(Msg::Install(InstallMsg::Frame(index, frame)));
                    }
                },
            );
            // Only the problem behind this failure is shown with it, not everything the checks find.
            let explains = |problem: &Problem| {
                matches!(
                    (&outcome, problem),
                    (Outcome::Failed(cargo::Failure::NoLinker), Problem::NoLinker(_))
                        | (Outcome::Failed(cargo::Failure::OldRust), Problem::OldRust { .. })
                )
            };
            let problems = match outcome {
                Outcome::Failed(cargo::Failure::NoLinker | cargo::Failure::OldRust) => {
                    checks::run(&machine).into_iter().filter(explains).collect()
                }
                _ => Vec::new(),
            };
            Ok(Msg::Install(InstallMsg::Finished { index, outcome, problems }))
        })
        .on_event(move |event| Msg::Install(InstallMsg::Event(index, event)))
    }

    /// Deletes the build folder of the ended queue and lets it go, then quits if that was
    /// waiting for it.
    fn release(&mut self) -> Command<Msg> {
        match self.installs.lock.take() {
            Some(lock) => {
                let machine = self.machine.clone();
                Command::perform(move || {
                    install::release(&machine, lock);
                    Msg::Install(InstallMsg::Released)
                })
            }
            None if self.installs.quitting => Command::quit(),
            None => Command::none(),
        }
    }

    /// Looks every few seconds whether the other window let go of the build folder.
    fn wait_for_other_window(&mut self) -> Command<Msg> {
        if self.installs.waiting.is_some() {
            return Command::none();
        }
        let machine = self.machine.clone();
        let task = Task::new("wait", move |cx| {
            while cx.sleep(WAIT) {
                if !install::held_elsewhere(&machine) {
                    return Ok(Msg::Install(InstallMsg::BuildFree));
                }
            }
            Err("stopped".to_owned())
        });
        self.installs.waiting = Some(task.id());
        Command::task(task)
    }
}

/// The background task of removing the member at `index` with `removal`.
fn remove_task(index: usize, removal: Removal) -> Task<Msg> {
    Task::new(APPS[index].command, move |cx| {
        let outcome =
            removal.run(&|| cx.is_cancelled(), &mut |line| cx.send(Msg::Install(InstallMsg::Line(index, line))));
        Ok(Msg::Install(InstallMsg::Finished { index, outcome, problems: Vec::new() }))
    })
    .on_event(move |event| Msg::Install(InstallMsg::Event(index, event)))
}

/// How a line of cargo's output reads in the log: its errors and warnings stand out.
fn level(line: &str) -> LogLevel {
    let line = line.trim_start();
    if line.starts_with("error") {
        LogLevel::Error
    } else if line.starts_with("warning") {
        LogLevel::Warn
    } else {
        LogLevel::Info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_follows_the_phases_and_keeps_the_last_crate() {
        let mut progress = Progress::default();
        assert_eq!(progress.phase, Phase::Starting);
        for line in ["    Updating crates.io index", "  Downloaded ratatui v0.29.0"] {
            progress.read(line);
        }
        assert_eq!(progress.phase, Phase::Downloading);
        progress.read("   Compiling serde v1.0.219");
        assert_eq!(
            (progress.phase, progress.krate.as_deref(), progress.fraction()),
            (Phase::Compiling, Some("serde"), None)
        );
        progress.read("    Building [=====>      ] 142/231: ratatui, serde");
        assert_eq!(progress.counted, Some((142, 231)));
        assert!((progress.fraction().expect("counted") - 142.0 / 231.0).abs() < 1e-6);
        progress.read("warning: unused import");
        assert_eq!(progress.krate.as_deref(), Some("ratatui"), "a line that says nothing changes nothing");
        progress.read("  Installing /home/ayse/.cargo/bin/qtools");
        assert_eq!(progress.phase, Phase::Placing);
    }

    #[test]
    fn errors_and_warnings_stand_out_in_the_log() {
        assert_eq!(level("error: linker `cc` not found"), LogLevel::Error);
        assert_eq!(level("error[E0425]: cannot find value"), LogLevel::Error);
        assert_eq!(level("warning: spurious network error"), LogLevel::Warn);
        assert_eq!(level("   Compiling serde v1.0.219"), LogLevel::Info);
    }
}
