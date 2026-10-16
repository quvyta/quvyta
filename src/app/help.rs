//! The two ways in to everything quvyta can do that the list of apps has no button for: `?`
//! opens the list of every key of the screen, and `ctrl p` opens the palette, where a command is
//! run by typing a few letters of its name.
//!
//! Both are the framework's own widgets; what is quvyta's is what they list. The list of keys is
//! the keys of the screen that is open, in the words the hint bar under that screen uses, and the
//! keymap's own actions, which the framework adds by itself under their own headings. The palette
//! is quvyta's own list rather than the keymap's: every command it offers is one the screen
//! offers right now, sending the very message that button or that key sends, so nothing runs
//! from it that would not have run from the button.

use qframe::env::Env;
use qframe::prelude::*;
use qframe::widgets::{CommandPalette, HelpLayer, PaletteCommand};

use super::installs::InstallMsg;
use super::path_notice::{PathMsg, PathNotice};
use super::updates::UpdateMsg;
use super::{Msg, Quvyta, Tab};
use crate::ecosystem::APPS;

impl Quvyta {
    /// What enter does for the app shown, in the hint bar's own words, or nothing when there is no
    /// such app to act on.
    ///
    /// The hint bar and the list of keys both read it, so the two cannot say different things
    /// about the key the whole screen is built around.
    pub(super) fn main_hint(&self) -> Option<String> {
        if self.installable(self.selected) {
            Some(t!("hints.install"))
        } else if self.opening(self.selected).is_some() {
            Some(t!("hints.open"))
        } else if self.updatable(self.selected) {
            Some(t!("hints.update"))
        } else {
            None
        }
    }

    /// The keys of the screen that is open that no widget declares, each with what it does here
    /// and now: what enter does for the app shown, `u`, `r`, and `esc` where the screen has a way
    /// back.
    ///
    /// The words are the hint bar's own, and the keys come from the keymap, so a rebound key is
    /// named in the list with the word the bar gives it too. The keys a widget takes are left to
    /// the framework: the list under the layer asks the focused widget, which declares its own and
    /// cannot fall behind itself. That is why the app list is left out of the keys below while it
    /// is narrow and takes enter itself.
    pub(super) fn screen_keys(&self, env: &Env) -> Vec<(String, String)> {
        let mut keys: Vec<(String, String)> = Vec::new();
        let mut add = |action: &str, label: String| {
            // An action the keymap binds no key to cannot be pressed, so there is nothing to list.
            if let Some(key) = env.keymap().label_for(Scope::App, action) {
                keys.push((key, label));
            }
        };
        // The page of what changed has one way out, and the Settings tab has none of the app
        // list's keys: they would act on an app nobody sees.
        if self.whats_new {
            add("back", t!("hints.back"));
            return keys;
        }
        if self.tab == Tab::Settings {
            add("back", t!("hints.apps"));
            return keys;
        }
        // Enter belongs to the app list itself while the list takes it: on a narrow screen a row
        // opens that app's page, and the framework names the key after the widget that owns it.
        // A hint here would say a second and different thing about the same key, so the narrow
        // list is left to the widget and has nothing of its own left to say.
        if self.wide() || self.detail_page {
            if let Some(label) = self.main_hint() {
                add("primary", label);
            }
            // Where enter updates already, `u` would say it twice.
            if self.updatable(self.selected) && self.opening(self.selected).is_some() {
                add("update", t!("hints.update"));
            }
            if self.anything_missing() {
                add("install-missing", t!("missing.hint"));
            }
            // On a wide screen the details stand beside the list, so esc is not the way back; on
            // the narrow one the list itself is the page and esc goes nowhere.
            if !self.wide() {
                add("back", t!("hints.back"));
            }
        }
        add("refresh", t!("hints.refresh"));
        keys
    }

    /// The list of every key of the screen that is open, over the screen: the framework's layer,
    /// with the screen's own keys as its hints and the keymap's actions under their own headings.
    /// Esc, the close mark and a click on the dimmed screen close it, as they close the palette,
    /// and the focus goes back where it was.
    pub(super) fn help_layer(&self, env: &Env) -> HelpLayer<Msg> {
        self.screen_keys(env)
            .into_iter()
            .fold(HelpLayer::new(Msg::Help(false)).close_on_click_outside(true), |layer, (key, label)| {
                layer.hint(key, label)
            })
    }

    /// The palette: every command quvyta can run right now, named by what it will do. The
    /// keymap's own actions are left out of the list, since the ones quvyta has no use for would
    /// be commands that change nothing.
    pub(super) fn palette(&self, env: &Env) -> CommandPalette<Msg> {
        CommandPalette::new(self.commands(env), Msg::Palette(false))
    }

    /// Every command the palette offers, in the order it lists them.
    ///
    /// The commands of the app list are the ones the list itself offers, so they are offered
    /// where the list is: the Settings tab and the page of what changed have no list to act on,
    /// and on a narrow screen an app's own page has taken the list's place. The PATH notice, the
    /// tabs and the way out are the screen's own wherever it is.
    fn commands(&self, env: &Env) -> Vec<PaletteCommand<Msg>> {
        let mut commands: Vec<PaletteCommand<Msg>> = Vec::new();
        if self.tab == Tab::Apps && !self.whats_new {
            // The list stands beside the details on a wide screen, and on a narrow one until an
            // app's own page has taken its place.
            if self.wide() || !self.detail_page {
                commands.extend((0..APPS.len()).flat_map(|index| self.app_commands(index)));
                // The button that installs every update at once stands under the list, so the
                // command that asks for it is offered while it is there.
                if (0..APPS.len()).any(|index| self.updatable(index)) {
                    let all = Msg::Updates(UpdateMsg::InstallAll);
                    commands.push(PaletteCommand::new("install-updates", t!("updates.install-all"), all));
                }
                // The line under the list that offers the missing apps, with the key that does
                // what its button does.
                if self.anything_missing() {
                    commands.push(with_chord(
                        PaletteCommand::new("install-missing", t!("palette.missing"), Msg::InstallMissing),
                        chord(env, Scope::App, "install-missing"),
                    ));
                }
            }
            // `r` asks crates.io from the list and from an app's own page alike, and the keymap
            // says which key it is.
            let check = Msg::Updates(UpdateMsg::Check);
            commands.push(with_chord(
                PaletteCommand::new("check", t!("palette.check"), check),
                chord(env, Scope::App, "refresh"),
            ));
        }
        // The notice's own Add, under the words the notice puts on its button.
        if self.path_notice.as_ref().is_some_and(PathNotice::offers_add) {
            commands.push(PaletteCommand::new("path-add", t!("path.add"), Msg::Path(PathMsg::Add)));
        }
        commands.push(PaletteCommand::new("tab:apps", t!("tabs.apps"), Msg::Tab(Tab::Apps)));
        commands.push(PaletteCommand::new("tab:settings", t!("tabs.settings"), Msg::Tab(Tab::Settings)));
        let quit = Msg::Quit;
        commands.push(with_chord(
            PaletteCommand::new("quit", t!("quvyta.keys.quit"), quit),
            chord(env, Scope::Global, "quit"),
        ));
        commands
    }

    /// The commands of the app at `index`: one for each button its details show now, and the one
    /// that shows them. Every command sends the message that button sends, so an install asks
    /// first and a removal asks first, as both do from the button.
    fn app_commands(&self, index: usize) -> Vec<PaletteCommand<Msg>> {
        let app = &APPS[index];
        let mut commands = Vec::new();
        let mut add = |what: &str, key: &str, message: Msg| {
            let id = format!("app:{}:{what}", app.key);
            commands.push(PaletteCommand::new(id, t!(&format!("palette.{key}"), command = app.command), message));
        };
        // An install that runs, waits or has failed stands where the app's own buttons are, with
        // its own lines and its own buttons, so none of them is offered then.
        if !self.installs.has(index) && !self.install_failed(index) {
            if self.opening(index).is_some() {
                add("open", "open", Msg::Open(index));
            }
            if self.installable(index) {
                add("install", "install", Msg::Install(InstallMsg::Ask(index)));
            }
            if self.updatable(index) {
                add("update", "update", Msg::Install(InstallMsg::Ask(index)));
            }
            if self.removable(index) {
                add("remove", "remove", Msg::Install(InstallMsg::AskRemove(index)));
            }
        }
        // The page is always worth showing: what an app is and where it came from is the same
        // whatever its buttons are, and an install in progress says it there too.
        add("show", "show", Msg::ShowDetail(index));
        commands
    }
}

/// The command with the key it already has beside it, and the same command without one where the
/// keymap binds none.
fn with_chord(command: PaletteCommand<Msg>, chord: Option<String>) -> PaletteCommand<Msg> {
    match chord {
        Some(chord) => command.chord(chord),
        None => command,
    }
}

/// The key bound to `action`, as a person writes it, or `None` when the keymap binds none.
fn chord(env: &Env, scope: Scope, action: &str) -> Option<String> {
    env.keymap().label_for(scope, action)
}
