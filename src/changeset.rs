use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::{Path, PathBuf};
use std::io::Write;

use anyhow::Result;
use std::collections::BTreeMap;
use std::iter::FromIterator;

#[derive(Debug, Clone)]
pub enum Action {
    Move(PathBuf),
    Delete,
    Ignore(String),
}

impl<S> From<S> for Action
    where S: AsRef<str> {
    fn from(s: S) -> Self {
        let s = s.as_ref();
        if s.trim().is_empty() {
            return Action::Delete;
        }

        if s.starts_with(char::is_whitespace) {
            return Action::Ignore(s.trim().to_string());
        }

        return Action::Move(PathBuf::from(s));
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Action::Move(target) => write!(f, "{}", target.display()),
            Action::Delete => write!(f, ""),
            Action::Ignore(comment) => write!(f, " {}", comment),
        };
    }
}

#[derive(Debug, Clone)]
pub struct ChangeSet {
    workspace: Workspace,

    records: BTreeMap<PathBuf, Action>,
}

impl ChangeSet {
    pub fn create(workspace: Workspace,
                  records: BTreeMap<PathBuf, Action>) -> Self {
        return Self {
            workspace,
            records,
        };
    }

    pub fn export(&self) -> Result<()> {
        let mut sources = File::create(self.workspace.sources_path())?;
        let mut targets = File::create(self.workspace.targets_path())?;

        for (source, target) in self.records.iter() {
            writeln!(sources, "{}", source.display())?;
            writeln!(targets, "{}", target)?;
        }

        return Ok(());
    }

    pub fn workspace(&self) -> &Workspace {
        return &self.workspace;
    }

    pub fn path(&self) -> &Path {
        return self.workspace.path();
    }

    pub fn records(&self) -> &BTreeMap<PathBuf, Action> {
        return &self.records;
    }

    pub fn split(self) -> (Workspace, BTreeMap<PathBuf, Action>) {
        return (self.workspace, self.records);
    }
}

impl IntoIterator for ChangeSet {
    type Item = (PathBuf, Action);
    type IntoIter = impl Iterator<Item=Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        return self.records.into_iter();
    }
}

// impl FromIterator<(PathBuf, Action)> for ChangeSet {
//     fn from_iter<T: IntoIterator<Item=(PathBuf, Action)>>(iter: T) -> Self {
//         return ChangeSet {
//             records: iter.into_iter().collect(),
//         };
//     }
// }

#[derive(Debug, Clone)]
pub struct ChangeSetImport {
    workspace: Workspace,

    records: BTreeMap<PathBuf, Action>,

    unmapped_sources: Vec<PathBuf>,
    unmapped_targets: Vec<Action>,
}

impl ChangeSetImport {
    pub fn empty(workspace: Workspace) -> Self {
        return Self {
            workspace,
            records: BTreeMap::new(),
            unmapped_sources: Vec::new(),
            unmapped_targets: Vec::new(),
        };
    }

    pub fn import(workingdir: Workspace) -> Result<Self> {
        let mut sources = BufReader::new(File::open(workingdir.sources_path())?).lines()
            .map::<Result<_>, _>(|line| Ok(PathBuf::from(line?)));
        let mut targets = BufReader::new(File::open(workingdir.targets_path())?).lines()
            .map::<Result<_>, _>(|line| Ok(Action::from(line?)));

        let mut result = Self::empty(workingdir);
        loop {
            match (sources.next(), targets.next()) {
                (Some(source), Some(target)) => {
                    result.records.insert(source?, target?);
                }
                (Some(source), None) => {
                    result.unmapped_sources.push(source?);
                }
                (None, Some(target)) => {
                    result.unmapped_targets.push(target?);
                }
                (None, None) => {
                    break;
                }
            }
        }

        return Ok(result);
    }

    pub fn workspace(&self) -> &Workspace {
        return &self.workspace;
    }

    pub fn path(&self) -> &Path {
        return self.workspace.path();
    }

    pub fn records(&self) -> &BTreeMap<PathBuf, Action> {
        return &self.records;
    }

    pub fn records_mut(&mut self) -> &mut BTreeMap<PathBuf, Action> {
        return &mut self.records;
    }

    pub fn is_clean(&self) -> bool {
        return self.unmapped_sources.is_empty() && self.unmapped_targets.is_empty();
    }

    pub fn clean(self) -> Option<ChangeSet> {
        if self.is_clean() {
            return Some(ChangeSet {
                workspace: self.workspace,
                records: self.records
            });
        } else {
            return None;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    path: PathBuf,
}

impl Workspace {
    pub fn at(path: impl AsRef<Path>) -> Self {
        return Self {
            path: path.as_ref().to_path_buf(),
        };
    }

    pub fn open(path: impl AsRef<Path>) -> Option<Self> {
        let workspace = Self::at(path);

        let sources = workspace.sources_path();
        let targets = workspace.targets_path();

        if sources.is_file() && targets.is_file() {
            return Some(workspace);
        } else {
            return None;
        }
    }

    pub fn import(self) -> Result<ChangeSetImport> {
        return ChangeSetImport::import(self);
    }

    pub fn path(&self) -> &Path {
        return &self.path;
    }

    pub fn sources_path(&self) -> PathBuf {
        return self.path.join(".mmv.sources");
    }

    pub fn targets_path(&self) -> PathBuf {
        return self.path.join(".mmv.targets");
    }
}

