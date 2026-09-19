//! quvyta: lists the Quvyta family of terminal applications, installs them, opens the members
//! that are installed, and updates and removes them.

use std::process::ExitCode;
use std::sync::Arc;

use qframe::runtime::Runtime;
use quvyta::cli::{self, Answer, Start};
use quvyta::{KEYS, LOCALES, Machine, Quvyta};

fn main() -> ExitCode {
    let i18n = Arc::new(cli::i18n(|name| std::env::var(name).ok()));
    let answer = qframe::i18n::scope(i18n, || Answer::to(cli::parse(std::env::args_os().skip(1))));
    if let Some(code) = answer.code() {
        answer.print();
        return code;
    }
    let app = Quvyta::new(Machine::from_env());
    let app = match answer {
        Answer::Start(Start::Install(members)) => app.asking(members),
        Answer::Start(Start::Show(member)) => app.showing(member),
        _ => app,
    };
    let run = LOCALES
        .iter()
        .fold(Runtime::new(app), |runtime, (file, text)| runtime.locale_source(*file, *text))
        .keymap_source("keymap.toml", KEYS)
        .run();
    match run {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("quvyta: {error}");
            ExitCode::FAILURE
        }
    }
}
