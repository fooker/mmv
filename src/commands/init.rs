use std::path::Path;

use clap::{ArgMatches, SubCommand, App, Arg};
use yansi::Paint;

use crate::{ProgramError, scan_tree};
use crate::changeset::{Action, ChangeSet, Workspace};

pub fn run(workspace: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
    let workspace = Workspace::at(workspace);

    if !matches.is_present("force") && workspace.is_initialized() {
        eprintln!("{} {}", Paint::red("Already initialized."), "Use -f to reset");
        return Ok(());
    }

    let records = scan_tree(workspace.path())
        .map(|path| {
            return (path.clone(), Action::Ignore(path.display().to_string()));
        })
        .collect();

    let changeset = ChangeSet::create(workspace, records);
    changeset.export()?;

    println!("{}", Paint::green("Initialized"));

    return Ok(());
}

pub fn subcommand() -> App<'static, 'static> {
    return SubCommand::with_name("init")
        .about("Initialize to a clean state (drops existing change set)")
        .arg(Arg::with_name("force")
            .short("f")
            .long("force")
            .takes_value(false)
            .help("Re-initialize even if already initialized"));
}