//! The details of one app: what it is, how it is installed and where it comes from.

use qframe::prelude::*;
use qframe::widgets::{Badge, CopyValue};

use super::Msg;
use crate::ecosystem::{QuvytaApp, Status};
use crate::inventory::State;
use crate::machine::Machine;

/// Draws the details of `app`, installed as `state` says; `None` while that is not known yet.
/// `latest` is the newer version it can be updated to. `main` draws the main part under how it
/// is installed: its buttons, or an install under way. `room` is the columns the heading has.
pub(super) fn show(
    app: &QuvytaApp,
    state: Option<&State>,
    latest: Option<&str>,
    main: impl FnOnce(&mut View<'_, Msg>),
    machine: &Machine,
    room: u16,
    ui: &mut View<'_, Msg>,
) {
    let text = |field: &str| t!(&format!("apps.{}.{field}", app.key));
    let title = text("title");
    let (status, variant) = match app.status() {
        Status::Released => (t!("status.released"), "success"),
        Status::Beta => (t!("status.beta"), "info"),
        // A warning: it works, but it is early enough that trying it is a choice to make.
        Status::Alpha => (t!("status.alpha"), "warning"),
        // The accent, not a warning: nothing is wrong, it is only not out yet.
        Status::Soon => (t!("status.soon"), "accent"),
    };
    // The title never wraps, so when the two do not fit beside each other the badge drops to a
    // line of its own rather than the title losing its end. The line is chosen here rather than
    // left to a wrapping row: a details area long enough to scroll gives the row two cells less
    // than the room measured here, and a row that wrapped would then drop the badge a line
    // earlier than this does.
    let beside = qframe::text::width(&title) + 2 + badge_width(&status) <= room;
    let heading = |ui: &mut View<'_, Msg>| {
        ui.add(Text::new(title).role("title").no_wrap());
        ui.add(Badge::new(status).variant(variant));
    };
    if beside {
        ui.row(heading).gap(2);
    } else {
        ui.column(heading).fill_width();
    }

    // Package and command sit on one quiet line under the title and wrap on narrow screens.
    let names = [
        Span::new(format!("{}  ", t!("detail.package"))).role("faint"),
        Span::new(app.package),
        Span::new(format!("    {}  ", t!("detail.command"))).role("faint"),
        Span::new(app.command),
    ];
    ui.add(Text::rich(names)).fill_width();
    ui.add(Text::new(text("summary"))).fill_width();

    match state {
        Some(State::Cargo { version, path }) => {
            let place = t!("detail.where", version = version.as_str(), path = machine.show(path));
            labelled(ui, &t!("detail.installed"), &place, latest);
        }
        Some(State::Elsewhere { path }) => {
            let place = t!("detail.where", version = t!("detail.unknown"), path = machine.show(path));
            labelled(ui, &t!("detail.installed"), &place, None);
            ui.add(Text::new(t!("detail.elsewhere")).role("secondary")).fill_width();
        }
        Some(State::This { version, .. }) => labelled(ui, &t!("detail.running"), version, latest),
        Some(State::Missing) | None => {}
    }
    main(ui);

    if let Some(library) = app.library {
        ui.add(Text::new(text("library"))).fill_width();
        copyable(ui, t!("detail.add"), library, "library");
    }
    // Copyable values go under their label so a narrow screen never cuts them short.
    copyable(ui, t!("detail.source"), app.repository, "source");
    match app.status() {
        Status::Beta => {
            ui.add(Text::new(t!("detail.beta")).role("secondary")).fill_width();
        }
        Status::Alpha => {
            ui.add(Text::new(t!("detail.alpha")).role("secondary")).fill_width();
        }
        Status::Soon => {
            ui.add(Text::new(t!("detail.soon")).role("faint")).fill_width();
        }
        Status::Released => {}
    }
}

/// A quiet label and its value on one line, with the newer version in the accent colour when
/// there is one; the words say it too, not the colour alone. A `Field` value is one plain text,
/// so it could not colour the version.
fn labelled(ui: &mut View<'_, Msg>, label: &str, value: &str, latest: Option<&str>) {
    let mut spans = vec![Span::new(format!("{label}  ")).role("faint"), Span::new(value)];
    if let Some(latest) = latest {
        spans.push(Span::new(format!("  {}", t!("detail.new", version = latest))).color("accent"));
    }
    ui.add(Text::rich(spans)).fill_width();
}

fn copyable(ui: &mut View<'_, Msg>, label: String, value: &str, id: &str) {
    ui.add(CopyValue::new(value).labelled(label)).id(id).fill_width();
}

/// The columns a badge takes: its words, its air and the dot before them, which is one cell
/// whichever glyph set is on.
pub(super) fn badge_width(label: &str) -> u16 {
    qframe::text::width(label) + 4
}
