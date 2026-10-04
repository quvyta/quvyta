//! Opening an app: the terminal is handed to it until it closes.

use std::path::PathBuf;

use qframe::prelude::*;
use qframe::runtime::{Handoff, HandoffOutcome};
use qframe::widgets::Toast;

use super::Msg;
use crate::ecosystem::QuvytaApp;

/// How an app is started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Opening {
    /// The command's full path, so opening works even when its folder is not on `PATH`.
    pub(super) program: PathBuf,
    /// The home folder: an app behaves the same wherever quvyta was started from.
    pub(super) dir: PathBuf,
}

impl Opening {
    /// Hands the terminal to the app at `index`; [`Msg::Closed`] arrives when it closes.
    pub(super) fn handoff(self, index: usize, app: &QuvytaApp) -> Command<Msg> {
        let handoff = Handoff::new(self.program, move |outcome| Msg::Closed { index, outcome })
            .dir(self.dir)
            .notice(t!("open.notice", command = app.command));
        Command::handoff(handoff)
    }
}

/// What to tell after `app` closed with `outcome`: nothing when it went well.
pub(super) fn report(app: &QuvytaApp, outcome: &HandoffOutcome) -> Command<Msg> {
    let command = app.command;
    let toast = match outcome {
        HandoffOutcome::Finished { code: Some(0) } => return Command::none(),
        HandoffOutcome::Finished { code: Some(code) } => {
            Toast::info(t!("open.exited", command = command, code = *code))
        }
        HandoffOutcome::Finished { code: None } => Toast::info(t!("open.signal", command = command)),
        HandoffOutcome::Failed(reason) => Toast::danger(t!("open.failed", command = command)).body(reason.clone()),
    };
    Command::toast(toast.key("closed"))
}
