use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process;

use cas::sync_agents;

fn main() {
    process::exit(match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    });
}

fn run() -> Result<i32, String> {
    let mut args = env::args_os();
    let program = args
        .next()
        .and_then(|arg| PathBuf::from(arg).file_name().map(OsStr::to_owned))
        .and_then(|arg| arg.into_string().ok())
        .unwrap_or_else(|| "cas".to_string());

    let Some(root) = args.next() else {
        eprintln!("{}", usage(&program));
        return Ok(2);
    };

    if root == OsStr::new("-h") || root == OsStr::new("--help") {
        println!("{}", usage(&program));
        return Ok(0);
    }

    if args.next().is_some() {
        eprintln!("{}", usage(&program));
        return Ok(2);
    }

    let report = sync_agents(PathBuf::from(root)).map_err(|error| error.to_string())?;

    println!(
        "created: {}, skipped: {}",
        report.summary.created, report.summary.skipped
    );

    for path in &report.skipped {
        println!("skipped: {}", path.display());
    }

    Ok(0)
}

fn usage(program: &str) -> String {
    format!(
        "Usage: {program} <directory>\n\nRecursively create AGENTS.md symlinks pointing to CLAUDE.md."
    )
}
