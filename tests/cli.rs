use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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
        let path = env::temp_dir().join(format!("cas-cli-{name}-{}-{nanos}", std::process::id()));
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
fn prints_help() {
    let output = run_cas(["--help"]);

    assert_success(&output);
    assert!(stdout(&output).contains("Usage: cas [directory]"));
    assert!(stdout(&output).contains("Recursively create AGENTS.md"));
    assert!(stdout(&output).contains(".claude/skills"));
    assert!(stdout(&output).contains("Defaults to the current directory"));
}

#[test]
fn defaults_to_current_directory() {
    let temp = TempDir::new("current-dir");
    fs::write(temp.path().join("CLAUDE.md"), "instructions").expect("CLAUDE.md should be written");

    let output = run_cas_in(temp.path(), std::iter::empty::<&str>());

    assert_success(&output);
    assert!(stdout(&output).contains("created: 1, skipped: 0"));
    assert_eq!(
        fs::read_link(temp.path().join("AGENTS.md")).expect("AGENTS.md should be a symlink"),
        PathBuf::from("CLAUDE.md")
    );
}

#[test]
fn creates_symlink_and_skips_existing_agents_on_second_run() {
    let temp = TempDir::new("sync");
    fs::write(temp.path().join("CLAUDE.md"), "instructions").expect("CLAUDE.md should be written");

    let first = run_cas([temp.path().as_os_str()]);
    assert_success(&first);
    assert!(stdout(&first).contains("created: 1, skipped: 0"));
    assert_eq!(
        fs::read_link(temp.path().join("AGENTS.md")).expect("AGENTS.md should be a symlink"),
        PathBuf::from("CLAUDE.md")
    );

    let second = run_cas([temp.path().as_os_str()]);
    assert_success(&second);
    assert!(stdout(&second).contains("created: 0, skipped: 1"));
    assert!(stdout(&second).contains("skipped:"));
}

#[test]
fn creates_agents_skills_symlink_and_skips_existing_on_second_run() {
    let temp = TempDir::new("skills-sync");
    let claude_skills = temp.path().join(".claude").join("skills");
    fs::create_dir_all(&claude_skills).expect(".claude/skills should be created");
    fs::write(claude_skills.join("SKILL.md"), "skill").expect("skill should be written");

    let first = run_cas([temp.path().as_os_str()]);
    assert_success(&first);
    assert!(stdout(&first).contains("created: 1, skipped: 0"));
    assert_eq!(
        fs::read_link(temp.path().join(".agents").join("skills"))
            .expect(".agents/skills should be a symlink"),
        PathBuf::from("../.claude/skills")
    );

    let second = run_cas([temp.path().as_os_str()]);
    assert_success(&second);
    assert!(stdout(&second).contains("created: 0, skipped: 1"));
    assert!(stdout(&second).contains("skipped:"));
}

fn run_cas<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    run_cas_command(args)
        .output()
        .expect("cas command should run")
}

fn run_cas_in<I, S>(current_dir: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut command = run_cas_command(args);
    command.current_dir(current_dir);
    command.output().expect("cas command should run")
}

fn run_cas_command<I, S>(args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut command = Command::new(env!("CARGO_BIN_EXE_cas"));
    command.args(args);
    command
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout(output),
        stderr(output)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
