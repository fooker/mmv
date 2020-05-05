use std::path::Path;

use clap::{Arg, ArgMatches, SubCommand, App};
use yansi::Paint;

use crate::changeset::{Action, Workspace};
use crate::ProgramError;

pub fn run(workspace: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
    let target = matches.value_of("target").expect("No target");
    let target = Path::new(target);

    let workspace = Workspace::open(workspace)
        .ok_or_else(|| ProgramError::NotInitialized)?;

    let changeset = workspace.import()?;
    let changeset = changeset.clean()
        .ok_or_else(|| ProgramError::NotClean)?;

    // Execute actions in two steps: first, copy files which should be moved, second delete files
    // either because they are moved or marked for deletion
    for (source, action) in changeset.records().iter() {
        let source = changeset.path().join(&source);

        match action {
            Action::Move(path) => {
                let target = target.join(path);

                print!("{} {} ", Paint::cyan("➤").bold(), target.display());

                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(anyhow::Error::from)?;
                }

                reflink::reflink_or_copy(&source, &target)
                    .map_err(anyhow::Error::from)?;

                std::fs::remove_file(&source)
                    .map_err(anyhow::Error::from)?;

                println!("{}", Paint::green("✓").bold());
            }

            Action::Delete => {
                print!("{} {} ", Paint::red("✕").bold(), source.display());

                std::fs::remove_file(&source)
                    .map_err(anyhow::Error::from)?;

                println!("{}", Paint::green("✓").bold());
            }

            Action::Ignore(_) => {}
        }
    }

    // TODO: Continue after error
    // TODO: Update changeset with moved / deleted files
    // TODO: Clean empty parent directories

    return Ok(());
}

pub fn subcommand() -> App<'static, 'static> {
    return SubCommand::with_name("execute")
        .about("Executes the change set")
        .alias("exec")
        .arg(Arg::with_name("target")
            .short("t")
            .long("target")
            .value_name("DIR")
            .help("The target directory to move files to")
            .takes_value(true)
            .index(1)
            .required(true));
}
