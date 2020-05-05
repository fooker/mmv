use std::path::{Path, PathBuf};

use clap::{App, AppSettings, Arg};
use walkdir::WalkDir;
use yansi::Paint;

mod changeset;
mod commands;

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
        .subcommand(commands::init::subcommand())
        .subcommand(commands::update::subcommand())
        .subcommand(commands::status::subcommand())
        .subcommand(commands::edit::subcommand())
        .subcommand(commands::execute::subcommand())
        .get_matches();

    let workspace = matches.value_of("source")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("No current path"));

    let result = match matches.subcommand() {
        ("init", Some(matches)) => commands::init::run(&workspace, matches),
        ("update", Some(matches)) => commands::update::run(&workspace, matches),
        ("status", Some(matches)) => commands::status::run(&workspace, matches),
        ("edit", Some(matches)) => commands::edit::run(&workspace, matches),
        ("execute", Some(matches)) => commands::execute::run(&workspace, matches),
        _ => unreachable!()
    };

    match result {
        Ok(()) => {
            std::process::exit(exitcode::OK);
        }

        Err(ProgramError::NotInitialized) => {
            eprintln!("{} {}", Paint::red("Not initialized."), "Use mmv init to do so");
            std::process::exit(exitcode::DATAERR);
        }
        Err(ProgramError::NotClean) => {
            eprintln!("{} {}", Paint::red("Not clean."), "Use mvv edit ro correct your changeset");
            std::process::exit(exitcode::DATAERR);
        }

        Err(ProgramError::InternalError(err)) => {
            return Err(err);
        }
    }
}
