# cas

`cas` recursively syncs Claude agent files into Codex-compatible paths.

It is intended for repositories that already keep agent instructions in
`CLAUDE.md` and `.claude/skills`, and want Codex-compatible paths without
duplicating content.

## Behavior

- Scans the target directory recursively.
- Creates `AGENTS.md -> CLAUDE.md` in each directory that contains `CLAUDE.md`.
- Creates `.agents/skills -> ../.claude/skills` in each directory that contains
  `.claude/skills`.
- Skips a target when `AGENTS.md` or `.agents/skills` already exists.
- Skips `.agents/skills` when `.agents` already exists but is not a plain
  directory.
- Never overwrites existing files or symlinks.
- Includes hidden directories in the scan.

## Usage

```sh
cas [directory]
```

If `directory` is omitted, `cas` scans the current directory.

Example:

```sh
cas
```

Output:

```text
created: 3, skipped: 1
skipped: ./docs/AGENTS.md
```

## Install

Install via Homebrew:

```sh
brew tap mkusaka/tap
brew install mkusaka/tap/cas
```

Install from a local checkout:

```sh
cargo install --path . --force
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
```

## Release

Pushing a `v*` tag runs the release workflow. It validates the tag against
`Cargo.toml`, creates a GitHub Release, builds Homebrew bottles for Apple
Silicon and Intel Macs on macOS 15 and 26, and updates
`mkusaka/homebrew-tap` when the `HOMEBREW_TAP_TOKEN` repository secret is
configured.

```sh
git tag vX.Y.Z
git push origin vX.Y.Z
```
