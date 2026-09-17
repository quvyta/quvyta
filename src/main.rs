//! quvyta: introduces the Quvyta family of terminal applications and will install them.

mod app;
mod family;

use qframe::runtime::Runtime;

/// The language files, compiled in so an installed binary needs nothing beside it.
const LOCALES: [(&str, &str); 2] =
    [("en.toml", include_str!("../assets/locales/en.toml")), ("tr.toml", include_str!("../assets/locales/tr.toml"))];

fn main() -> std::io::Result<()> {
    LOCALES
        .iter()
        .fold(Runtime::new(app::Quvyta::default()), |runtime, (file, text)| runtime.locale_source(*file, *text))
        .run()
}
