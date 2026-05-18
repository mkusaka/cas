use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use jwalk::WalkDir;

const CLAUDE_FILE: &str = "CLAUDE.md";
const AGENTS_FILE: &str = "AGENTS.md";
const CLAUDE_SKILLS_DIR: &str = ".claude/skills";
const AGENTS_DIR: &str = ".agents";
const AGENTS_SKILLS_DIR: &str = ".agents/skills";
const AGENTS_SKILLS_TARGET: &str = "../.claude/skills";

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
    sync_agents_file(dir, report)?;
    sync_skills_dir(dir, report)?;

    Ok(())
}

fn sync_agents_file(dir: &Path, report: &mut SyncReport) -> io::Result<()> {
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

fn sync_skills_dir(dir: &Path, report: &mut SyncReport) -> io::Result<()> {
    let claude_skills_path = dir.join(CLAUDE_SKILLS_DIR);
    if !claude_skills_path.try_exists()? || !claude_skills_path.is_dir() {
        return Ok(());
    }

    let agents_dir = dir.join(AGENTS_DIR);
    match fs::symlink_metadata(&agents_dir) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            report.summary.skipped += 1;
            report.skipped.push(dir.join(AGENTS_SKILLS_DIR));
            return Ok(());
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(&agents_dir)?;
        }
        Err(error) => return Err(error),
    }

    let agents_skills_path = dir.join(AGENTS_SKILLS_DIR);
    match fs::symlink_metadata(&agents_skills_path) {
        Ok(_) => {
            report.summary.skipped += 1;
            report.skipped.push(agents_skills_path);
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            create_skills_symlink(&agents_skills_path)?;
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

#[cfg(unix)]
fn create_skills_symlink(agents_skills_path: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(AGENTS_SKILLS_TARGET, agents_skills_path)
}

#[cfg(windows)]
fn create_skills_symlink(agents_skills_path: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(AGENTS_SKILLS_TARGET, agents_skills_path)
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
    fn creates_agents_skills_symlink_next_to_claude_skills_recursively() {
        let temp = TempDir::new("skills-recursive");
        let nested = temp.path().join("nested");
        let root_claude_skills = temp.path().join(CLAUDE_SKILLS_DIR);
        let nested_claude_skills = nested.join(CLAUDE_SKILLS_DIR);
        fs::create_dir_all(&root_claude_skills).expect("root .claude/skills should be created");
        fs::create_dir_all(&nested_claude_skills).expect("nested .claude/skills should be created");
        fs::write(root_claude_skills.join("SKILL.md"), "root")
            .expect("root skill should be written");
        fs::write(nested_claude_skills.join("SKILL.md"), "nested")
            .expect("nested skill should be written");

        let report = sync_agents(temp.path()).expect("sync should succeed");

        assert_eq!(report.summary.created, 2);
        assert_eq!(report.summary.skipped, 0);
        assert!(report.skipped.is_empty());
        assert_eq!(
            fs::read_link(temp.path().join(AGENTS_SKILLS_DIR))
                .expect("root .agents/skills should be a symlink"),
            PathBuf::from(AGENTS_SKILLS_TARGET)
        );
        assert_eq!(
            fs::read_link(nested.join(AGENTS_SKILLS_DIR))
                .expect("nested .agents/skills should be a symlink"),
            PathBuf::from(AGENTS_SKILLS_TARGET)
        );
    }

    #[test]
    fn skips_existing_agents_skills_without_overwriting_it() {
        let temp = TempDir::new("skills-existing");
        let claude_skills = temp.path().join(CLAUDE_SKILLS_DIR);
        let agents_skills = temp.path().join(AGENTS_SKILLS_DIR);
        fs::create_dir_all(&claude_skills).expect(".claude/skills should be created");
        fs::create_dir_all(&agents_skills).expect(".agents/skills should be created");
        fs::write(agents_skills.join("SKILL.md"), "existing")
            .expect("existing skill should be written");

        let report = sync_agents(temp.path()).expect("sync should succeed");

        assert_eq!(report.summary.created, 0);
        assert_eq!(report.summary.skipped, 1);
        assert_eq!(report.skipped, vec![agents_skills.clone()]);
        assert_eq!(
            fs::read_to_string(agents_skills.join("SKILL.md"))
                .expect("existing skill should remain readable"),
            "existing"
        );
    }

    #[test]
    fn skips_agents_skills_when_agents_path_is_not_a_directory() {
        let temp = TempDir::new("skills-agents-file");
        let claude_skills = temp.path().join(CLAUDE_SKILLS_DIR);
        fs::create_dir_all(&claude_skills).expect(".claude/skills should be created");
        fs::write(temp.path().join(AGENTS_DIR), "existing")
            .expect(".agents file should be written");

        let report = sync_agents(temp.path()).expect("sync should succeed");

        assert_eq!(report.summary.created, 0);
        assert_eq!(report.summary.skipped, 1);
        assert_eq!(report.skipped, vec![temp.path().join(AGENTS_SKILLS_DIR)]);
        assert_eq!(
            fs::read_to_string(temp.path().join(AGENTS_DIR))
                .expect(".agents file should remain readable"),
            "existing"
        );
    }

    #[cfg(unix)]
    #[test]
    fn skips_agents_skills_when_agents_path_is_a_symlink() {
        let temp = TempDir::new("skills-agents-symlink");
        let claude_skills = temp.path().join(CLAUDE_SKILLS_DIR);
        fs::create_dir_all(&claude_skills).expect(".claude/skills should be created");
        std::os::unix::fs::symlink(CLAUDE_SKILLS_DIR, temp.path().join(AGENTS_DIR))
            .expect(".agents symlink should be created");

        let report = sync_agents(temp.path()).expect("sync should succeed");

        assert_eq!(report.summary.created, 0);
        assert_eq!(report.summary.skipped, 1);
        assert_eq!(report.skipped, vec![temp.path().join(AGENTS_SKILLS_DIR)]);
        assert!(
            !claude_skills.join("skills").exists(),
            "sync should not create nested skills through the .agents symlink"
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
