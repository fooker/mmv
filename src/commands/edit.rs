use std::path::Path;

use clap::{App, ArgMatches, SubCommand};

use crate::changeset::Workspace;
use crate::ProgramError;

pub fn run(workspace: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
    let workspace = Workspace::open(workspace)
        .ok_or_else(|| ProgramError::NotInitialized)?;

    let changeset = workspace.import()?;

    let status = std::process::Command::new("vim")
        .args(&[
            "-O",
            &format!("{}", changeset.workspace().sources_path().display()),
            &format!("{}", changeset.workspace().targets_path().display()),
            "-c", "setlocal readonly | setlocal nobuflisted | windo set scb | set cursorline",
        ])
        .status()
        .map_err(anyhow::Error::from)?;

    // TODO: Print brief status afterwards

    return Ok(());
}

pub fn subcommand() -> App<'static, 'static> {
    return SubCommand::with_name("edit")
        .about("Opens an editor for the change set");
}