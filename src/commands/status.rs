use std::path::Path;

use clap::{App, ArgMatches, SubCommand};
use yansi::Paint;

use crate::changeset::Workspace;
use crate::ProgramError;

pub fn run(workspace: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
    let workspace = Workspace::open(workspace)
        .ok_or_else(|| ProgramError::NotInitialized)?;

    let changeset = workspace.import()?;

    if changeset.is_clean() {
        println!("{}", Paint::green("Workspace is clean"));
    } else {
        println!("{}", Paint::red("Workspace is not clean").bold());
    }

    return Ok(());
}

pub fn subcommand() -> App<'static, 'static> {
    return SubCommand::with_name("status")
        .about("Prints the current status");
}