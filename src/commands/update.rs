use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::{ArgMatches, SubCommand, App};
use itertools::{EitherOrBoth, Itertools};
use yansi::Paint;

use crate::{ProgramError, scan_tree};
use crate::changeset::{Action, ChangeSet, Workspace};

pub fn run(workspace: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
    let workspace = Workspace::open(workspace)
        .ok_or_else(|| ProgramError::NotInitialized)?;

    let changeset = workspace.import()?;
    let changeset = changeset.clean()
        .ok_or_else(|| ProgramError::NotClean)?;

    // Collect the current filesystem tree
    let tree = scan_tree(changeset.path()).collect::<BTreeSet<_>>();

    // Get bi-directional difference to determine additions and deletions
    let (workspace, records) = changeset.split();
    let records = Itertools::merge_join_by(tree.into_iter(), records,
                                           |a, (b, _)| PathBuf::cmp(a, b))
        .filter_map(|difference| {
            match difference {
                EitherOrBoth::Left(path) => {
                    println!("{} {}", Paint::green("+").bold(), path.display());
                    return Some((path.clone(), Action::Ignore(path.display().to_string())));
                }
                EitherOrBoth::Right((path, action)) => {
                    println!("{} {}", Paint::green("-").bold(), path.display());
                    return None;
                }
                EitherOrBoth::Both(_, (path, action)) => {
                    return Some((path, action));
                }
            }
        })
        .collect();

    let changeset = ChangeSet::create(workspace, records);
    changeset.export()?;

    return Ok(());
}

pub fn subcommand() -> App<'static, 'static> {
    return SubCommand::with_name("update")
        .alias("refresh")
        .about("Refresh the input list and updates the change set");
}