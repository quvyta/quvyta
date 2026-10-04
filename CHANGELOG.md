# Changelog

What changed in each release of quvyta. Versions follow [Semantic Versioning](https://semver.org/); while the version starts with 0, a minor release may change how things look or where they are kept.

## 0.3.2 (2026-10-04)

- Three more Quvyta apps are on the list and in both install scripts: **qexp**, the file explorer, **qbrow**, the web browser that runs an unmodified Chromium out of sight, and **qcli**, the small coding agent. qcli is an alpha with only pre-releases so far, which cargo installs only when the version is named, so quvyta and `install.sh` ask crates.io for the newest one first; its page carries an "Alpha" badge. The three are built for Linux and macOS: `install.ps1` lists them and leaves them out on Windows, and the setup wizard does not offer them there.
- **Install updates** asks before it does anything. It puts one question in front of every update: each app it would update, named with the version it has and the one it would go to, each with the exact cargo command that runs, and where the programs are put. The same checks a single install goes through run first, and nothing starts until you agree; when they find something in the way, the question says what that is and offers to look again instead of a button to press. With one app to update it is that app's own update question, and changing your mind leaves every one of them waiting as it was.
- An app you built yourself with `cargo install --path` or `cargo install --git` is left alone. quvyta reads where each program on the machine came from, so such an app counts as one installed some other way: it still opens, and it is never updated and never removed, so the version on crates.io does not quietly replace your own build. quvyta itself is treated the same way. Nothing is asked about it, and it is not counted among the updates on offer.
- The question before installing qcli, which is out only as pre-releases and so cargo installs only when its version is named, now names that version: the command on the screen is the command that will run, down to the version, even when the update check at start never found qcli or the shared update notice is turned off. quvyta asks crates.io while it looks for Rust and a C linker, so the answer is there before the button can be pressed.
- The list offers only what runs on this machine. A Quvyta app built for Arch Linux only says **Arch Linux only** in its row on any other system, and the three built for Linux and macOS say **Not for Windows** on Windows; its page carries the same words where the Install button would be, with nothing to press, and `quvyta install tools` on a machine that is not Arch Linux says why instead of asking. The list and the first-start wizard now ask one rule, in every language's own words. An app already installed on such a machine is not held back by any of this: it still opens, updates and removes like any other.
- After an update, the first start tells you what changed in that version. A page over the list shows the entry for the version you now run, in the words this changelog already writes it, once and then no more; someone new meets the setup wizard and never this page. Esc, the **Back** button or a click closes it.
- The Settings tab offers to install the Nerd Font symbols when the machine has none, the way the first-start wizard does. It asks first, naming the download, the version and the folder it goes into, and says that the download is checked against the checksum written into quvyta, that nothing outside your own account is told about the new font, and that deleting that folder undoes it. No sudo is needed. A machine that has one of these fonts already is told nothing.
- cargo now runs in your home folder, every time quvyta calls it. Starting quvyta inside a Rust project no longer lets that project's toolchain file decide which Rust cargo uses, or make rustup download another toolchain just to look for updates and read what is installed.
- Finishing the first-start wizard no longer writes quvyta's default settings into `launcher.conf`. A setting you never chose keeps following the default, so a better default in a later version reaches you instead of being frozen at the value from the day you set quvyta up.
- The update check now asks crates.io for a hundred crates at a time instead of twenty. The crates around the Quvyta apps share the name, and their number grows with every app, so a shorter answer would let an app fall off the list of what quvyta can see.
- The share shown while an app installs is now written the way your language writes numbers, and the bar along the foot of the screen takes each key from the keymap the program is running with, so moving a key can no longer leave that bar advertising a key that does nothing.
- On the Settings tab, the titles over the follow table and over quvyta's own settings look like the Appearance heading above them, and in the follow table the faint "shared" of the row you are on is drawn in that row's colour instead of fading against the selection.
- Started without `HOME`, or with a relative folder on `PATH`, quvyta no longer looks for cargo in the folder it was started from.
- `quvyta --help` in Chinese and Japanese lines its descriptions up in one column again: the forms were padded by letters, and a letter there takes two cells.
- The line that puts cargo's folder on `PATH`, written by quvyta and by `install.sh`, now quotes a folder whose name holds a space, a quote, a dollar sign or a backquote, so the shell's start-up file keeps working. The usual folders get the same line as before.
- **Stop** ends an install at once even while quvyta is still asking crates.io which version of an alpha app to install, and that question, like the check of the Rust version, gives up after a while on a network that does not answer instead of holding the install.
- An install that ends by itself while you are being asked whether to stop it closes the question, so pressing **Stop** a moment late can no longer stop the next install in the queue.
- The follow table on the Settings tab shows the theme an app really draws with when its own theme is one the framework does not have: the built-in default, not the name in its file.
- In the install question, a value too long to sit beside its label, such as a long target folder, now goes on its own line under the label instead of wrapping back under it.
- `quvyta --help` and the README call the Quvyta apps apps, in every language, where they still said members.
- Requires quvyta-framework 0.1.32.

## 0.3.1 (2026-09-25)

- qdesk is out, so quvyta no longer calls it coming soon: its row says whether it is installed, its page has an Install button, `quvyta install qdesk` and both install scripts install it, `all` includes it, and the setup wizard offers it. What quvyta does with a member that is not released yet stays as it was, for the next one.
- Reduced motion is now shared by every Quvyta app, like the language and the theme: the Settings tab has an "In every Quvyta application" box under it too, and the follow table shows, in a column of its own, which apps reduce motion or keep full motion of their own, with a choice to put them back on the shared setting.
- The follow table reads qdesk's settings from `desktop.conf`, where qdesk keeps them, instead of a `desk.conf` it never writes; which file each app keeps its settings in now comes from the framework's own list of Quvyta apps.
- Where the table's columns do not all fit, as with a long language name in Portuguese at 100 columns, the Settings tab shows one line per app instead of a table that scrolls sideways and hides a value.
- Requires quvyta-framework 0.1.29.

## 0.3.0 (2026-09-23)

- The Quvyta apps are now called the Quvyta ecosystem everywhere quvyta speaks of them, instead of a family: in the descriptions on the list, the command-line help and messages, the notes about Rust and cargo before an install, the one-line installers, the README and the package description, in all nine languages. Nothing else changes: the same apps, the same settings files and the same shared switch.

## 0.2.10 (2026-09-23)

- A member can be put back on the family's settings from quvyta. On the Settings tab, press enter on or click a member in the follow table that chose a language, theme or icon set of its own: a small menu offers to follow the shared one again, one choice for each setting it does not share. Only that setting changes in the member's own file; the family's shared settings and the member's other settings stay as they are. On a narrow terminal the same choices are buttons under the member. A member that shares everything, has never been opened, or has a file quvyta cannot read offers nothing, and quvyta never writes to a file it could not read.
- The buttons under an install keep one layout rule in every language: they stand side by side, and the ones that do not fit move to the next line, instead of the whole row turning into a column the moment one button is too wide.
- Requires quvyta-framework 0.1.19.

## 0.2.9 (2026-09-23)

- quvyta follows the family's update notice, the one switch every Quvyta application shares. The Settings tab shows it under the appearance rows as **Say when an update is out**, in the same words as every other member, and turning it off there turns it off for the whole family; quvyta's own **Check for updates at start** is gone. quvyta now asks crates.io at most once a day, as the notice promises, instead of every six hours; `r` still asks at any time. If you had turned quvyta's own switch off, your choice is kept: the first start turns the family's switch off and removes `check_updates` from `launcher.conf`. The README has a new section on exactly what quvyta sends over the network.
- The Settings tab now shows whether each installed member of the family follows the shared language, theme and icons. Between the shared appearance and quvyta's own settings, a table lists every member you have installed with, for each of the three, a faint "shared" when it follows the family or its own value when it chose one, named the way the appearance rows name it; on a narrow terminal each member takes one line listing only what it does not share, or "all shared". A member you have never opened says so and counts as following, since it will start on the family's values. When a member's settings file cannot be read, its row says "unreadable", a notice tells you once where and why, and quvyta leaves the file exactly as it is. The files are only read, never written, and they are read again each time you open the tab, so a member you opened in the meantime shows what it saved.
- Requires quvyta-framework 0.1.18.

## 0.2.8 (2026-09-21)

- No more copying commands into a terminal. When an install cannot start because Rust or a C linker is missing, the question now has **Install here** beside the command that puts it right: quvyta steps aside, runs exactly that line in the terminal you are looking at, waits for a key so you can read what it printed, and checks again. For a linker that is your system's own `sudo pacman`, `sudo apt` or `sudo dnf` line; sudo asks for your password itself, on that terminal, and quvyta never sees it. Nothing runs until you press the button. Without Rust, the line is rustup's official installer, which asks its own questions and needs no sudo. On a system quvyta does not recognise, the commands are shown to copy and none is run, since it cannot tell which one is right. A build that failed for want of a linker has the same button.
- The one-line installers offer the same. When the C linker is missing, `install.sh` shows the one command for your distribution and asks whether to run it, `install.ps1` offers to run the winget command for the Visual Studio Build Tools, and on macOS the script offers to open Apple's installer for the Command Line Tools. Only a yes typed on the terminal runs these; `--yes` never answers them, and the answer is no by default. Started with PowerShell on Linux or macOS, `install.ps1` offers to run `install.sh` for you.
- Nothing is cut on a narrow terminal, whichever language you read in. The details now open beside the list only once the words of that language fit beside it, measured over the whole family so the layout never shifts while you walk down the list; a badge that will not sit beside a title drops to its own line, and the headings over an install wrap instead of breaking off. The lines that shorten themselves on purpose still do: a long file path, the `PATH` line for your shell, and the crate name cargo is compiling.
- quvyta now speaks nine languages: English, Turkish, German, Spanish, French, Brazilian Portuguese, Russian, Simplified Chinese and Japanese. Every screen, dialog, hint and command-line message reads in all of them, and each language uses its own everyday words rather than a word-for-word English; program names, packages, commands, files and paths stay as they are. The language files are compiled into the program, so an installed binary needs nothing beside it, and the Settings tab picks the language for the whole family. No language is missing a line that English has.
- Requires quvyta-framework 0.1.17.

## 0.2.7 (2026-09-20)

- The first start now asks before it shows anything else: the language, the colour theme and the icon set the whole family shares, each with a box saying where the choice holds, and then which members of the family you would like. Nothing is written until you finish, so a quvyta closed half-way leaves the settings folder exactly as it was and asks again next time; the members you checked go through the same question before an install as the list does, one after another, and nothing is installed unasked. Members that run on Arch Linux only, and the ones that are not out yet, are listed but cannot be checked elsewhere. Someone who already has quvyta's settings file is not asked. Requires quvyta-framework 0.1.11.
- The one-line installers know the name `desk`. Naming it answers the way quvyta itself does, with one line saying qdesk is not released yet and that quvyta's own list shows it as coming soon, and installs nothing for it; other names given with it are installed as usual. It is not part of `all` and has no number in the picker, since there is nothing on crates.io to install, and the family list shows it as coming soon so the family is seen whole.
- While a member installs, the screen now shows how far cargo has got: the crates built out of the crates in the build, the crate being compiled, and a bar that fills instead of sweeping. cargo redraws those counts over the same line, and quvyta now reads every redraw; the lines under Details and the log file stay exactly the lines cargo wrote.

## 0.2.6 (2026-09-20)

- The Settings tab now opens with the appearance every Quvyta application shares: the language, the colour theme and the icon set, each with a box saying whether the choice holds in every Quvyta application or in quvyta alone, and under them Reduce motion and the pillar. Choosing here takes at once, and every member that follows the family opens with it.
- qcode is described as running coding agents, the words the family uses everywhere now.

## 0.2.5 (2026-09-20)

- The README opens with a short recording of a visit: the family list, choosing a member, the question with the exact command, the install running, and Open on it at the end. It is drawn from the same invented scenes as the pictures, so nothing in it comes from a real machine.
- The pictures show only the progress a real install delivers. cargo's `Building 142/231` frames are written over the same line, and quvyta does not receive them yet, so no picture pretends otherwise.
- Requires quvyta-framework 0.1.10.

## 0.2.4 (2026-09-20)

- qdesk, a desktop that runs inside the terminal, joins the family list as "Coming soon". Its page shows what it will be, its command, package and source; quvyta does not offer to install or update it until it is released, and `quvyta install qdesk` says so. A build of qdesk already on your PATH opens like any other member.
- `quvyta show NAME` opens straight on one member's page, so another program can send you to it; on a narrow terminal `esc` goes back to the list.

## 0.2.3 (2026-09-19)

- The README opens with what quvyta is for in one sentence, and shows the Settings tab.
- This changelog, a contributing guide, and templates for bug reports and ideas.

## 0.2.2 (2026-09-19)

- A Settings tab beside the family list, for quvyta's own settings: whether it looks for updates at start, what happens when a program opened from quvyta closes, and whether `~/.cargo/bin` is on your PATH, with Add when it is not. Every change is saved at once; if the file cannot be written, the setting goes back and quvyta says why.
- The tests pass when `CARGO_TARGET_DIR` is set in the shell that runs them.

## 0.2.1 (2026-09-19)

- `install.sh` works on macOS: the PATH line goes to `.zprofile` for zsh or `.bash_profile` for bash, the Command Line Tools are checked as the linker, and the members that run only on Arch Linux are skipped.
- `install.ps1` installs the family on Windows from PowerShell, without an administrator: Rust through rustup, the members from crates.io, and cargo's folder on your user PATH only after asking. If the Visual Studio C++ Build Tools are missing it prints the command that installs them.
- A command line: `quvyta --help`, `quvyta --version`, and `quvyta install NAME...`, which opens on the install question of each member named.
- The install and remove questions show cargo's whole command: the dialog widens to fit it, and on a narrow screen the command wraps.
- Requires quvyta-framework 0.1.7.

## 0.2.0 (2026-09-18)

- quvyta installs, opens, updates and removes the family's programs. Before an install it shows the source, the version, where the program goes and the exact cargo command; installs run one at a time in a queue, can be stopped, and a failure is told in a plain sentence with cargo's last lines and a kept log.
- It reads what is installed from cargo's own record, and finds members installed some other way on your PATH.
- It asks crates.io for newer versions at most once every six hours; **Install updates** updates them all, and `u` updates one.
- An installed member opens from the list; when it closes, quvyta comes back (or quits, with `after_close = "shell"`).
- It offers to put `~/.cargo/bin` on your PATH, with the same rules as `install.sh`.

## 0.1.2 (2026-09-18)

- A one-line installer, `install.sh`: it lists the family, installs Rust with rustup if cargo is missing, installs the members you pick with cargo, and adds cargo's folder to your shell's PATH only if you agree.
- qpackages' command is `qpac`.

## 0.1.1 (2026-09-18)

- The family's applications are shown as betas, each with the command that installs it.
- A beta note and the family table in the README.

## 0.1.0 (2026-09-17)

- The first release: a terminal app that introduces the Quvyta family, with what each member does, its package and its command, in English and Turkish.
