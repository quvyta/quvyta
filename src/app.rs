//! The screen: the family as tabs and the chosen member's details.

use qframe::prelude::*;
use qframe::widgets::{Badge, CopyValue, ScrollView};

use crate::family::{FAMILY, Member, Status};

/// The application's state: which member is shown.
#[derive(Debug, Default)]
pub struct Quvyta {
    selected: usize,
}

/// Everything that can happen.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    /// Shows the member at this index of [`FAMILY`].
    Select(usize),
}

impl App for Quvyta {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Select(index) if index < FAMILY.len() => self.selected = index,
            Msg::Select(_) => {}
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        AppShell::new().header(header).body(|ui| self.body(ui)).footer(footer).show(ui);
    }
}

impl Quvyta {
    fn body(&self, ui: &mut View<'_, Msg>) {
        ui.column(|ui| {
            let tabs = FAMILY.iter().map(|member| t!(&format!("family.{}.tab", member.key)));
            ui.add(Tabs::new(tabs).active(self.selected).on_select(Msg::Select)).fill_width().id("family");
            let member = &FAMILY[self.selected];
            // Keyed by member so copy confirmations do not carry over to the next tab.
            ui.add_with(ScrollView::new(), |ui| {
                ui.add_with(Panel::new().gap(1), |ui| details(member, ui)).fill_width().id(member.key);
            })
            .fill();
        })
        .gap(1)
        .padding(Padding { top: 1, right: 2, bottom: 0, left: 2 })
        .fill();
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

fn footer(ui: &mut View<'_, Msg>) {
    ui.add(
        KeyHints::new()
            .hint("←→", t!("hints.switch"))
            .action(Scope::Global, "focus-next")
            .action_right(Scope::Global, "quit"),
    )
    .fill_width();
}

fn details(member: &Member, ui: &mut View<'_, Msg>) {
    let text = |field: &str| t!(&format!("family.{}.{field}", member.key));
    ui.row(|ui| {
        ui.add(Text::new(text("title")).role("title").no_wrap());
        match member.status {
            Status::Released => ui.add(Badge::new(t!("status.released")).variant("success")),
            Status::Beta => ui.add(Badge::new(t!("status.beta")).variant("info")),
        };
    })
    .gap(2);

    // Package and command sit on one quiet line under the title and wrap on narrow screens.
    let mut names = vec![Span::new(format!("{}  ", t!("detail.package"))).role("faint"), Span::new(member.package)];
    if let Some(command) = member.command {
        names.push(Span::new(format!("    {}  ", t!("detail.command"))).role("faint"));
        names.push(Span::new(command));
    }
    ui.add(Text::rich(names)).fill_width();
    ui.add(Text::new(text("summary"))).fill_width();

    // Copyable values go under their label so a narrow screen never cuts them short.
    copyable(ui, t!("detail.source"), member.repository, "source");
    // A library is added to a project; an application, which has a command, is installed.
    let install = if member.command.is_some() { t!("detail.install") } else { t!("detail.add") };
    copyable(ui, install, member.install, "install");
    if member.status == Status::Beta {
        ui.add(Text::new(t!("detail.beta")).role("secondary")).fill_width();
    }
}

fn copyable(ui: &mut View<'_, Msg>, label: String, value: &str, id: &str) {
    ui.column(|ui| {
        ui.add(Text::new(label).role("faint").no_wrap());
        ui.add(CopyValue::new(value)).id(id);
    })
    .fill_width();
}

#[cfg(test)]
mod tests;
