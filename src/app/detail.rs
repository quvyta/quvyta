//! The details of one member: what it is, how it is installed and where it comes from.

use qframe::prelude::*;
use qframe::widgets::{Badge, CopyValue};

use super::Msg;
use crate::family::{Member, Status};
use crate::inventory::State;
use crate::machine::Machine;

/// Draws the details of `member`, installed as `state` says; `None` while that is not known yet.
/// `latest` is the newer version it can be updated to. `main` draws the main part under how it
/// is installed: its buttons, or an install under way.
pub(super) fn show(
    member: &Member,
    state: Option<&State>,
    latest: Option<&str>,
    main: impl FnOnce(&mut View<'_, Msg>),
    machine: &Machine,
    ui: &mut View<'_, Msg>,
) {
    let text = |field: &str| t!(&format!("family.{}.{field}", member.key));
    ui.row(|ui| {
        ui.add(Text::new(text("title")).role("title").no_wrap());
        match member.status {
            Status::Released => ui.add(Badge::new(t!("status.released")).variant("success")),
            Status::Beta => ui.add(Badge::new(t!("status.beta")).variant("info")),
            // The accent, not a warning: nothing is wrong, it is only not out yet.
            Status::Soon => ui.add(Badge::new(t!("status.soon")).variant("accent")),
        };
    })
    .gap(2);

    // Package and command sit on one quiet line under the title and wrap on narrow screens.
    let names = [
        Span::new(format!("{}  ", t!("detail.package"))).role("faint"),
        Span::new(member.package),
        Span::new(format!("    {}  ", t!("detail.command"))).role("faint"),
        Span::new(member.command),
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

    if let Some(library) = member.library {
        ui.add(Text::new(text("library"))).fill_width();
        copyable(ui, t!("detail.add"), library, "library");
    }
    // Copyable values go under their label so a narrow screen never cuts them short.
    copyable(ui, t!("detail.source"), member.repository, "source");
    match member.status {
        Status::Beta => {
            ui.add(Text::new(t!("detail.beta")).role("secondary")).fill_width();
        }
        Status::Soon => {
            ui.add(Text::new(t!("detail.soon")).role("faint")).fill_width();
        }
        Status::Released => {}
    }
}

/// A quiet label and its value on one line, with the newer version in the accent colour when
/// there is one; the words say it too, not the colour alone.
fn labelled(ui: &mut View<'_, Msg>, label: &str, value: &str, latest: Option<&str>) {
    let mut spans = vec![Span::new(format!("{label}  ")).role("faint"), Span::new(value)];
    if let Some(latest) = latest {
        spans.push(Span::new(format!("  {}", t!("detail.new", version = latest))).color("accent"));
    }
    ui.add(Text::rich(spans)).fill_width();
}

fn copyable(ui: &mut View<'_, Msg>, label: String, value: &str, id: &str) {
    ui.column(|ui| {
        ui.add(Text::new(label).role("faint").no_wrap());
        ui.add(CopyValue::new(value)).id(id);
    })
    .fill_width();
}
