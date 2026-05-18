# cas

`cas` recursively creates `AGENTS.md` symlinks next to existing `CLAUDE.md`
files.

It is intended for repositories that already keep agent instructions in
`CLAUDE.md` and want Codex-compatible `AGENTS.md` files without duplicating
content.

## Behavior

- Scans the target directory recursively.
- Creates `AGENTS.md -> CLAUDE.md` in each directory that contains `CLAUDE.md`.
- Skips a directory when `AGENTS.md` already exists.
- Never overwrites existing files or symlinks.
- Includes hidden directories in the scan.

## Usage

```sh
cas <directory>
```

Example:

```sh
cas .
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
`mkusaka/homebrew-tap`. The workflow requires the `HOMEBREW_TAP_TOKEN`
repository secret.

```sh
git tag vX.Y.Z
git push origin vX.Y.Z
```
