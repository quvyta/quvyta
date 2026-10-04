//! What quvyta keeps of the framework's numbers and keys must stay the framework's own.
//!
//! The split between the list and the details, the room an app's details need, the width of
//! the tabs in the header and the room an app's line has under a checkbox are measured from
//! `update` and `action`, where the environment and with it the icon set is out of reach, so
//! quvyta keeps its own arithmetic instead of asking the widget. The keys in the hint bar, on
//! the other hand, can be asked: they come from the keymap the runtime loaded, and the share a
//! row carries while an app installs is a number the framework writes in the language on
//! screen.
//!
//! Every test here draws the framework's own widget, or rebinds a key, and compares. When the
//! framework measures a button, a badge, a checkbox, a list or a strip of tabs differently, or
//! when a key is bound elsewhere, these tests say so instead of the screen quietly moving.

use qframe::env::{AssetDirs, Env};
use qframe::icons::GlyphMode;
use qframe::keymap::Scope;
use qframe::prelude::*;
use qframe::runtime::{Command, Harness};
use qframe::widgets::{Badge, Button, Checkbox, List};

use super::detail::badge_width;
use super::install_view::{button_width, percent};
use super::tests::{LANGUAGES, env, machine};
use super::wizard::LINE_INDENT;
use super::*;
use crate::updates::tests::search_scenario;

/// The key the footer's hint for `action` says, as the keymap writes it, or `None` when the bar
/// leaves that action out.
fn hint_key(h: &Harness<Quvyta>, action: &str, label: &str) -> Option<String> {
    let key = h.env().keymap().chords_for(Scope::App, action).first()?.label();
    let hint = format!(" {key}  {}", h.env().i18n().translate(label, &[]));
    h.screen().contains(&hint).then_some(key)
}

/// The environment the tests see, with quvyta's own keys replaced by `keys`.
fn env_with_keys(keys: &str) -> Env {
    let dirs = AssetDirs {
        locale_sources: crate::LOCALES.iter().map(|(file, text)| ((*file).to_owned(), (*text).to_owned())).collect(),
        keymap_source: Some(("keymap.toml".to_owned(), keys.to_owned())),
        ..AssetDirs::default()
    };
    Env::load_with(&dirs, |name| (name == "COLORTERM").then(|| "truecolor".to_owned()))
        .expect("the locales are readable")
}

/// What [`Draw`] paints on a screen of its own.
type Painting = Box<dyn Fn(&mut View<'_, ()>)>;

/// Draws one thing on a screen of its own, so the cells the framework gives it can be read off
/// the ground it paints.
struct Draw(Painting);

impl App for Draw {
    type Msg = ();
    fn update(&mut self, _: ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        (self.0)(ui);
    }
}

/// A harness of `width` columns for one thing on a screen of its own.
fn alone(width: u16, height: u16, build: impl Fn(&mut View<'_, ()>) + 'static) -> Harness<Draw> {
    let mut h = Harness::with_env(Draw(Box::new(build)), env(), width, height);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    h.render();
    h
}

/// The columns `build` covers with its own ground on the first row of a screen `width` cells
/// wide: from the first cell that is not the canvas to the last.
fn ground_width(width: u16, build: impl Fn(&mut View<'_, ()>) + 'static) -> u16 {
    let h = alone(width, 3, build);
    let canvas = h.env().theme().color("canvas");
    let not_canvas = |x: u16| h.bg(x, 0) != canvas;
    match ((0..width).find(|x| not_canvas(*x)), (0..width).rev().find(|x| not_canvas(*x))) {
        (Some(first), Some(last)) => last - first + 1,
        _ => 0,
    }
}

/// The hint bar names the keys the keymap binds, so a key bound elsewhere is named where it
/// went and the key it used to be does nothing.
#[test]
fn the_footer_names_the_keys_the_keymap_binds() {
    for locale in ["en", "tr"] {
        let (_root, mut h) = super::tests::harness(100, 24);
        h.set_locale(locale);
        let open = h.env().i18n().translate("hints.open", &[]);
        // Enter, `r` and esc are what quvyta binds today, and the bar must say so.
        assert_eq!(hint_key(&h, "primary", "hints.open").as_deref(), Some("enter"), "{locale}");
        assert_eq!(hint_key(&h, "refresh", "hints.refresh").as_deref(), Some("r"), "{locale}");
        let settings = h.env().i18n().translate("tabs.settings", &[]);
        h.click_text(&settings);
        assert_eq!(hint_key(&h, "back", "hints.apps").as_deref(), Some("esc"), "{locale}");
        assert!(!h.screen().contains(&format!("enter  {open}")), "the bar has changed with the tab:\n{}", h.screen());
    }
    // The same screen with the keys somewhere else: the bar follows the keymap, and the key the
    // bar names is the one that opens the app.
    let root = tempfile::tempdir().expect("temp");
    let app = Quvyta::new(machine(root.path()));
    let keys = "[app]\nprimary = \"f2\"\nback = \"esc\"\nrefresh = \"f3\"\nupdate = \"f4\"\n";
    let mut h = Harness::with_env(app, env_with_keys(keys), 100, 24);
    h.set_locale("en").set_glyph_mode(GlyphMode::Unicode);
    h.render();
    assert_eq!(hint_key(&h, "primary", "hints.open").as_deref(), Some("f2"), "{}", h.screen());
    assert_eq!(hint_key(&h, "refresh", "hints.refresh").as_deref(), Some("f3"), "{}", h.screen());
    h.press("enter").press("r");
    assert!(h.handoffs().is_empty(), "neither of the keys quvyta used to bind does anything now");
    h.press("f2");
    let [request] = h.handoffs() else { panic!("f2 opens what the bar says: {:?}", h.handoffs()) };
    assert!(std::path::Path::new(&request.program).ends_with("qcode"), "{request:?}");
}

/// A button is its label with the theme's air on both sides, and its icon adds a glyph and a
/// space: the room the details reserve for their buttons is measured in quvyta's own numbers.
#[test]
fn a_button_is_wider_than_its_label_by_the_theme_air() {
    for label in ["Install", "Güncellemeleri kur", "安装更新", "coping", ""] {
        let drawn = ground_width(200, {
            let label = label.to_owned();
            move |ui| {
                ui.add(Button::new(label.clone()));
            }
        });
        assert_eq!(button_width(label), drawn, "`{label}`");
    }
    // The details button carries a chevron before its words, so it is two cells wider.
    let plain = ground_width(200, {
        let label = "Details".to_owned();
        move |ui| {
            ui.add(Button::new(label.clone()));
        }
    });
    let with_icon = ground_width(200, {
        let label = "Details".to_owned();
        move |ui| {
            ui.add(Button::new(label.clone()).icon("chevron-right"));
        }
    });
    assert_eq!(with_icon, plain + 2, "a glyph and its space, as the split assumes");
}

/// A badge is its words with one cell of padding on each side and a one-cell dot before them,
/// whichever glyph set is on: the room the details reserve for the badge beside the title.
#[test]
fn a_badge_is_wider_than_its_words_by_its_padding_and_its_dot() {
    for (mode, label) in [
        (GlyphMode::Nerd, "Released"),
        (GlyphMode::Unicode, "Yayında"),
        (GlyphMode::Unicode, "ベータ"),
        (GlyphMode::Ascii, "Coming soon"),
    ] {
        let drawn = ground_width(40, {
            let label = label.to_owned();
            move |ui| {
                ui.add(Badge::new(label.clone()));
            }
        });
        // The dot is the one cell that could differ between the glyph sets, so the badge is
        // drawn again in this set and measured the same way.
        let mut h = alone(40, 3, {
            let label = label.to_owned();
            move |ui| {
                ui.add(Badge::new(label.clone()));
            }
        });
        h.set_glyph_mode(mode).render();
        let canvas = h.env().theme().color("canvas");
        let painted = u16::try_from((0..40).filter(|x| h.bg(*x, 0) != canvas).count()).expect("fewer cells than 65536");
        assert_eq!(badge_width(label), painted, "{mode:?} `{label}`");
        assert_eq!(badge_width(label), drawn, "{mode:?} `{label}`");
    }
}

/// The line under a checkbox in the wizard starts where the checkbox's own label starts, so a
/// app's words stand under its box and not under its name.
#[test]
fn the_wizard_line_under_a_checkbox_starts_where_its_label_does() {
    let h = alone(40, 3, |ui| {
        ui.add(Checkbox::new(false).label("Label"));
    });
    let (x, _) = h.find("Label").expect("the label is drawn");
    assert_eq!(u16::try_from(x).expect("a column"), LINE_INDENT);
}

/// The list is as wide as the framework measures its widest row, and a row is whole at that
/// width and cut two cells less: the number the list column is made of is the framework's own.
#[test]
fn the_list_column_is_the_width_the_widest_row_needs() {
    let (_root, h) = super::tests::harness(200, 20);
    let app = h.app();
    let compact = app.compact();
    let need = app.list_width(compact);
    assert!(need > 4, "{need} columns is too little for a row");
    let items = app.list_items(compact);
    // Drawn alone, the list takes exactly the width it asks for, and every row's state ends one
    // cell before its right edge, where the row keeps a spare cell for the slide.
    let detail = app.row_text(0, compact).expect("a row says how its app is installed");
    let drawn = alone(200, 20, {
        let items = items.clone();
        move |ui| {
            ui.add(List::new(items.clone()));
        }
    });
    let (x, _) = drawn.find(&detail).unwrap_or_else(|| panic!("`{detail}` is drawn"));
    let state_end = u16::try_from(x).expect("a column") + qframe::text::width(&detail) + 1;
    assert_eq!(need, state_end, "quvyta measures the list as {need}, the framework as {state_end}");
    for (width, cut) in [(need, false), (need - 2, true)] {
        let h = alone(width, 20, {
            let items = items.clone();
            move |ui| {
                ui.add(List::new(items.clone()));
            }
        });
        let screen = h.screen();
        assert_eq!(screen.contains('…'), cut, "at {width} columns, a row of {need}:\n{screen}");
    }
}

/// What crates.io answers, with an update waiting for two apps.
fn search() -> String {
    format!(
        "quvyta-code = \"0.1.2\"\nquvyta-tools = \"0.1.2\"\nquvyta-framework-showcase = \"0.1.5\"\nquvyta = \"{}\"\n",
        env!("CARGO_PKG_VERSION")
    )
}

/// The surfaces `row` carries, each a run of cells that are not the canvas: one button is one
/// run, wherever its own words begin and however they are shortened.
fn surfaces_on(h: &Harness<Quvyta>, row: u16, width: u16) -> usize {
    let canvas = h.env().theme().color("canvas");
    let (mut runs, mut inside) = (0, false);
    for x in 0..width {
        let painted = h.bg(x, row) != canvas;
        if painted && !inside {
            runs += 1;
        }
        inside = painted;
    }
    runs
}

/// The columns the list area has in the language on screen: the app measures it from the words
/// of the language files, so it is asked with that language in scope.
fn list_area_width(h: &Harness<Quvyta>) -> u16 {
    let i18n = std::sync::Arc::new(h.env().i18n().clone());
    qframe::i18n::scope(i18n, || if h.app().wide() { h.app().list_column() } else { h.app().list_room() })
}

/// The count and the button beside it, as the framework measures them side by side.
fn updates_width(count: &str, install: &str) -> u16 {
    let count = count.to_owned();
    let install = install.to_owned();
    ground_width(200, move |ui| {
        ui.row(|ui| {
            ui.add(Button::new(count.clone()));
            ui.add(Button::new(install.clone()).variant("primary"));
        })
        .gap(2);
    })
}

/// The count and the button beside it stay on one line exactly while the column is wide enough
/// for the two: the row asks, and quvyta no longer measures the buttons a second time.
#[test]
fn the_buttons_under_the_list_stay_beside_each_other_exactly_while_they_fit() {
    for locale in LANGUAGES {
        for width in (30..=130).step_by(3) {
            let root = tempfile::tempdir().expect("temp");
            let app = Quvyta::new(machine(root.path()));
            search_scenario(root.path(), &search(), 0);
            let mut h = super::install_tests::run(app, width, 30);
            h.set_locale(locale);
            super::install_tests::settle(&mut h);
            // The words the screen shows in this language, and the room the two buttons take in
            // it as the framework measures them.
            let i18n = h.env().i18n();
            let count = i18n.translate("updates.count", &[("n", qframe::i18n::Arg::Int(2))]);
            let install = i18n.translate("updates.install-all", &[]);
            let need = updates_width(&count, &install);
            let screen = h.screen();
            let (_, at) =
                h.find(&count).unwrap_or_else(|| panic!("`{count}` is missing at {locale} {width}:\n{screen}"));
            // Beside each other when the count's own line carries two surfaces: a button is one
            // run of cells whatever its words are, shortened or not.
            let row = u16::try_from(at).expect("a row");
            let column = list_area_width(&h);
            assert_eq!(
                surfaces_on(&h, row, width),
                usize::from(column >= need) + 1,
                "{locale} at {width}: a column of {column} and buttons of {need}\n{screen}"
            );
        }
    }
}

/// The share a row carries while an app installs is written the way the language on screen
/// writes a number, which is the one way every number the framework draws is written. A whole
/// number has no decimal point to place, so both ends of this link write the same characters in
/// every language quvyta has; what is pinned here is the value, and where it is written.
#[test]
fn the_share_of_a_running_install_is_the_frameworks_own_number() {
    for locale in LANGUAGES {
        let mut i18n = env().i18n().clone();
        assert!(i18n.set_active(locale), "{locale} is a language quvyta has");
        let i18n = std::sync::Arc::new(i18n);
        qframe::i18n::scope(i18n, || {
            for (fraction, want) in
                [(0.0, "0"), (0.006, "0"), (0.5, "50"), (0.614, "61"), (0.999, "99"), (1.0, "100"), (1.4, "100")]
            {
                let shown = percent(fraction);
                assert_eq!(shown, want, "{locale} at {fraction}");
                assert_eq!(
                    shown,
                    qframe::i18n::number(f64::from(fraction * 100.0).clamp(0.0, 100.0).floor(), 0),
                    "{locale} at {fraction}"
                );
            }
        });
    }
}

/// quvyta is where the ecosystem's apps are found, so an app the framework lists is one
/// quvyta lists, installs and opens. The test the apps are known by runs the other way only.
#[test]
fn an_app_the_framework_lists_is_one_quvyta_lists() {
    for known in crate::ecosystem::FRAMEWORK_APPS {
        let listed = APPS.iter().find(|app| app.package == known.package);
        let listed = listed.unwrap_or_else(|| panic!("`{}` is an app the framework lists", known.package));
        assert_eq!(listed.key, known.id, "{}: quvyta and the framework call it different things", known.package);
        assert_eq!(listed.command, known.command, "{}", known.package);
    }
}

/// The tabs of the header stand where the header counts them, and the name and the tagline keep
/// their four cells from them: the strip the header measures is the one drawn, so the tagline
/// never reads as one more word of the tabs.
#[test]
fn the_name_and_the_tagline_keep_their_room_from_the_tabs_the_framework_draws() {
    for locale in ["en", "tr", "ja"] {
        let (_root, mut h) = super::tests::harness(100, 24);
        h.set_locale(locale);
        let i18n = h.env().i18n();
        let labels = [i18n.translate("tabs.apps", &[]), i18n.translate("tabs.settings", &[])];
        let tagline = i18n.translate("header.tagline", &[]);
        let tabs = tabs_width(h.env(), &labels);
        // The fewest columns where the name and the tagline fit with the four cells each of them
        // keeps from the strip: the header's own padding, the strip and the space around it.
        let words = qframe::text::width("quvyta") + qframe::text::width(&format!("  {tagline}"));
        let fits = 8 + tabs + words;
        for width in (fits.saturating_sub(2)..=fits + 2).chain([60, 100]) {
            let (_root, mut h) = super::tests::harness(width, 24);
            h.set_locale(locale);
            let screen = h.screen();
            let header = screen.lines().next().unwrap_or_default();
            // The strip starts with the open tab's mark, the first tab being open; its label
            // follows it.
            let at = header.rfind('▌').unwrap_or_else(|| panic!("the open tab at {locale} {width}:\n{screen}"));
            assert!(header[at..].contains(&labels[0]), "`{}` at {locale} {width}:\n{screen}", labels[0]);
            assert_eq!(
                qframe::text::width(&header[..at]),
                width - 2 - tabs,
                "quvyta counts the strip as {tabs}, the framework draws it elsewhere at {locale} {width}:\n{screen}"
            );
            assert_eq!(
                header.contains(&tagline),
                width >= fits,
                "the tagline and the tabs at {locale} {width}:\n{screen}"
            );
        }
    }
}
