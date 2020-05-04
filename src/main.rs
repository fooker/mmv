use std::path::{Path, PathBuf};

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use walkdir::WalkDir;

use crate::changeset::{Action, ChangeSet, Workspace};
use std::process::Command;
use itertools::{Itertools, EitherOrBoth};
use std::collections::BTreeSet;
use yansi::Paint;

mod changeset;

#[derive(thiserror::Error, Debug)]
pub enum ProgramError {
    #[error("Not initialized")]
    NotInitialized,

    #[error("Not clean")]
    NotClean,

    #[error(transparent)]
    InternalError(#[from] anyhow::Error),
}

fn scan_tree<'a>(workdir: impl AsRef<Path> + 'a) -> impl Iterator<Item=PathBuf> + 'a {
    return WalkDir::new(workdir.as_ref())
        .min_depth(1)
        .sort_by(|a, b| Ord::cmp(a.file_name(), b.file_name()))
        .into_iter()
        .filter_map(|entry| {
            match entry {
                Ok(entry) => {
                    return Some(entry);
                }
                Err(err) => {
                    eprintln!("{}", err);
                    return None;
                }
            }
        })
        .filter_map(move |entry| {
            // Only list files
            if !entry.file_type().is_file() {
                return None;
            }

            // Ignore mmv status files
            if entry.file_name().to_str()
                .map(|s| s.starts_with(".mmv"))
                .unwrap_or(false) {
                return None;
            }

            // The path is absolute. The common prefix is removed to make the path relative to the
            // working directory
            let path = entry.path().strip_prefix(workdir.as_ref())
                .expect("Path not relative");

            return Some(path.to_path_buf());
        });
}

fn init(workspace: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
    let workspace = Workspace::at(workspace);

    let records = scan_tree(workspace.path())
        .map(|path| {
            return (path.clone(), Action::Ignore(path.display().to_string()));
        })
        .collect();

    let changeset = ChangeSet::create(workspace, records);
    changeset.export()?;

    eprintln!("{}", Paint::green("Initialized"));

    return Ok(());
}

fn update(workspace: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
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
                    eprintln!("{} Added {}", Paint::green("+"), path.display());
                    return Some((path.clone(), Action::Ignore(path.display().to_string())));
                }
                EitherOrBoth::Right((path, action)) => {
                    eprintln!("{} Removed {}", Paint::green("-"), path.display());
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

fn status(workingdir: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
    let workingdir = Workspace::open(workingdir)
        .ok_or_else(|| ProgramError::NotInitialized)?;

    let changeset = workingdir.import()?;

    if changeset.is_clean() {
        eprintln!("{}", Paint::green("Workspace is clean").bold());
    } else {
        eprintln!("{}", Paint::red("Workspace is not clean").bold());
    }

    return Ok(());
}

fn edit(workingdir: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
    let workingdir = Workspace::open(workingdir)
        .ok_or_else(|| ProgramError::NotInitialized)?;

    let changeset = workingdir.import()?;

    let status = Command::new("vim")
        .args(&[
            "-O",
            &format!("{}", changeset.workspace().sources_path().display()),
            &format!("{}", changeset.workspace().targets_path().display()),
            "-c", "setlocal readonly | setlocal nobuflisted | windo set scb",
        ])
        .status()
        .map_err(anyhow::Error::from)?;

    // TODO: Print brief status afterwards

    return Ok(());
}

fn execute(workingdir: &Path, matches: &ArgMatches) -> Result<(), ProgramError> {
    let target = matches.value_of("target").expect("No target");
    let target = Path::new(target);

    let workingdir = Workspace::open(workingdir)
        .ok_or_else(|| ProgramError::NotInitialized)?;

    let changeset = workingdir.import()?;
    let changeset = changeset.clean()
        .ok_or_else(|| ProgramError::NotClean)?;

    // Execute actions in two steps: first, copy files which should be moved, second delete files
    // either because they are moved or marked for deletion
    for (source, action) in changeset.records().iter() {
        let source = changeset.path().join(&source);

        match action {
            Action::Move(path) => {
                let target = target.join(path);

                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(anyhow::Error::from)?;
                }

                reflink::reflink_or_copy(&source, &target)
                    .map_err(anyhow::Error::from)?;

                std::fs::remove_file(&source)
                    .map_err(anyhow::Error::from)?;

                eprintln!("{} {} ({})", Paint::cyan("➤"), source.display(), target.display());
            }

            Action::Delete => {
                std::fs::remove_file(&source)
                    .map_err(anyhow::Error::from)?;

                eprintln!("{} {}", Paint::red("✕"), source.display());
            }

            Action::Ignore(_) => {
            }
        }
    }

    // TODO: Continue after error
    // TODO: Update changeset with moved / deleted files
    // TODO: Clean empty parent directories

    return Ok(());
}

fn main() -> Result<(), anyhow::Error> {
    let matches = App::new("mmv")
        .about("Mass Move files with interactive renaming")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Dustin Frisch <fooker@lab.sh>")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(Arg::with_name("source")
            .short("s")
            .long("source")
            .value_name("DIR")
            .help("The source directory to work on")
            .takes_value(true)
            .required(false))
        // .arg(Arg::with_name("verbose")
        //     .short("v")
        //     .long("verbose")
        //     .help("Enables detailed output"))
        .subcommand(SubCommand::with_name("init")
            .alias("reset")
            .about("Initialize to a clean state (drops your change set)"))
        .subcommand(SubCommand::with_name("update")
            .alias("refresh")
            .about("Refresh the input list and updates the change set"))
        .subcommand(SubCommand::with_name("status")
            .about("Prints the current status"))
        .subcommand(SubCommand::with_name("edit")
            .about("Opens an editor for the change set"))
        .subcommand(SubCommand::with_name("execute")
            .about("Executes the change set")
            .alias("exec")
            .arg(Arg::with_name("target")
                .short("t")
                .long("target")
                .value_name("DIR")
                .help("The target directory to move files to")
                .takes_value(true)
                .index(1)
                .required(true)))
        .get_matches();

    let workspace = matches.value_of("source")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("No current path"));

    let result = match matches.subcommand() {
        ("init", Some(matches)) => init(&workspace, matches),
        ("update", Some(matches)) => update(&workspace, matches),
        ("status", Some(matches)) => status(&workspace, matches),
        ("edit", Some(matches)) => edit(&workspace, matches),
        ("execute", Some(matches)) => execute(&workspace, matches),
        _ => unreachable!()
    };

    match result {
        Ok(()) => {
            std::process::exit(exitcode::OK);
        }

        Err(ProgramError::NotInitialized) => {
            eprintln!("Not initialized - use mmv init to do so");
            std::process::exit(exitcode::DATAERR);
        }
        Err(ProgramError::NotClean) => {
            eprintln!("Not clean - use mvv edit ro correct your changeset");
            std::process::exit(exitcode::DATAERR);
        }

        Err(ProgramError::InternalError(err)) => {
            return Err(err);
        }
    }
}
