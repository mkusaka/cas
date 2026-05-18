use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use jwalk::WalkDir;

const CLAUDE_FILE: &str = "CLAUDE.md";
const AGENTS_FILE: &str = "AGENTS.md";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncSummary {
    pub created: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub summary: SyncSummary,
    pub skipped: Vec<PathBuf>,
}

pub fn sync_agents(root: impl AsRef<Path>) -> io::Result<SyncReport> {
    let root = root.as_ref();
    let metadata = fs::metadata(root)?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a directory", root.display()),
        ));
    }

    let mut report = SyncReport {
        summary: SyncSummary::default(),
        skipped: Vec::new(),
    };
    for entry in WalkDir::new(root).skip_hidden(false).sort(true) {
        let entry = entry.map_err(|error| io::Error::other(error.to_string()))?;
        if entry.file_type().is_dir() {
            sync_directory(&entry.path(), &mut report)?;
        }
    }

    Ok(report)
}

fn sync_directory(dir: &Path, report: &mut SyncReport) -> io::Result<()> {
    let claude_path = dir.join(CLAUDE_FILE);
    if !claude_path.try_exists()? {
        return Ok(());
    }

    let agents_path = dir.join(AGENTS_FILE);
    match fs::symlink_metadata(&agents_path) {
        Ok(_) => {
            report.summary.skipped += 1;
            report.skipped.push(agents_path);
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            create_agents_symlink(&agents_path)?;
            report.summary.created += 1;
        }
        Err(error) => return Err(error),
    }

    Ok(())
}

#[cfg(unix)]
fn create_agents_symlink(agents_path: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(CLAUDE_FILE, agents_path)
}

#[cfg(windows)]
fn create_agents_symlink(agents_path: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_file(CLAUDE_FILE, agents_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            let path = env::temp_dir().join(format!("cas-{name}-{}-{nanos}", process::id()));
            fs::create_dir(&path).expect("temp dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn creates_agents_symlinks_next_to_claude_files_recursively() {
        let temp = TempDir::new("recursive");
        let nested = temp.path().join("nested").join("deeper");
        let hidden = temp.path().join(".codex");
        fs::create_dir_all(&nested).expect("nested dir should be created");
        fs::create_dir_all(&hidden).expect("hidden dir should be created");
        fs::write(temp.path().join(CLAUDE_FILE), "root").expect("root CLAUDE.md should be written");
        fs::write(nested.join(CLAUDE_FILE), "nested").expect("nested CLAUDE.md should be written");
        fs::write(hidden.join(CLAUDE_FILE), "hidden").expect("hidden CLAUDE.md should be written");

        let report = sync_agents(temp.path()).expect("sync should succeed");

        assert_eq!(report.summary.created, 3);
        assert_eq!(report.summary.skipped, 0);
        assert!(report.skipped.is_empty());
        assert_eq!(
            fs::read_link(temp.path().join(AGENTS_FILE))
                .expect("root AGENTS.md should be a symlink"),
            PathBuf::from(CLAUDE_FILE)
        );
        assert_eq!(
            fs::read_link(nested.join(AGENTS_FILE)).expect("nested AGENTS.md should be a symlink"),
            PathBuf::from(CLAUDE_FILE)
        );
        assert_eq!(
            fs::read_link(hidden.join(AGENTS_FILE)).expect("hidden AGENTS.md should be a symlink"),
            PathBuf::from(CLAUDE_FILE)
        );
    }

    #[test]
    fn skips_existing_correct_symlink() {
        let temp = TempDir::new("unchanged");
        fs::write(temp.path().join(CLAUDE_FILE), "root").expect("CLAUDE.md should be written");
        create_agents_symlink(&temp.path().join(AGENTS_FILE)).expect("symlink should be created");

        let report = sync_agents(temp.path()).expect("sync should succeed");

        assert_eq!(report.summary.created, 0);
        assert_eq!(report.summary.skipped, 1);
        assert_eq!(report.skipped, vec![temp.path().join(AGENTS_FILE)]);
    }

    #[test]
    fn skips_existing_agents_file_without_overwriting_it() {
        let temp = TempDir::new("conflict-file");
        let agents_path = temp.path().join(AGENTS_FILE);
        fs::write(temp.path().join(CLAUDE_FILE), "root").expect("CLAUDE.md should be written");
        fs::write(&agents_path, "existing").expect("AGENTS.md should be written");

        let report = sync_agents(temp.path()).expect("sync should succeed");

        assert_eq!(report.summary.created, 0);
        assert_eq!(report.summary.skipped, 1);
        assert_eq!(report.skipped, vec![agents_path.clone()]);
        assert_eq!(
            fs::read_to_string(agents_path).expect("AGENTS.md should remain readable"),
            "existing"
        );
    }

    #[test]
    fn skips_symlink_to_another_target() {
        let temp = TempDir::new("conflict-symlink");
        fs::write(temp.path().join(CLAUDE_FILE), "root").expect("CLAUDE.md should be written");
        fs::write(temp.path().join("OTHER.md"), "other").expect("OTHER.md should be written");
        symlink_file("OTHER.md", temp.path().join(AGENTS_FILE)).expect("symlink should be created");

        let report = sync_agents(temp.path()).expect("sync should succeed");

        assert_eq!(report.summary.created, 0);
        assert_eq!(report.summary.skipped, 1);
        assert_eq!(report.skipped, vec![temp.path().join(AGENTS_FILE)]);
    }

    #[cfg(unix)]
    fn symlink_file(original: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
        std::os::unix::fs::symlink(original, link)
    }

    #[cfg(windows)]
    fn symlink_file(original: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
        std::os::windows::fs::symlink_file(original, link)
    }
}
