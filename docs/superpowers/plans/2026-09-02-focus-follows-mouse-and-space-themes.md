# Focus Follows Mouse and Per-Space Themes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in `ui.focus_follows_mouse` setting that focuses the pane under the pointer, and an optional per-space theme override chosen from the space's right-click menu.

**Architecture:** Herdr is a Rust client/server TUI. The client shell (`src/client/shell/`) owns chrome rendering, hit-testing, mouse routing, and the settings overlay; it talks to the server through endpoint API methods (`crate::api::schema::Method`). The server (`src/app/`) owns `Workspace` state, persistence (`src/persist/`), and split-border rendering (`src/ui/panes.rs`). The two sides share a bincode wire protocol (`src/protocol/wire.rs`). Feature 1 is client-only plus config. Feature 2 adds a `theme: Option<String>` to workspaces on the server, ships it to the client in the snapshot, and both sides resolve a palette per space through the existing theme pipeline.

**Tech Stack:** Rust 1.96, ratatui, serde/serde_json/bincode, schemars, cargo-nextest, `just`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-09-02-focus-follows-mouse-and-space-themes-design.md`.
- Both features are opt-in; default behaviour must be byte-for-byte unchanged for users who do not enable them.
- Never write to the real config file or the real session file from tests. `save_settings_edit` writes to `crate::config::config_path()`; do not call `apply_settings_choice` on config-writing sections in tests.
- Do not touch anything outside the repo directory. Do not install anything. Do not run `herdr` from the fork against the user's running server.
- Unit tests live next to code in `#[cfg(test)] mod tests`. Client shell tests live in `src/client/shell/tests/*.rs`.
- Run a single test with `cargo nextest run --locked --bin herdr <filter>`. The full suite is `cargo nextest run --locked --bin herdr` (bin only; integration tests spawn real binaries and are slower, run them once at the end).
- Format with `cargo fmt --all` and lint with `cargo clippy --all-targets -- -D warnings` before every commit.
- Commit messages: conventional prefix (`feat:`, `test:`, `docs:`), body optional, and end with:
  ```
  Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01WovtrovHQPERykXs5mn2MU
  ```
- New config keys must be added to `docs/next/website/src/data/config-reference.json` or `python3 scripts/config_reference_check.py` fails.
- Adding an API method requires: `Method` variant, `api_method_name` arm, dispatch arm, `CLIENT_SHELL_METHODS` entry (sorted), a digest in `tests/fixtures/endpoint-method-shapes-v1.json`, regenerating `docs/next/api/herdr-api.schema.json`, and the socket-api docs table (three languages).
- Changing `ClientShellWorkspace` changes the bincode wire format: bump `PROTOCOL_VERSION` in `src/protocol/wire.rs` from 22 to 23.

---

## File Structure

Feature 1 (focus follows mouse):
- Modify `src/config/model.rs` — `UiConfig::focus_follows_mouse` field, default, test.
- Modify `src/config/write.rs` — `ConfigEdit::FocusFollowsMouse(bool)` and unit test.
- Modify `src/client/shell/state.rs` — `ClientShellConfig::focus_follows_mouse`, `ClientSettingsSection::Focus`.
- Modify `src/client/shell/config.rs` — projection in `from_config` and `apply_live_config`.
- Modify `src/client/shell/settings.rs` — section index, choice count, apply.
- Modify `src/client/shell/settings_overlay.rs` — render the Focus section.
- Modify `src/client/shell/mouse.rs` — settings immediate-apply set; the `Moved` arm.
- Modify `src/main.rs` — config template comment.
- Modify `docs/next/website/src/data/config-reference.json`.
- Create `src/client/shell/tests/focus_follows_mouse.rs` (register in `src/client/shell/tests/mod.rs`).

Feature 2 (per-space themes):
- Modify `src/workspace.rs` — `Workspace::theme`.
- Modify `src/persist/snapshot.rs`, `src/persist/restore.rs` — persist/restore `theme`.
- Modify `src/api/schema/workspaces.rs`, `src/api/schema.rs`, `src/api/server.rs`, `src/app/api.rs`, `src/app/api/workspaces.rs`, `src/server/client_commands.rs`, `tests/fixtures/endpoint-method-shapes-v1.json`, `docs/next/api/herdr-api.schema.json`, `docs/next/website/src/content/docs/{,ja/,zh-cn/}socket-api.mdx`, `src/logging.rs` — the `workspace.set_theme` method.
- Modify `src/app/state.rs`, `src/app/mod.rs`, `src/app/theme_sync.rs` — server palette cache and `palette_for_workspace`.
- Modify `src/ui/panes.rs` — borders and titles use the space palette.
- Modify `src/protocol/wire.rs`, `src/server/client_shell.rs` — `ClientShellWorkspace::theme`, protocol bump.
- Modify `src/client/shell/state.rs`, `src/client/shell/composition.rs`, `src/client/shell/render.rs`, `src/client/shell/tabs.rs`, `src/client/shell/sidebar.rs`, `src/client/shell/config.rs` — client palette map and rendering.
- Modify `src/client/shell/context_menu.rs`, `src/client/shell/settings.rs`, `src/client/shell/settings_overlay.rs`, `src/client/shell/mouse.rs` — the `Theme...` item and the targeted picker.
- Create `src/client/shell/tests/space_themes.rs` (register in `tests/mod.rs`).
- Test-only struct-literal updates: `src/client/shell/tests/{mod,chrome_context,mobile,agents_worktrees_notifications}.rs`, `src/protocol/wire.rs` test.

---

### Task 1: `ui.focus_follows_mouse` config key and config edit

**Files:**
- Modify: `src/config/model.rs:860` (field after `mouse_capture`), `src/config/model.rs:1107` (default), tests near `src/config/model.rs:1608`
- Modify: `src/config/write.rs`
- Modify: `src/main.rs:247-250` (template comment)
- Modify: `docs/next/website/src/data/config-reference.json:891-896`

**Interfaces:**
- Produces: `Config::ui.focus_follows_mouse: bool` (default `false`); `crate::config::ConfigEdit::FocusFollowsMouse(bool)`.

- [ ] **Step 1: Write the failing config test**

In `src/config/model.rs`, directly after the `mouse_capture_default_on_and_parse` test (around line 1618), add:

```rust
    #[test]
    fn focus_follows_mouse_default_off_and_parse() {
        let default_config = Config::default();
        assert!(!default_config.ui.focus_follows_mouse);

        let toml = r#"
[ui]
focus_follows_mouse = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.ui.focus_follows_mouse);
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo nextest run --locked --bin herdr focus_follows_mouse_default_off_and_parse`
Expected: compile error, `no field focus_follows_mouse`.

- [ ] **Step 3: Add the field and default**

In `src/config/model.rs`, after the `mouse_capture` field (line 860):

```rust
    /// Focus the pane under the mouse pointer without clicking. Default: false.
    pub focus_follows_mouse: bool,
```

In `impl Default for UiConfig` after `mouse_capture: true,` (line 1107):

```rust
            focus_follows_mouse: false,
```

- [ ] **Step 4: Write the failing config-edit test**

At the bottom of `src/config/write.rs` add:

```rust
#[cfg(test)]
mod tests {
    use super::ConfigEdit;

    #[test]
    fn focus_follows_mouse_edit_upserts_under_ui() {
        let written = ConfigEdit::FocusFollowsMouse(true).apply("");
        let parsed: toml::Value = toml::from_str(&written).unwrap();
        assert_eq!(
            parsed["ui"]["focus_follows_mouse"],
            toml::Value::Boolean(true)
        );

        let toggled = ConfigEdit::FocusFollowsMouse(false).apply(&written);
        let parsed: toml::Value = toml::from_str(&toggled).unwrap();
        assert_eq!(
            parsed["ui"]["focus_follows_mouse"],
            toml::Value::Boolean(false)
        );
    }
}
```

- [ ] **Step 5: Add the `ConfigEdit` variant**

In `src/config/write.rs`:

```rust
#[derive(Clone, Copy)]
pub(crate) enum ConfigEdit<'a> {
    Theme(&'a str),
    StatusIndicators(super::StatusIndicatorStyle),
    Sound(bool),
    ToastDelivery(super::ToastDelivery),
    FocusFollowsMouse(bool),
}
```

In `description`:

```rust
            Self::FocusFollowsMouse(_) => "focus setting",
```

In `apply`, before the `Self::ToastDelivery` arm:

```rust
            Self::FocusFollowsMouse(enabled) => {
                super::upsert_section_bool(content, "ui", "focus_follows_mouse", enabled)
            }
```

- [ ] **Step 6: Document the key**

In `src/main.rs`, after the `# mouse_capture = true` template line (line 250), add:

```
# Focus the pane under the mouse pointer without clicking, like a
# focus-follows-mouse window manager. Sidebar and tab bar still need a click.
# focus_follows_mouse = false
```

(Each line is inside the existing raw string; match the surrounding `# ` comment style exactly.)

In `docs/next/website/src/data/config-reference.json`, after the `ui.mouse_capture` object (ends line 896), add:

```json
        {
          "key": "ui.focus_follows_mouse",
          "type": "boolean",
          "default": "false",
          "description": "Focus the pane under the mouse pointer without clicking."
        },
```

- [ ] **Step 7: Run tests and the reference check**

Run: `cargo nextest run --locked --bin herdr focus_follows_mouse && python3 scripts/config_reference_check.py`
Expected: both tests PASS; the script exits 0 with no missing keys.

- [ ] **Step 8: Commit**

```bash
cargo fmt --all
git add src/config/model.rs src/config/write.rs src/main.rs docs/next/website/src/data/config-reference.json
git commit -m "feat(config): add ui.focus_follows_mouse key and config edit"
```

---

### Task 2: Client config projection and the `focus` settings section

**Files:**
- Modify: `src/client/shell/state.rs:100` (`ClientShellConfig`), `src/client/shell/state.rs:408-434` (`ClientSettingsSection`)
- Modify: `src/client/shell/config.rs:132` (`from_config`), `src/client/shell/config.rs:319` (`apply_live_config`)
- Modify: `src/client/shell/settings.rs:52-58`, `:100-107`, `:210-230`
- Modify: `src/client/shell/settings_overlay.rs:173-183`
- Modify: `src/client/shell/mouse.rs:1445-1452`
- Test: `src/client/shell/tests/keybindings_settings.rs`

**Interfaces:**
- Consumes: `Config::ui.focus_follows_mouse`, `ConfigEdit::FocusFollowsMouse`.
- Produces: `ClientShellConfig::focus_follows_mouse: bool`; `ClientSettingsSection::Focus`.

- [ ] **Step 1: Write the failing test**

Append to `src/client/shell/tests/keybindings_settings.rs`:

```rust
#[test]
fn settings_focus_section_reflects_config_and_offers_two_choices() {
    let mut config = Config::default();
    config.ui.focus_follows_mouse = true;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    assert!(state.config.focus_follows_mouse);
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.open_settings_overlay();
    state.select_settings_section(ClientSettingsSection::Focus, &mut ClientShellInput::default());
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::Settings(settings)) => {
            assert_eq!(settings.section, ClientSettingsSection::Focus);
            // "on" is index 0, "off" is index 1; config says on.
            assert_eq!(settings.selected, 0);
        }
        _ => panic!("settings overlay"),
    }
    state.compose(106, 30).expect("settings overlay frame");
    assert_eq!(state.hits.settings_choices.len(), 2);
    assert!(state
        .hits
        .settings_tabs
        .iter()
        .any(|(_, section)| *section == ClientSettingsSection::Focus));
}

#[test]
fn client_config_live_reload_applies_focus_follows_mouse() {
    let mut config = ClientShellConfig::from_config(&Config::default());
    assert!(!config.focus_follows_mouse);
    let mut loaded = Config::default();
    loaded.ui.focus_follows_mouse = true;
    config.apply_live_config(&loaded, &[], &[]);
    assert!(config.focus_follows_mouse);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo nextest run --locked --bin herdr settings_focus_section`
Expected: compile error, no `focus_follows_mouse` field / no `Focus` variant.

- [ ] **Step 3: Add the config field and projection**

In `src/client/shell/state.rs` after `pub(super) mouse_capture: bool,` (line 100):

```rust
    pub(super) focus_follows_mouse: bool,
```

In `src/client/shell/config.rs` `from_config` after `mouse_capture: config.ui.mouse_capture,`:

```rust
            focus_follows_mouse: config.ui.focus_follows_mouse,
```

In `apply_live_config` after `self.mouse_capture = ui.mouse_capture;`:

```rust
                self.focus_follows_mouse = ui.focus_follows_mouse;
```

- [ ] **Step 4: Add the settings section**

In `src/client/shell/state.rs` replace the `ClientSettingsSection` enum and impl:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClientSettingsSection {
    Theme,
    Indicators,
    Sound,
    Focus,
    Toast,
    Integrations,
}

impl ClientSettingsSection {
    pub(super) const ALL: &[Self] = &[
        Self::Theme,
        Self::Indicators,
        Self::Sound,
        Self::Focus,
        Self::Toast,
        Self::Integrations,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Indicators => "indicators",
            Self::Sound => "sound",
            Self::Focus => "focus",
            Self::Toast => "toasts",
            Self::Integrations => "integrations",
        }
    }
}
```

In `src/client/shell/settings.rs`:

`selected_index_for_settings_section` gets:

```rust
            ClientSettingsSection::Focus => usize::from(!self.config.focus_follows_mouse),
```

`settings_choice_count`:

```rust
                ClientSettingsSection::Indicators
                | ClientSettingsSection::Sound
                | ClientSettingsSection::Focus => 2,
```

`apply_settings_choice`, after the `Sound` arm:

```rust
            ClientSettingsSection::Focus => {
                self.save_settings_edit(
                    crate::config::ConfigEdit::FocusFollowsMouse(selected == 0),
                    outcome,
                );
            }
```

In `src/client/shell/settings_overlay.rs`, after the `Sound` render arm:

```rust
        ClientSettingsSection::Focus => {
            render_choice_section(
                buffer,
                content,
                "focus follows mouse",
                "focus the pane under the pointer without clicking",
                &["on", "off"],
                settings.selected,
                palette,
                &mut choice_hits,
            );
        }
```

In `src/client/shell/mouse.rs` the immediate-apply match (line 1447):

```rust
                            section: ClientSettingsSection::Indicators
                                | ClientSettingsSection::Sound
                                | ClientSettingsSection::Focus
                                | ClientSettingsSection::Toast,
```

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run --locked --bin herdr keybindings_settings`
Expected: all PASS, including the two new tests.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
git add src/client/shell
git commit -m "feat(settings): add focus follows mouse toggle to the settings overlay"
```

---

### Task 3: Focus the hovered pane on mouse move

**Files:**
- Modify: `src/client/shell/mouse.rs:2216-2225` (the `MouseEventKind::Moved` arm)
- Create: `src/client/shell/tests/focus_follows_mouse.rs`
- Modify: `src/client/shell/tests/mod.rs` (register module; look at how existing `mod` lines are declared near the top and add `mod focus_follows_mouse;` alongside them)

**Interfaces:**
- Consumes: `ClientShellConfig::focus_follows_mouse`, `self.focused_pane_id()`, `self.push_endpoint_method`.

- [ ] **Step 1: Write the failing tests**

Create `src/client/shell/tests/focus_follows_mouse.rs`:

```rust
use super::*;

fn state_with_focus_follows_mouse(enabled: bool) -> ClientShellState {
    let mut config = Config::default();
    config.ui.focus_follows_mouse = enabled;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    // The surface pane is "pane_1"; make the snapshot say some other pane is focused
    // so hovering pane_1 is a genuine focus change.
    let mut projected = snapshot();
    projected.focused_pane_id = Some("pane_other".into());
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("pane frame");
    state
}

fn moved_over_pane(state: &ClientShellState) -> RawInputEvent {
    let pane = state.hits.panes[0].clone();
    RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: pane.inner_rect.x + 1,
        row: pane.inner_rect.y,
        modifiers: KeyModifiers::empty(),
    })
}

fn pane_focus_requests(outcome: &ClientShellInput) -> Vec<String> {
    outcome
        .actions
        .iter()
        .filter_map(|action| match action {
            ClientShellAction::Endpoint { request, .. } => match &request.method {
                crate::api::schema::Method::PaneFocus(target) => Some(target.pane_id.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn hovering_an_unfocused_pane_requests_focus_when_enabled() {
    let mut state = state_with_focus_follows_mouse(true);
    let outcome = state.handle_raw_events(vec![moved_over_pane(&state)]);
    assert_eq!(pane_focus_requests(&outcome), vec!["pane_1".to_string()]);
}

#[test]
fn hovering_does_nothing_when_disabled() {
    let mut state = state_with_focus_follows_mouse(false);
    let outcome = state.handle_raw_events(vec![moved_over_pane(&state)]);
    assert!(pane_focus_requests(&outcome).is_empty());
}

#[test]
fn hovering_the_already_focused_pane_sends_nothing() {
    let mut config = Config::default();
    config.ui.focus_follows_mouse = true;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("pane frame");
    let outcome = state.handle_raw_events(vec![moved_over_pane(&state)]);
    assert!(pane_focus_requests(&outcome).is_empty());
}

#[test]
fn hovering_the_sidebar_sends_nothing() {
    let mut state = state_with_focus_follows_mouse(true);
    let row = state.hits.workspaces[0].rect;
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: row.x + 1,
        row: row.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(pane_focus_requests(&outcome).is_empty());
}

#[test]
fn hovering_with_an_overlay_open_sends_nothing() {
    let mut state = state_with_focus_follows_mouse(true);
    state.open_settings_overlay();
    let outcome = state.handle_raw_events(vec![moved_over_pane(&state)]);
    assert!(pane_focus_requests(&outcome).is_empty());
    state.cancel_settings_overlay();
}

#[test]
fn hovering_outside_terminal_mode_sends_nothing() {
    let mut state = state_with_focus_follows_mouse(true);
    state.mode = ClientShellMode::Navigate;
    let outcome = state.handle_raw_events(vec![moved_over_pane(&state)]);
    assert!(pane_focus_requests(&outcome).is_empty());
}

#[test]
fn hovering_during_a_chrome_drag_sends_nothing() {
    let mut state = state_with_focus_follows_mouse(true);
    state.chrome_drag = Some(ClientChromeDrag::SidebarWidth);
    let outcome = state.handle_raw_events(vec![moved_over_pane(&state)]);
    assert!(pane_focus_requests(&outcome).is_empty());
}

#[test]
fn hovering_with_mouse_capture_off_sends_nothing() {
    let mut config = Config::default();
    config.ui.focus_follows_mouse = true;
    config.ui.mouse_capture = false;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    let mut projected = snapshot();
    projected.focused_pane_id = Some("pane_other".into());
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("pane frame");
    let outcome = state.handle_raw_events(vec![moved_over_pane(&state)]);
    assert!(pane_focus_requests(&outcome).is_empty());
}
```

Register it: in `src/client/shell/tests/mod.rs` find the existing `mod chrome_context;`-style declarations (grep `^mod ` in that file) and add `mod focus_follows_mouse;` in alphabetical position.

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run --locked --bin herdr focus_follows_mouse`
Expected: `hovering_an_unfocused_pane_requests_focus_when_enabled` FAILS (no request); the negative tests pass trivially.

- [ ] **Step 3: Implement the `Moved` arm**

In `src/client/shell/mouse.rs` replace the `MouseEventKind::Moved` arm (around line 2216) with:

```rust
            MouseEventKind::Moved => {
                let hovered = self
                    .hits
                    .panes
                    .iter()
                    .find(|hit| super::contains(hit.inner_rect, point))
                    .cloned();
                if let Some(hit) = hovered.as_ref() {
                    let follows_mouse = self.config.focus_follows_mouse
                        && self.config.mouse_capture
                        && self.mode == ClientShellMode::Terminal
                        && self.overlay.is_none()
                        && self.chrome_drag.is_none()
                        && self.pane_mouse_gesture.is_none()
                        && !self
                            .selection
                            .as_ref()
                            .is_some_and(crate::selection::Selection::is_dragging)
                        && !hit.popup;
                    if follows_mouse
                        && self.focused_pane_id().as_deref() != Some(hit.pane_id.as_str())
                    {
                        self.push_endpoint_method(
                            crate::api::schema::Method::PaneFocus(crate::api::schema::PaneTarget {
                                pane_id: hit.pane_id.clone(),
                            }),
                            outcome,
                        );
                    }
                }
                if let Some(hit) = hovered.filter(|hit| hit.mouse_reporting) {
                    self.push_pane_mouse_event(&hit, mouse, mouse.modifiers, outcome);
                }
            }
```

Check `Selection::is_dragging` is `pub fn is_dragging(&self) -> bool` at `src/selection.rs:172`; it is.

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run --locked --bin herdr focus_follows_mouse`
Expected: all 8 PASS. Then run `cargo nextest run --locked --bin herdr mouse_selection` to confirm existing mouse tests still pass.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add src/client/shell/mouse.rs src/client/shell/tests
git commit -m "feat(mouse): focus the hovered pane when ui.focus_follows_mouse is on"
```

---

### Task 4: `Workspace::theme` with persistence

**Files:**
- Modify: `src/workspace.rs:175-207` (struct), `:236-273` (`from_existing_pane`), `:404-420` (`new_with_tab`), `:1174-1215` (`test_new`)
- Modify: `src/persist/snapshot.rs:50-68` (`WorkspaceSnapshot`), `:144-165` (legacy `From`), `:283-300` (`capture_workspace`), tests at the bottom of the file
- Modify: `src/persist/restore.rs:410-425`

**Interfaces:**
- Produces: `Workspace::theme: Option<String>`, `WorkspaceSnapshot::theme: Option<String>`.

- [ ] **Step 1: Write the failing persistence test**

In `src/persist/snapshot.rs` `#[cfg(test)] mod tests` (find it with `grep -n "mod tests" src/persist/snapshot.rs`), add:

```rust
    #[test]
    fn workspace_snapshot_theme_round_trips_and_defaults_to_none() {
        let with_theme = WorkspaceSnapshot {
            id: Some("wabc".into()),
            custom_name: None,
            identity_cwd: PathBuf::from("/repo"),
            worktree_space: None,
            public_pane_numbers: HashMap::new(),
            next_public_pane_number: 2,
            public_tab_numbers: vec![1],
            next_public_tab_number: 2,
            tabs: Vec::new(),
            active_tab: 0,
            theme: Some("nord".into()),
        };
        let json = serde_json::to_value(&with_theme).unwrap();
        assert_eq!(json["theme"], serde_json::json!("nord"));
        let restored: WorkspaceSnapshot = serde_json::from_value(json).unwrap();
        assert_eq!(restored.theme.as_deref(), Some("nord"));

        let without_theme = WorkspaceSnapshot {
            theme: None,
            ..with_theme
        };
        let json = serde_json::to_value(&without_theme).unwrap();
        assert!(json.get("theme").is_none());

        let legacy = serde_json::json!({
            "identity_cwd": "/repo",
            "tabs": []
        });
        let restored: WorkspaceSnapshot = serde_json::from_value(legacy).unwrap();
        assert!(restored.theme.is_none());
    }
```

`WorkspaceSnapshot` derives `Serialize, Deserialize` but not `Clone`; the `..with_theme` struct update moves the remaining fields, which is fine because `with_theme` is not used afterwards.

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run --locked --bin herdr workspace_snapshot_theme_round_trips`
Expected: compile error, no field `theme`.

- [ ] **Step 3: Add the fields**

`src/workspace.rs` struct, after `pub worktree_space: Option<WorktreeSpaceMembership>,`:

```rust
    /// Optional per-space theme override (canonical built-in theme name). `None` follows the global theme.
    pub theme: Option<String>,
```

Add `theme: None,` to every `Workspace { .. }` literal: `src/workspace.rs` in `from_existing_pane` (after `worktree_space: None,`), in `new_with_tab` (after `worktree_space: None,`), in `test_new` (after `worktree_space: None,`).

`src/persist/snapshot.rs` `WorkspaceSnapshot`, after `active_tab`:

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
```

In `impl From<LegacyWorkspaceSnapshot>` add `theme: None,` after `tabs: vec![tab],`. In `capture_workspace` add `theme: ws.theme.clone(),` after `worktree_space: ws.worktree_space.clone(),`. Update the two test literals at `snapshot.rs:660` and `:1227` with `theme: None,` (the compiler will point at every literal that is missing it).

`src/persist/restore.rs:420` in the `Workspace { .. }` literal add `theme: snap.theme.clone(),` after `worktree_space,`.

- [ ] **Step 4: Build and run**

Run: `cargo nextest run --locked --bin herdr persist`
Expected: all persist tests PASS including the new one. If any other `Workspace {` or `WorkspaceSnapshot {` literal fails to compile, add `theme: None,` there.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add src/workspace.rs src/persist
git commit -m "feat(workspace): persist an optional per-space theme override"
```

---

### Task 5: `workspace.set_theme` API method and server palette cache

**Files:**
- Modify: `src/api/schema/workspaces.rs` (params struct), `src/api/schema.rs:82-84` (variant), `src/api/server.rs:394` (`api_method_name`), `src/app/api.rs:999-1001` (dispatch), `src/app/api/workspaces.rs` (handler + tests), `src/logging.rs:247`
- Modify: `src/server/client_commands.rs:48`, `tests/fixtures/endpoint-method-shapes-v1.json`, `docs/next/api/herdr-api.schema.json`, `docs/next/website/src/content/docs/socket-api.mdx:103` and the same table row in `ja/socket-api.mdx` and `zh-cn/socket-api.mdx`
- Modify: `src/app/state.rs:858-863` (field), `:1083-1091` (default), `src/app/mod.rs:334-350` (helper), `src/app/theme_sync.rs:32-45`
- Modify: `src/app/mod.rs` `App::new` after the `AppState { .. }` literal (search for `let mut state = AppState {` at line 440 and add the call after the literal closes)

**Interfaces:**
- Produces: `Method::WorkspaceSetTheme(WorkspaceSetThemeParams { workspace_id: String, theme: Option<String> })` with wire name `workspace.set_theme`; `AppState::palette_for_workspace(&self, workspace_id: &str) -> &Palette`; `AppState::rebuild_workspace_theme_palettes(&mut self)`; `crate::app::named_theme_palette(&ThemeRuntimeConfig, &str) -> Palette`.

- [ ] **Step 1: Write the failing handler tests**

In `src/app/api/workspaces.rs` tests module, add:

```rust
    fn app_with_one_workspace() -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = vec![Workspace::test_new("themed")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app
    }

    #[tokio::test]
    async fn workspace_set_theme_stores_canonical_name_and_resolves_palette() {
        let mut app = app_with_one_workspace();
        let workspace_id = app.public_workspace_id(0);
        let response = app.handle_workspace_set_theme(
            "req".into(),
            crate::api::schema::WorkspaceSetThemeParams {
                workspace_id: workspace_id.clone(),
                theme: Some("Tokyo Night".into()),
            },
        );
        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert!(matches!(success.result, ResponseResult::WorkspaceInfo { .. }));
        assert_eq!(
            app.state.workspaces[0].theme.as_deref(),
            Some("tokyo-night")
        );
        assert_eq!(
            app.state.palette_for_workspace(&workspace_id).accent,
            crate::app::state::Palette::tokyo_night().accent
        );
        assert!(app.state.session_dirty);

        let response = app.handle_workspace_set_theme(
            "req2".into(),
            crate::api::schema::WorkspaceSetThemeParams {
                workspace_id: workspace_id.clone(),
                theme: None,
            },
        );
        let _: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert!(app.state.workspaces[0].theme.is_none());
        assert_eq!(
            app.state.palette_for_workspace(&workspace_id).accent,
            app.state.palette.accent
        );
    }

    #[tokio::test]
    async fn workspace_set_theme_rejects_unknown_theme_and_workspace() {
        let mut app = app_with_one_workspace();
        let workspace_id = app.public_workspace_id(0);
        let response = app.handle_workspace_set_theme(
            "req".into(),
            crate::api::schema::WorkspaceSetThemeParams {
                workspace_id: workspace_id.clone(),
                theme: Some("not-a-theme".into()),
            },
        );
        let error: ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(error.error.code, "invalid_theme");
        assert!(app.state.workspaces[0].theme.is_none());

        let response = app.handle_workspace_set_theme(
            "req".into(),
            crate::api::schema::WorkspaceSetThemeParams {
                workspace_id: "wzzzz".into(),
                theme: Some("nord".into()),
            },
        );
        let error: ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(error.error.code, "workspace_not_found");
    }
```

Check whether `Palette::tokyo_night()` is `pub`; the constructors on `Palette` in `src/app/state.rs:74-526` are `pub fn`. If `Palette` itself is not reachable as `crate::app::state::Palette` from this module, use `crate::app::state::Palette::from_name("tokyo-night").unwrap().accent`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run --locked --bin herdr workspace_set_theme`
Expected: compile errors for the missing params type and handler.

- [ ] **Step 3: Add the schema and method plumbing**

`src/api/schema/workspaces.rs`, after `WorkspaceRenameParams`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceSetThemeParams {
    pub workspace_id: String,
    /// Built-in theme name to apply to this workspace only, or `null` to follow the global theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}
```

Make sure the type is re-exported wherever `WorkspaceRenameParams` is (grep `WorkspaceRenameParams` in `src/api/schema.rs` and mirror any `pub use`).

`src/api/schema.rs`, after the `WorkspaceRename` variant:

```rust
    #[serde(rename = "workspace.set_theme")]
    WorkspaceSetTheme(WorkspaceSetThemeParams),
```

`src/api/server.rs` `api_method_name`, after the `WorkspaceRename` arm:

```rust
        Method::WorkspaceSetTheme(_) => "workspace.set_theme",
```

`src/app/api.rs` dispatch, after the `WorkspaceRename` arm:

```rust
            Method::WorkspaceSetTheme(params) => {
                return self.handle_workspace_set_theme(request.id, params);
            }
```

`src/server/client_commands.rs` `CLIENT_SHELL_METHODS`: insert `"workspace.set_theme",` between `"workspace.rename",` and `"worktree.create",` (the list must stay sorted).

`src/logging.rs`, after `workspace_renamed`:

```rust
pub(crate) fn workspace_theme_changed(workspace_id: &str, theme: Option<&str>) {
    tracing::info!(
        event = "workspace.set_theme",
        subsystem = "workspace",
        outcome = "ok",
        workspace_id,
        theme = theme.unwrap_or("global"),
        "workspace theme changed"
    );
}
```

Build with `cargo build --locked` and add the new arm to any other exhaustive `match` over `Method` the compiler reports.

- [ ] **Step 4: Add the server palette cache**

`src/app/state.rs` `AppState`, after `pub theme_runtime: ThemeRuntimeConfig,`:

```rust
    /// Resolved palettes for per-workspace theme overrides, keyed by canonical theme name.
    pub workspace_theme_palettes: std::collections::HashMap<String, Palette>,
```

In the `AppState` default literal (line 1083 region) after the `theme_runtime: ThemeRuntimeConfig { .. },` entry:

```rust
            workspace_theme_palettes: std::collections::HashMap::new(),
```

Also add it to the `AppState { .. }` literal in `src/app/mod.rs` `App::new` (line 440 region), next to `theme_runtime`. Grep `theme_runtime,` and `theme_name,` in `src/app/mod.rs` to find the exact spot.

In `impl AppState` (after `mark_session_dirty`), add:

```rust
    /// Palette for the given public workspace id: its theme override when set, otherwise the global palette.
    pub fn palette_for_workspace(&self, workspace_id: &str) -> &Palette {
        self.workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .and_then(|workspace| workspace.theme.as_deref())
            .and_then(|theme| self.workspace_theme_palettes.get(theme))
            .unwrap_or(&self.palette)
    }

    /// Re-resolve every per-workspace theme override against the current theme runtime.
    pub fn rebuild_workspace_theme_palettes(&mut self) {
        let mut palettes = std::collections::HashMap::new();
        for theme in self
            .workspaces
            .iter()
            .filter_map(|workspace| workspace.theme.as_deref())
        {
            if !palettes.contains_key(theme) {
                palettes.insert(
                    theme.to_owned(),
                    crate::app::named_theme_palette(&self.theme_runtime, theme),
                );
            }
        }
        self.workspace_theme_palettes = palettes;
    }
```

`src/app/mod.rs`, next to `client_palette_for_theme` (line 334):

```rust
/// Resolve a named built-in theme through the same override pipeline as the global theme.
pub(crate) fn named_theme_palette(
    runtime: &state::ThemeRuntimeConfig,
    name: &str,
) -> state::Palette {
    resolve_palette_for_theme_name(name, "catppuccin", runtime, None)
}
```

and change `client_palette_for_theme` to delegate: `named_theme_palette(runtime, name)`.

`src/app/theme_sync.rs` `refresh_effective_app_theme`: add `self.state.rebuild_workspace_theme_palettes();` as the first statement, before the early-return comparison.

`src/app/mod.rs` `App::new`: after the `let mut state = AppState { .. };` literal is complete, add `state.rebuild_workspace_theme_palettes();` so restored sessions get palettes.

- [ ] **Step 5: Add the handler**

`src/app/api/workspaces.rs`: add `WorkspaceSetThemeParams` to the `use crate::api::schema::{ .. }` import, then after `handle_workspace_rename`:

```rust
    pub(super) fn handle_workspace_set_theme(
        &mut self,
        id: String,
        params: WorkspaceSetThemeParams,
    ) -> String {
        let Some(index) = self.parse_workspace_id(&params.workspace_id) else {
            return workspace_not_found(id, &params.workspace_id);
        };
        if self.state.workspaces.get(index).is_none() {
            return workspace_not_found(id, &params.workspace_id);
        }
        let theme = match params.theme.as_deref() {
            None => None,
            Some(name) => match crate::config::canonical_theme_name(name) {
                Some(canonical) => Some(canonical.to_owned()),
                None => {
                    return encode_error(
                        id,
                        "invalid_theme",
                        format!(
                            "unknown theme {name:?}; expected one of {}",
                            crate::config::THEME_NAMES.join(", ")
                        ),
                    );
                }
            },
        };
        let workspace = &mut self.state.workspaces[index];
        workspace.theme = theme;
        crate::logging::workspace_theme_changed(&workspace.id, workspace.theme.as_deref());
        self.state.rebuild_workspace_theme_palettes();
        self.state.mark_session_dirty();
        self.schedule_session_save();
        self.render_dirty.request_generic();
        self.render_notify.notify_one();

        encode_success(
            id,
            ResponseResult::WorkspaceInfo {
                workspace: self.workspace_info(index),
            },
        )
    }
```

Confirm `canonical_theme_name` is exported from `crate::config` (grep `canonical_theme_name` in `src/config.rs`; it is `pub(crate)` in `src/config/theme.rs`, add a `pub(crate) use theme::canonical_theme_name;` if the facade lacks it). Confirm `mark_session_dirty` exists on `AppState` (`src/app/state.rs:896`) and that `render_dirty`/`render_notify` are `App` fields (used in `theme_sync.rs`).

- [ ] **Step 6: Run the handler tests**

Run: `cargo nextest run --locked --bin herdr workspace_set_theme`
Expected: both PASS.

- [ ] **Step 7: Update the method-shape fixture and schema artifact**

Run: `cargo nextest run --locked --bin herdr advertised_client_shell_method_shapes_stay_at_the_v1_contract`
Expected: FAIL with an assertion diff showing the actual map contains a `workspace.set_theme` digest. Copy that 64-hex digest into `tests/fixtures/endpoint-method-shapes-v1.json` as a new line between `workspace.rename` and `worktree.create`, keeping JSON valid. Re-run; expected PASS. All other digests must remain unchanged.

Run: `HERDR_UPDATE_API_SCHEMA=1 cargo nextest run --locked --bin herdr generated_protocol_schema_artifact_is_current` then `cargo nextest run --locked --bin herdr generated_protocol_schema_artifact_is_current`
Expected: second run PASS; `git diff --stat docs/next/api/herdr-api.schema.json` shows only additions for the new method.

- [ ] **Step 8: Document the method**

In `docs/next/website/src/content/docs/socket-api.mdx` line 103, insert `` `workspace.set_theme`, `` after `` `workspace.rename`, ``. Apply the same edit to the matching row in `docs/next/website/src/content/docs/ja/socket-api.mdx` and `docs/next/website/src/content/docs/zh-cn/socket-api.mdx` (grep `workspace.rename` in each to find the row).

- [ ] **Step 9: Run related tests and commit**

Run: `cargo nextest run --locked --bin herdr client_commands && cargo nextest run --locked --bin herdr schema`
Expected: PASS.

```bash
cargo fmt --all
git add src/api src/app src/server/client_commands.rs src/logging.rs tests/fixtures/endpoint-method-shapes-v1.json docs/next/api/herdr-api.schema.json docs/next/website/src/content/docs
git commit -m "feat(api): add workspace.set_theme and per-workspace palette resolution"
```

---

### Task 6: Server-rendered split borders and titles follow the space palette

**Files:**
- Modify: `src/ui/panes.rs:489-493` and `:656-660`
- Test: `src/ui/panes.rs` tests module (near the `render_view_pane_borders` helper, line 824)

- [ ] **Step 1: Write the failing test**

Copy the existing split-border focus test (the one asserting `buffer[(2, 2)].style().fg == Some(app.palette.accent)` around line 1075) and adapt it. Add in the tests module:

```rust
    #[test]
    fn split_borders_use_the_workspace_theme_palette() {
        let mut app = AppState::test_new();
        app.view.terminal_area = Rect::new(0, 0, 4, 4);
        app.view.pane_infos = vec![
            PaneInfo {
                id: PaneId::from_raw(1),
                rect: Rect::new(0, 0, 2, 2),
                inner_rect: Rect::default(),
                scrollbar_rect: None,
                borders: Borders::ALL,
                is_focused: true,
            },
            PaneInfo {
                id: PaneId::from_raw(2),
                rect: Rect::new(2, 0, 2, 2),
                inner_rect: Rect::default(),
                scrollbar_rect: None,
                borders: Borders::ALL,
                is_focused: false,
            },
        ];
        let split_borders = vec![crate::layout::SplitBorder {
            pos: 2,
            direction: ratatui::layout::Direction::Horizontal,
            ratio: 0.5,
            area: Rect::new(0, 0, 4, 2),
            path: vec![],
        }];
        let mut ws = Workspace::test_new("themed");
        ws.theme = Some("nord".into());
        app.workspaces = vec![ws];
        app.rebuild_workspace_theme_palettes();
        let expected = Palette::nord().accent;
        assert_ne!(expected, app.palette.accent);

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(4, 4)).unwrap();
        terminal
            .draw(|frame| {
                render_view_pane_borders(&app, &app.workspaces[0], &split_borders, frame)
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 0)].style().fg, Some(expected));
    }
```

Adjust the imports at the top of the tests module if `Palette` or `Workspace` are not already imported there (look at the existing tests for the exact `use` lines). If `app.workspaces` is not the field name on `AppState`, check `src/app/state.rs` and use the correct one (it is `pub workspaces: Vec<Workspace>` per `AppState`).

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run --locked --bin herdr split_borders_use_the_workspace_theme_palette`
Expected: FAIL, colour equals the global accent.

- [ ] **Step 3: Use the workspace palette**

In `render_pane_borders` (line 453) add at the top after the early return:

```rust
    let palette = app.palette_for_workspace(&ws.id);
```

and change the colour selection at lines 489-493 to `palette.accent` / `palette.overlay0`. In `render_pane_border_titles` (line 622) add `let palette = app.palette_for_workspace(&ws.id);` after the `let area = buf.area;` line and change lines 656-660 to `palette.accent` / `palette.overlay0`.

- [ ] **Step 4: Run tests**

Run: `cargo nextest run --locked --bin herdr ui::panes`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add src/ui/panes.rs
git commit -m "feat(render): colour split borders and titles with the space theme"
```

---

### Task 7: Ship the space theme to the client

**Files:**
- Modify: `src/protocol/wire.rs:20` (version), `:1012-1025` (`ClientShellWorkspace`), `:2646` (test literal)
- Modify: `src/server/client_shell.rs:53-77`
- Modify test literals: `src/client/shell/tests/mod.rs:29`, `src/client/shell/tests/chrome_context.rs:54`, `src/client/shell/tests/mobile.rs:253,452,492`, `src/client/shell/tests/agents_worktrees_notifications.rs:102`

**Interfaces:**
- Produces: `crate::protocol::ClientShellWorkspace::theme: Option<String>`; `PROTOCOL_VERSION == 23`.

- [ ] **Step 1: Write the failing test**

In `src/protocol/wire.rs` tests, next to `client_shell_snapshot_roundtrip`, add:

```rust
    #[test]
    fn client_shell_workspace_theme_round_trips() {
        let workspace = ClientShellWorkspace {
            workspace_id: "w1".into(),
            active_tab_id: "w1:t1".into(),
            new_workspace_cwd: "/tmp".into(),
            number: 1,
            label: "shell".into(),
            custom_label: false,
            branch: None,
            git_ahead_behind: None,
            tokens: Vec::new(),
            worktree: None,
            focused: false,
            agent_status: crate::api::schema::AgentStatus::Idle,
            theme: Some("nord".into()),
        };
        let encoded =
            bincode::serde::encode_to_vec(&workspace, bincode::config::standard()).unwrap();
        let (decoded, _): (ClientShellWorkspace, _) =
            bincode::serde::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        assert_eq!(decoded.theme.as_deref(), Some("nord"));
        assert_eq!(PROTOCOL_VERSION, 23);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run --locked --bin herdr client_shell_workspace_theme_round_trips`
Expected: compile error (no `theme` field).

- [ ] **Step 3: Add the field, bump the version, project it**

`src/protocol/wire.rs`: set `pub const PROTOCOL_VERSION: u32 = 23;`. In `ClientShellWorkspace` after `agent_status`:

```rust
    /// Per-space theme override (canonical built-in name), `None` when the space follows the global theme.
    #[serde(default)]
    pub theme: Option<String>,
```

`src/server/client_shell.rs` in the `protocol::ClientShellWorkspace { .. }` literal add `theme: state.theme.clone(),` after `agent_status: workspace.agent_status,`.

Add `theme: None,` to every `ClientShellWorkspace { .. }` literal the compiler reports: `src/protocol/wire.rs` test at 2646, `src/client/shell/tests/mod.rs`, `chrome_context.rs`, `mobile.rs` (three), `agents_worktrees_notifications.rs`.

- [ ] **Step 4: Build and run protocol tests**

Run: `cargo nextest run --locked --bin herdr protocol::wire`
Expected: PASS. Then `cargo nextest run --locked --bin herdr client::shell` to confirm the fixture updates compile and pass.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add src/protocol/wire.rs src/server/client_shell.rs src/client/shell/tests
git commit -m "feat(protocol): carry the per-space theme in the client shell snapshot"
```

---

### Task 8: Client rendering of space palettes (sidebar rows, accent bar, tab bar)

**Files:**
- Modify: `src/client/shell/state.rs` (`ClientShellState` fields at ~885, constructor at ~1036, `ClientSettingsOverlay` at 437)
- Modify: `src/client/shell/config.rs:53-70` (`reload_client_config`), and the appearance-change path that sets `self.config.palette` (grep `client_palette_for_appearance` in `src/client/shell/`)
- Modify: `src/client/shell/render.rs:196-209` (`ShellRenderState`), `:224-260`
- Modify: `src/client/shell/composition.rs:50-70`
- Modify: `src/client/shell/tabs.rs:7-18`
- Modify: `src/client/shell/sidebar.rs:25-90` (collapsed), `:280-330` (expanded loop), `:620-700` (`render_workspace_rows`)
- Create: `src/client/shell/tests/space_themes.rs` (register `mod space_themes;` in `tests/mod.rs`)

**Interfaces:**
- Produces: `ClientShellState::theme_palette_cache: HashMap<String, Palette>`; `ClientShellState::workspace_palettes(&mut self) -> HashMap<String, Palette>` (keyed by workspace id, only for spaces with an override, honouring the picker preview); `ShellRenderState::workspace_palettes: &'a HashMap<String, Palette>`; `render_tab_bar(.., palette: &Palette, ..)`.
- Preview hook consumed here, produced in Task 9: `ClientSettingsOverlay::workspace_preview: Option<(String, Option<String>)>` meaning `(workspace_id, previewed theme name or None for global)`. Add the field in this task with no producer yet.

- [ ] **Step 1: Write the failing tests**

Create `src/client/shell/tests/space_themes.rs`:

```rust
use super::*;

fn themed_snapshot(theme: Option<&str>) -> ClientShellSnapshot {
    let mut projected = snapshot();
    projected.workspaces[0].theme = theme.map(str::to_owned);
    projected
}

fn cell_fg(frame: &FrameData, x: u16, y: u16) -> Option<ratatui::style::Color> {
    let index = usize::from(y) * usize::from(frame.width) + usize::from(x);
    frame.cells[index].style.fg
}

#[test]
fn themed_space_row_shows_an_accent_bar_in_its_own_palette() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(Some("nord"))));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 20).expect("frame");
    let row = state.hits.workspaces[0].rect;
    let nord = crate::app::client_palette_for_theme(&state.config.theme_runtime, "nord");
    assert_ne!(nord.accent, state.config.palette.accent);
    assert_eq!(cell_fg(&frame, row.x, row.y), Some(nord.accent));
}

#[test]
fn untitled_space_row_has_no_accent_bar() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(None)));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 20).expect("frame");
    let row = state.hits.workspaces[0].rect;
    assert_ne!(cell_fg(&frame, row.x, row.y), Some(state.config.palette.accent));
}

#[test]
fn tab_bar_uses_the_active_space_palette() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(Some("nord"))));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 20).expect("frame");
    let tab = state.hits.tabs[0].rect;
    let nord = crate::app::client_palette_for_theme(&state.config.theme_runtime, "nord");
    let index = usize::from(tab.y) * usize::from(frame.width) + usize::from(tab.x);
    assert_eq!(frame.cells[index].style.bg, Some(nord.accent));
}

#[test]
fn unknown_space_theme_falls_back_to_the_global_palette() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(Some("not-a-theme"))));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("frame");
    assert!(state.workspace_palettes().is_empty());
}
```

Before running, confirm the frame cell API: look at how `tab_bar_renders_endpoint_status_ellipses_and_clamps_to_useful_scroll` in `keybindings_settings.rs` reads `frame.cells[..]` and `cell.symbol`; check `FrameData` cell struct fields (grep `pub struct FrameCell` or similar in `src/protocol/`) and adjust `cell_fg` to the real field names (`style.fg` vs `fg`). Also confirm `state.hits.tabs[0]` has a `rect` field (`TabHit` in `src/client/shell/state.rs`); the focused tab is drawn with `bg(palette.accent)` at `tabs.rs:105-106`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run --locked --bin herdr space_themes`
Expected: compile error (`workspace_palettes` missing, `theme` field on snapshot exists from Task 7).

- [ ] **Step 3: Add the client palette cache and map**

`src/client/shell/state.rs` `ClientShellState` fields (near `hits`):

```rust
    /// Resolved palettes for per-space theme overrides, keyed by canonical theme name.
    pub(super) theme_palette_cache: HashMap<String, Palette>,
```

and in `ClientShellState::new` literal: `theme_palette_cache: HashMap::new(),`.

`ClientSettingsOverlay` gains:

```rust
    /// Live preview for a space-targeted picker: `(workspace_id, theme name or None for global)`.
    pub(super) workspace_preview: Option<(String, Option<String>)>,
```

and `open_settings_overlay` in `settings.rs` sets `workspace_preview: None,`.

In `src/client/shell/state.rs` `impl ClientShellState` add:

```rust
    /// Palettes for spaces with a theme override, keyed by workspace id. Honors the
    /// settings overlay preview for the targeted space. Unknown names are skipped so
    /// those spaces fall back to the global palette.
    pub(super) fn workspace_palettes(&mut self) -> HashMap<String, Palette> {
        let mut palettes = HashMap::new();
        let Some(snapshot) = self.snapshot.as_deref() else {
            return palettes;
        };
        let preview = match self.overlay.as_ref() {
            Some(ClientShellOverlay::Settings(settings)) => settings.workspace_preview.clone(),
            _ => None,
        };
        let runtime = &self.config.theme_runtime;
        for workspace in &snapshot.workspaces {
            let theme = match preview.as_ref() {
                Some((workspace_id, theme)) if *workspace_id == workspace.workspace_id => {
                    theme.as_deref()
                }
                _ => workspace.theme.as_deref(),
            };
            let Some(theme) = theme.and_then(crate::config::canonical_theme_name) else {
                continue;
            };
            let palette = self
                .theme_palette_cache
                .entry(theme.to_owned())
                .or_insert_with(|| crate::app::client_palette_for_theme(runtime, theme));
            palettes.insert(workspace.workspace_id.clone(), palette.clone());
        }
        palettes
    }
```

If the borrow checker complains about `runtime` borrowed from `self.config` while `self.theme_palette_cache` is mutated, clone `self.config.theme_runtime` into a local first (it is `Clone`).

Invalidate the cache wherever the global palette is re-resolved: in `src/client/shell/config.rs` `reload_client_config` after `apply_live_config(..)` add `self.theme_palette_cache.clear();`, and in the host-appearance handler that calls `client_palette_for_appearance` (grep it in `src/client/shell/`), add the same clear.

- [ ] **Step 4: Thread the palettes into rendering**

`src/client/shell/render.rs` `ShellRenderState` gains:

```rust
    pub(super) workspace_palettes: &'a HashMap<String, Palette>,
```

`src/client/shell/composition.rs` before `let mut buffer = Buffer::empty(..)`: `let workspace_palettes = self.workspace_palettes();` and pass `workspace_palettes: &workspace_palettes,` in the `ShellRenderState { .. }` literal.

In `render_shell`, the collapsed sidebar call gets a new argument `state.workspace_palettes` (add a `workspace_palettes: &HashMap<String, Palette>` parameter to `render_collapsed_sidebar` after `selected_workspace_id`). The tab bar call becomes:

```rust
    if layout.tab_bar.height > 0 {
        let tab_palette = snapshot
            .focused_workspace_id
            .as_deref()
            .and_then(|id| state.workspace_palettes.get(id))
            .unwrap_or(&config.palette);
        render_tab_bar(
            buffer,
            layout.tab_bar,
            snapshot,
            config,
            tab_palette,
            state.tab_scroll,
            state.reveal_focused_tab,
            state.tab_drag_insert_index,
            &mut hits,
        );
    }
```

`src/client/shell/tabs.rs` `render_tab_bar`: add `palette: &Palette,` after `config: &ClientShellConfig,` and replace `let palette = &config.palette;` with nothing (the parameter is already named `palette`). Fix any other callers the compiler reports (grep `render_tab_bar(`; `mobile.rs` may call it, pass `&config.palette` there).

- [ ] **Step 5: Sidebar rows**

`src/client/shell/sidebar.rs` expanded loop (line 283 onward): after `let dragged = ..;` add

```rust
        let row_palette = state
            .workspace_palettes
            .get(&workspace.workspace_id)
            .unwrap_or(palette);
        let themed = state.workspace_palettes.contains_key(&workspace.workspace_id);
```

Use `row_palette` for the three `buffer.set_style(rect, ..)` background lines and pass `row_palette` (instead of `palette`) to `render_workspace_rows`. After `render_workspace_rows(..)` returns, draw the bar:

```rust
        if themed {
            for bar_y in rect.y..rect.bottom() {
                buffer[(rect.x, bar_y)]
                    .set_symbol("▌")
                    .set_fg(row_palette.accent);
            }
        }
```

`render_workspace_rows` already paints the background after the text, so draw the bar after the call to keep the bar's foreground on top of that background (the background loop only sets `bg`).

Collapsed sidebar (`render_collapsed_sidebar`): after computing `selected`, add

```rust
        let row_palette = workspace_palettes
            .get(&workspace.workspace_id)
            .unwrap_or(palette);
```

and use `row_palette` in `selection_background`, the two `set_style` calls, and `number_style`; when `workspace_palettes.contains_key(..)` and the row is neither selected nor focused, use `Style::default().fg(row_palette.accent)` for the number instead of `overlay0`. Keep `status_color(status, palette)` on the global palette so agent-state colours stay consistent across rows.

- [ ] **Step 6: Run tests**

Run: `cargo nextest run --locked --bin herdr space_themes && cargo nextest run --locked --bin herdr client::shell`
Expected: PASS. If `themed_space_row_shows_an_accent_bar_in_its_own_palette` fails because the row rect starts one column to the right of where the bar is drawn, print `row` and check `body.x` in `render_sidebar`; the bar is at `rect.x` which equals `body.x`, and `hits.workspaces[..].rect` is the same `rect`.

- [ ] **Step 7: Commit**

```bash
cargo fmt --all
git add src/client/shell
git commit -m "feat(client): render sidebar rows and the tab bar with per-space palettes"
```

---

### Task 9: `Theme...` context-menu item and the space-targeted picker

**Files:**
- Modify: `src/client/shell/state.rs:537-553` (`ClientContextMenuAction::Theme`), `:437-448` (`ClientSettingsOverlay::target`, new enum `ClientSettingsTarget`)
- Modify: `src/client/shell/context_menu.rs:9-44` (items), `:219-290` (action)
- Modify: `src/client/shell/settings.rs` (`open_settings_overlay`, new `open_workspace_theme_picker`, `sections()`, `move_settings_section`, `settings_choice_count`, `preview_selected_theme`, `cancel_settings_overlay`, `apply_settings_choice`)
- Modify: `src/client/shell/settings_overlay.rs:75-110` (tabs loop), `:131-159` (theme list)
- Modify: `src/client/shell/mouse.rs` (no change needed beyond Task 2, verify)
- Test: `src/client/shell/tests/space_themes.rs`

**Interfaces:**
- Produces: `ClientSettingsTarget { Global, Workspace { workspace_id: String } }`; `ClientShellState::open_workspace_theme_picker(&mut self, workspace_id: String)`; `ClientSettingsOverlay::sections(&self) -> &'static [ClientSettingsSection]`; `ClientSettingsOverlay::theme_choices(&self) -> Vec<&'static str>`.

- [ ] **Step 1: Write the failing tests**

Append to `src/client/shell/tests/space_themes.rs`:

```rust
#[test]
fn workspace_context_menu_offers_theme_and_opens_a_targeted_picker() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(Some("nord"))));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("frame");
    let row = state.hits.workspaces[0].rect;
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: row.x + 2,
        row: row.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let theme_index = match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .iter()
            .position(|item| item.action == ClientContextMenuAction::Theme)
            .expect("Theme... item"),
        _ => panic!("workspace context menu"),
    };
    assert_eq!(theme_index, 1, "Theme... sits right after Rename");
    state.compose(106, 20).expect("menu frame");
    let item = state.hits.context_menu_rows[theme_index].0;
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: item.x + 1,
        row: item.y,
        modifiers: KeyModifiers::empty(),
    })]);
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::Settings(settings)) => {
            assert!(matches!(
                settings.target,
                ClientSettingsTarget::Workspace { ref workspace_id } if workspace_id == "ws_1"
            ));
            assert_eq!(settings.section, ClientSettingsSection::Theme);
            assert_eq!(settings.sections(), &[ClientSettingsSection::Theme]);
            let choices = settings.theme_choices();
            assert_eq!(choices[0], "use global theme");
            let nord = crate::config::THEME_NAMES
                .iter()
                .position(|name| *name == "nord")
                .unwrap();
            assert_eq!(settings.selected, nord + 1);
        }
        _ => panic!("targeted settings overlay"),
    }
}

#[test]
fn targeted_picker_previews_applies_and_cancels() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(None)));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("frame");
    state.open_workspace_theme_picker("ws_1".into());
    // Move down once: from "use global theme" to THEME_NAMES[0] ("catppuccin"),
    // then to THEME_NAMES[1] ("catppuccin-latte").
    state.move_settings_selection(1);
    state.move_settings_selection(1);
    let previewed = state.workspace_palettes();
    let latte = crate::app::client_palette_for_theme(&state.config.theme_runtime, "catppuccin-latte");
    assert_eq!(previewed.get("ws_1").map(|p| p.accent), Some(latte.accent));
    // The global palette is untouched by a space preview.
    assert_eq!(state.config.theme_name, "catppuccin");

    let mut outcome = ClientShellInput::default();
    state.apply_settings_choice(&mut outcome);
    let [ClientShellAction::Endpoint { request, .. }] = &outcome.actions[..] else {
        panic!("apply should send one endpoint request");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::WorkspaceSetTheme(params)
            if params.workspace_id == "ws_1" && params.theme.as_deref() == Some("catppuccin-latte")
    ));
    assert!(state.overlay.is_none());
    assert!(state.workspace_palettes().is_empty());

    state.open_workspace_theme_picker("ws_1".into());
    state.move_settings_selection(1);
    assert!(!state.workspace_palettes().is_empty());
    state.cancel_settings_overlay();
    assert!(state.workspace_palettes().is_empty());
}

#[test]
fn targeted_picker_use_global_sends_none() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(Some("nord"))));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("frame");
    state.open_workspace_theme_picker("ws_1".into());
    state.select_settings_choice(0);
    let mut outcome = ClientShellInput::default();
    state.apply_settings_choice(&mut outcome);
    let [ClientShellAction::Endpoint { request, .. }] = &outcome.actions[..] else {
        panic!("apply should send one endpoint request");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::WorkspaceSetTheme(params)
            if params.workspace_id == "ws_1" && params.theme.is_none()
    ));
}

#[test]
fn global_picker_is_unchanged() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.open_settings_overlay();
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::Settings(settings)) => {
            assert!(matches!(settings.target, ClientSettingsTarget::Global));
            assert_eq!(settings.sections(), ClientSettingsSection::ALL);
            assert_eq!(settings.theme_choices().len(), crate::config::THEME_NAMES.len());
        }
        _ => panic!("settings overlay"),
    }
    state.cancel_settings_overlay();
}
```

`select_settings_choice` must trigger the theme preview for the Theme section the same way `move_settings_selection` does; check `settings.rs:125-140` (the end of `select_settings_choice`) and make sure it calls `preview_selected_theme` when the section is `Theme`. If it does not, add that call.

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run --locked --bin herdr space_themes`
Expected: compile errors for `ClientContextMenuAction::Theme`, `ClientSettingsTarget`, `open_workspace_theme_picker`, `sections`, `theme_choices`.

- [ ] **Step 3: State types**

`src/client/shell/state.rs`: add `Theme,` to `ClientContextMenuAction` after `Rename,`. Add:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ClientSettingsTarget {
    Global,
    Workspace { workspace_id: String },
}
```

and in `ClientSettingsOverlay` add `pub(super) target: ClientSettingsTarget,` as the first field and `pub(super) original_workspace_theme: Option<String>,` after `original_palette`. Add:

```rust
impl ClientSettingsOverlay {
    pub(super) fn sections(&self) -> &'static [ClientSettingsSection] {
        match self.target {
            ClientSettingsTarget::Global => ClientSettingsSection::ALL,
            ClientSettingsTarget::Workspace { .. } => &[ClientSettingsSection::Theme],
        }
    }

    /// Theme list rows. For a space target the first row means "follow the global theme".
    pub(super) fn theme_choices(&self) -> Vec<&'static str> {
        let mut choices = Vec::with_capacity(crate::config::THEME_NAMES.len() + 1);
        if matches!(self.target, ClientSettingsTarget::Workspace { .. }) {
            choices.push("use global theme");
        }
        choices.extend(crate::config::THEME_NAMES.iter().copied());
        choices
    }

    /// Theme name for a row index, `None` for the "use global theme" row.
    pub(super) fn theme_for_choice(&self, index: usize) -> Option<Option<&'static str>> {
        let choices = self.theme_choices();
        let label = *choices.get(index)?;
        Some((label != "use global theme").then_some(label))
    }
}
```

- [ ] **Step 4: Context menu**

`src/client/shell/context_menu.rs` `items()`: insert `item("Theme...", Action::Theme),` immediately after `item("Rename", Action::Rename),` in all four `Workspace` arms.

`activate_workspace_context_action`: add before the `Close` arm:

```rust
            ClientContextMenuAction::Theme => self.open_workspace_theme_picker(workspace_id),
```

- [ ] **Step 5: Settings logic**

`src/client/shell/settings.rs`:

`open_settings_overlay` sets `target: ClientSettingsTarget::Global,` and `original_workspace_theme: None,` and `workspace_preview: None,`.

Add:

```rust
    pub(super) fn open_workspace_theme_picker(&mut self, workspace_id: String) {
        let current = self
            .snapshot
            .as_deref()
            .and_then(|snapshot| {
                snapshot
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.workspace_id == workspace_id)
            })
            .and_then(|workspace| workspace.theme.as_deref())
            .and_then(crate::config::canonical_theme_name)
            .map(str::to_owned);
        let selected = current
            .as_deref()
            .and_then(|name| crate::config::THEME_NAMES.iter().position(|n| *n == name))
            .map_or(0, |index| index + 1);
        self.overlay = Some(ClientShellOverlay::Settings(ClientSettingsOverlay {
            target: ClientSettingsTarget::Workspace { workspace_id },
            section: ClientSettingsSection::Theme,
            selected,
            original_theme_name: self.config.theme_name.clone(),
            original_palette: self.config.palette.clone(),
            original_workspace_theme: current,
            workspace_preview: None,
            integrations: Vec::new(),
            integration_messages: Vec::new(),
            loading_integrations: false,
            installing_integrations: false,
        }));
    }
```

`move_settings_section`: replace both uses of `ClientSettingsSection::ALL` with `settings.sections()` (bind `let sections = settings.sections();` before mutating).

`settings_choice_count`: `ClientSettingsSection::Theme => settings.theme_choices().len(),`.

`preview_selected_theme`:

```rust
    fn preview_selected_theme(&mut self) {
        let Some(ClientShellOverlay::Settings(settings)) = self.overlay.as_mut() else {
            return;
        };
        match settings.target.clone() {
            ClientSettingsTarget::Global => {
                let Some(name) = crate::config::THEME_NAMES.get(settings.selected) else {
                    return;
                };
                self.config.theme_name = (*name).to_owned();
                self.config.palette =
                    crate::app::client_palette_for_theme(&self.config.theme_runtime, name);
            }
            ClientSettingsTarget::Workspace { workspace_id } => {
                let Some(theme) = settings.theme_for_choice(settings.selected) else {
                    return;
                };
                settings.workspace_preview = Some((workspace_id, theme.map(str::to_owned)));
            }
        }
    }
```

`cancel_settings_overlay` is already correct: taking the overlay drops the preview; restoring the global name/palette is a no-op for a workspace target because they were never changed.

`apply_settings_choice` `Theme` arm:

```rust
            ClientSettingsSection::Theme => match settings.target.clone() {
                ClientSettingsTarget::Global => {
                    let Some(name) = crate::config::THEME_NAMES.get(selected).copied() else {
                        return;
                    };
                    if self.save_settings_edit(crate::config::ConfigEdit::Theme(name), outcome) {
                        self.overlay = None;
                    }
                }
                ClientSettingsTarget::Workspace { workspace_id } => {
                    let Some(theme) = settings.theme_for_choice(selected) else {
                        return;
                    };
                    self.overlay = None;
                    self.push_endpoint_method(
                        crate::api::schema::Method::WorkspaceSetTheme(
                            crate::api::schema::WorkspaceSetThemeParams {
                                workspace_id,
                                theme: theme.map(str::to_owned),
                            },
                        ),
                        outcome,
                    );
                    outcome.repaint = true;
                }
            },
```

`settings` is borrowed immutably at the top of `apply_settings_choice` via `as_ref()`; clone `target` and compute `theme` before the `self.overlay = None` assignment (the code above does this by evaluating `settings.theme_for_choice(selected)` first; if the borrow checker objects, bind `let target = settings.target.clone(); let theme = settings.theme_for_choice(selected);` right after `let selected = settings.selected;` and use those locals).

- [ ] **Step 6: Overlay rendering**

`src/client/shell/settings_overlay.rs`: in the tab loop replace `for section in ClientSettingsSection::ALL {` with `for section in settings.sections() {`.

In the `ClientSettingsSection::Theme` render arm, replace the iteration over `crate::config::THEME_NAMES` with the overlay's choices:

```rust
        ClientSettingsSection::Theme => {
            let choices = settings.theme_choices();
            let visible = usize::from(content.height);
            let scroll = settings.selected.saturating_sub(visible.saturating_sub(1));
            let current_name: Option<&str> = match settings.target {
                ClientSettingsTarget::Global => Some(settings.original_theme_name.as_str()),
                ClientSettingsTarget::Workspace { .. } => settings.original_workspace_theme.as_deref(),
            };
            for (visible_index, (index, name)) in choices
                .iter()
                .enumerate()
                .skip(scroll)
                .take(visible)
                .enumerate()
            {
                let rect = Rect::new(
                    content.x,
                    content.y + visible_index as u16,
                    content.width,
                    1,
                );
                let is_current = match settings.theme_for_choice(index) {
                    Some(None) => current_name.is_none(),
                    Some(Some(theme)) => current_name.is_some_and(|current| {
                        super::super::settings::normalized_theme_name(theme)
                            == super::super::settings::normalized_theme_name(current)
                    }),
                    None => false,
                };
                draw_choice(buffer, rect, name, index == settings.selected, is_current, palette);
                choice_hits.push((rect, index));
            }
        }
```

Add `ClientSettingsTarget` to the `use` list at the top of `settings_overlay.rs` if it is not glob-imported (the file uses `use super::super::state::{..}` style imports; mirror how `ClientSettingsSection` is imported).

- [ ] **Step 7: Run tests**

Run: `cargo nextest run --locked --bin herdr space_themes && cargo nextest run --locked --bin herdr client::shell`
Expected: PASS. `context_menus_capture_stable_targets_and_route_actions` in `chrome_context.rs` clicks `context_menu_rows[0]` expecting Rename; that still holds because `Theme...` is inserted at index 1.

- [ ] **Step 8: Commit**

```bash
cargo fmt --all
git add src/client/shell
git commit -m "feat(client): pick a per-space theme from the space context menu"
```

---

### Task 10: Full verification, docs, and spec reconciliation

**Files:**
- Modify: `docs/superpowers/specs/2026-09-02-focus-follows-mouse-and-space-themes-design.md` (two small reconciliations)
- Modify: `CHANGELOG.md` top "Unreleased" section (look at the top of the file for the current heading style and add two bullets)

- [ ] **Step 1: Reconcile the spec with what was built**

Edit the spec: in "Palette resolution", change the server cache description from "keyed by workspace id" to "keyed by canonical theme name, exposed through `palette_for_workspace(workspace_id)`". In "Behaviour" for the sidebar, add one sentence: "In the collapsed sidebar, the space number is drawn in the space accent colour instead of a bar, because the two-column number leaves no spare column." In "Rendering changes", remove the `composition.rs` bullet (the client does not draw pane frames; the server does).

- [ ] **Step 2: Changelog**

Add under the unreleased heading in `CHANGELOG.md` (match surrounding bullet style):

```
- Added `ui.focus_follows_mouse`, an opt-in setting (also in Settings → focus) that focuses the pane under the pointer without a click.
- Spaces can now carry their own theme: right-click a space → Theme... to pick one. It colours that space's pane borders, tab bar, and sidebar row; everything else keeps the global theme, and new spaces follow the global theme until overridden.
```

- [ ] **Step 3: Format, lint, unit tests**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo nextest run --locked --bin herdr
python3 scripts/config_reference_check.py
```

Expected: all clean. Fix anything reported and re-run.

- [ ] **Step 4: Integration tests and architecture checks**

Run: `cargo nextest run --locked --status-level fail --final-status-level fail --failure-output final --success-output never`
Expected: PASS. This runs the CLI/API integration tests, which spawn the freshly built fork binary with isolated config/session paths through `tests/cli/harness.rs`. If a test fails on `tests/fixtures/endpoint-method-shapes-v1.json` or the schema artifact, revisit Task 5 Step 7.

Also run `just ui-hot-path-architecture-test` (check its recipe body in `justfile:27` first; it is a Python/cargo check that does not touch the user's environment).

- [ ] **Step 5: Commit and push the branch to the fork**

```bash
git add -f docs/superpowers/specs docs/superpowers/plans
git add CHANGELOG.md
git commit -m "docs: changelog and spec reconciliation for focus-follows-mouse and space themes"
git push -u origin feature/focus-follows-mouse-and-space-themes
```

Do not open a pull request against `herdrdev/herdr`.
