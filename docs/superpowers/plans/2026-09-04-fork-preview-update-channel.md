# Fork Preview Update Channel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every push to the fork's `master` publishes preview binaries and a manifest, and the updater can be pointed at that manifest through config.

**Architecture:** A new GitHub Actions workflow on the fork mirrors upstream's preview pipeline (five-target build, prerelease, committed `distribution/preview.json`) using only the workflow token. The updater gains an optional `update.preview_manifest_url` config key resolved by one function shared with remote attach.

**Tech Stack:** GitHub Actions, Python (`scripts/preview.py`), Rust config/updater.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-09-04-fork-preview-update-channel-design.md`.
- Leave `.github/workflows/preview.yml` untouched.
- New config keys must be added to `docs/next/website/src/data/config-reference.json`.
- Build/test with the scratchpad cargo wrapper; `cargo test --locked --bin herdr <filter>`.

---

### Task 1: `scripts/preview.py` derives `--repo` from `GITHUB_REPOSITORY`

- [ ] Add `import os`; change both `--repo` defaults to `os.environ.get("GITHUB_REPOSITORY", "herdrdev/herdr")`.
- [ ] Run `python3 -m unittest scripts.test_preview` and confirm it passes.
- [ ] Commit `build(preview): derive release repo from GITHUB_REPOSITORY`.

### Task 2: `update.preview_manifest_url` config key and resolver

- [ ] Test in `src/config/model.rs`: default `None`, TOML parse of the key.
- [ ] `UpdateConfig`: drop `Copy`, add `pub preview_manifest_url: Option<String>` with doc comment, default `None`.
- [ ] Test in `src/update.rs`: `preview_manifest_url` returns the override when set and non-blank, else `https://herdr.dev/preview.json`.
- [ ] `src/update.rs`: `pub(crate) fn preview_manifest_url(update: &crate::config::UpdateConfig) -> String`; `fetch_preview_manifest` loads config and uses it; remove the constant.
- [ ] `src/remote/attach.rs`: replace the preview constant use with the resolver.
- [ ] `src/main.rs` template comment and `config-reference.json` entry; run `python3 scripts/config_reference_check.py`.
- [ ] `cargo test --locked --bin herdr -- preview_manifest_url update::tests config::model`; clippy; commit `feat(update): configurable preview manifest URL`.

### Task 3: `.github/workflows/fork-preview.yml`

- [ ] Write the workflow per the spec: push to master + dispatch, gated to `potable-anarchy/herdr`, preflight without checks, same build matrix, publish with workflow token and `--repo "$GITHUB_REPOSITORY"`, manifest commit as `github-actions[bot]`, prune to 30.
- [ ] Validate YAML with `python3 -c "import yaml"` if available, else `ruby -ryaml`; commit `ci: fork preview workflow`.

### Task 4: Ship and verify

- [ ] Push branch, fast-forward `master`, push `master`; watch the workflow with `gh run watch`.
- [ ] Confirm a `preview-*` prerelease with five assets and a manifest commit on `master`.
- [ ] Report the config lines and `herdr update` steps to the user.
