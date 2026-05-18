use std::fs;
use std::path::Path;

#[test]
fn homebrew_formula_template_has_required_release_placeholders() {
    let template = include_str!("../packaging/homebrew/cas.rb");

    assert!(template.contains("class Cas < Formula"));
    assert!(template.contains("system \"cargo\", \"install\", *std_cargo_args"));
    assert!(template.contains("assert_match \".claude/skills\""));

    for placeholder in [
        "__SOURCE_URL__",
        "__VERSION__",
        "__SOURCE_SHA256__",
        "__ROOT_URL__",
        "__ARM64_TAHOE_SHA256__",
        "__TAHOE_SHA256__",
        "__ARM64_SEQUOIA_SHA256__",
        "__SEQUOIA_SHA256__",
    ] {
        assert!(
            template.contains(placeholder),
            "missing placeholder {placeholder}"
        );
    }
}

#[test]
fn release_workflow_updates_homebrew_tap_for_cas() {
    let Some(workflow) = read_repo_file(".github/workflows/release.yml") else {
        return;
    };

    assert!(workflow.contains("echo \"formula_name=cas\""));
    assert!(workflow.contains("gh repo clone mkusaka/homebrew-tap homebrew-tap"));
    assert!(workflow.contains("cp packaging/homebrew/cas.rb \"$FORMULA_PATH\""));
    assert!(workflow.contains("git -C homebrew-tap add \"Formula/${FORMULA_NAME}.rb\""));
    assert!(workflow.contains("git -C homebrew-tap push origin HEAD:main"));

    for field in [
        "__SOURCE_URL__",
        "__VERSION__",
        "__SOURCE_SHA256__",
        "__ROOT_URL__",
        "__ARM64_TAHOE_SHA256__",
        "__TAHOE_SHA256__",
        "__ARM64_SEQUOIA_SHA256__",
        "__SEQUOIA_SHA256__",
    ] {
        assert!(
            workflow.contains(field),
            "missing formula replacement {field}"
        );
    }
}

fn read_repo_file(path: impl AsRef<Path>) -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    match fs::read_to_string(&path) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping repository-only fixture {}", path.display());
            None
        }
        Err(error) => panic!("failed to read {}: {error}", path.display()),
    }
}
