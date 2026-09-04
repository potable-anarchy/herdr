# Fork-hosted preview update channel

Date: 2026-09-04
Status: approved design, branch `feature/fork-preview-channel` on `potable-anarchy/herdr`

## Context

Herdr's updater polls compile-time manifest URLs on herdr.dev. The preview
channel is produced by `.github/workflows/preview.yml` (manual dispatch,
gated to `herdrdev/herdr`), which builds five targets, publishes a GitHub
prerelease, and commits `distribution/preview.json` to master; the private
website then serves it. The fork cannot reach any of that, so `herdr update`
on a fork build would install upstream's binary over it.

Goal: every push to the fork's `master` produces installable preview builds,
and the updater can be pointed at the fork's manifest.

## Workflow

New file `.github/workflows/fork-preview.yml`, adapted from `preview.yml`;
`preview.yml` is left untouched.

- Triggers: `push` to `master` and `workflow_dispatch`. Every job is gated
  with `if: github.repository == 'potable-anarchy/herdr'`.
- `preflight`: checks out `master`, computes `commit`, `short_sha`,
  `build_id` (`<commit date>-<12-char sha>`), `tag` (`preview-<build_id>`),
  `built_at`, `base_version` (Cargo.toml), `protocol`
  (`PROTOCOL_VERSION`). Skips publishing when `distribution/preview.json`
  already names this commit. No `just check` gate.
- `build`: the same five-target matrix, runners, toolchain steps, zig
  install, packaging, static-link checks, `.sha256` files, `BUILD_INFO.txt`,
  and artifact upload as upstream, with `HERDR_BUILD_CHANNEL=preview`,
  `HERDR_BUILD_ID`, and `HERDR_BUILD_COMMIT` set at job level.
- `publish` (`permissions: contents: write`): downloads artifacts, writes
  `preview-sha256.json`, generates notes and the manifest with
  `python3 scripts/preview.py ... --repo "$GITHUB_REPOSITORY"`, creates the
  prerelease with `softprops/action-gh-release` and the workflow token,
  un-drafts it with `gh release edit`, commits `distribution/preview.json`
  to `master` as `github-actions[bot]` using the workflow token (with the
  same three-attempt rebase loop), and prunes to the newest 30
  `preview-*` prereleases. The docs snapshot and issue-labeling steps are
  omitted.
- Pushes made with the workflow token do not trigger workflows, so the
  manifest commit does not start another run.

## Script

`scripts/preview.py`: `--repo` defaults to
`os.environ.get("GITHUB_REPOSITORY", "herdrdev/herdr")` for both `notes`
and `manifest`. Existing tests pass `repo` explicitly and are unaffected.

## Updater

- `UpdateConfig` gains `preview_manifest_url: Option<String>` (default
  `None`). The struct loses `Copy`; call sites take `.clone()` or a reference.
- `crate::update::preview_manifest_url(config: &UpdateConfig) -> String`
  returns the override when set and non-empty, otherwise
  `https://herdr.dev/preview.json`.
- `fetch_preview_manifest` in `src/update.rs` and the preview asset lookup
  in `src/remote/attach.rs` both use that function, replacing the duplicated
  constant.
- Install-decision logic is unchanged.
- Config template comment in `src/main.rs`, entry in
  `docs/next/website/src/data/config-reference.json`.

## User configuration

```toml
[update]
channel = "preview"
preview_manifest_url = "https://raw.githubusercontent.com/potable-anarchy/herdr/master/distribution/preview.json"
```

## Testing

- Config: default is `None`; TOML parse of the key.
- Resolver: override wins; empty or absent falls back to the upstream URL.
- `scripts/test_preview.py` continues to pass.
- Workflow: after pushing, dispatch or push to `master` on the fork, confirm
  a `preview-<build_id>` prerelease with five assets and a manifest commit,
  then run `herdr update` outside a Herdr session.

## Out of scope

- Exposing `HERDR_BUILD_COMMIT` through `build_info`.
- Changing the default channel for non-Windows preview builds.
- `distribution/install.sh` preview support.
