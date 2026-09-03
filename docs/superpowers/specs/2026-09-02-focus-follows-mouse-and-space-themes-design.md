# Focus follows mouse, and per-space themes

Date: 2026-09-02
Status: approved design, fork branch `feature/focus-follows-mouse-and-space-themes`

## Context

Herdr is a client/server terminal multiplexer for coding agents. The client
shell owns chrome rendering, hit-testing, and mouse routing. The server owns
workspace state, PTYs, split-border rendering, and persistence.

Upstream (`herdrdev/herdr`) does not accept unsolicited pull requests from
accounts outside `.github/APPROVED_CONTRIBUTORS`. This work lives in the
`potable-anarchy/herdr` fork. The eventual upstream path is a GitHub
Discussion that references the fork branch. Two existing discussions cover the
same ground: #1259 and #2230 (focus follows mouse), #1942 (per-space colour).

Both features are opt-in and change nothing for users who do not enable them.

## Feature 1: focus follows mouse

### Behaviour

When `ui.focus_follows_mouse = true`, moving the pointer into a pane focuses
that pane without a click. Nothing else changes: clicking still focuses,
keyboard focus movement still works, and the sidebar, tab bar, status bars,
and agents panel do not react to hover.

Focus is requested only when all of the following hold:

- the setting is on
- `config.ui.mouse_capture` is on
- the client shell mode is `Terminal` (not Copy, Navigate, Resize)
- no overlay is open (settings, help, menus, dialogs, onboarding, and so on)
- no chrome drag is in progress (split resize, tab drag, workspace drag,
  sidebar dividers, scrollbars)
- no pane mouse gesture is in progress (button held over a mouse-reporting pane)
- no text selection drag is in progress
- the pointer is inside a non-popup pane's inner rect
- that pane is not already the focused pane

Button-held motion arrives as `Drag`, not `Moved`, so text selection and split
resizing keep the pane they started in with no extra logic. The mobile layout
is unaffected because it has one visible pane.

After the focus request, the existing `Moved` handling still runs, so pane
apps that track motion keep receiving it.

### Configuration

- Field: `UiConfig::focus_follows_mouse: bool`, default `false`, with a doc
  comment. Serde default keeps old config files valid.
- Projected to `ClientShellConfig::focus_follows_mouse` and copied in
  `apply_live_config`, so `herdr config reload` and the settings overlay apply
  it live with no restart. It is client-only; the server does not need it.
- `ConfigEdit::FocusFollowsMouse(bool)` writes `focus_follows_mouse` under
  `[ui]` with the existing comment-preserving `upsert_section_bool`.
- The shipped config template comment in `src/main.rs` and
  `docs/next/website/src/data/config-reference.json` gain the key, so
  `scripts/config_reference_check.py` stays green.

### Settings overlay

A new `ClientSettingsSection::Focus`, labelled `focus`, placed after `sound`.
It renders through `render_choice_section` with the title "focus follows
mouse", the description "focus the pane under the pointer without clicking",
and the choices `on` / `off`. Keyboard and mouse behave exactly as the sound
section: the choice applies immediately on click or Enter, then
`save_settings_edit` writes the config and reloads it.

## Feature 2: per-space themes

### Behaviour

Every space has an optional theme override. By default it is absent and the
space follows the global theme, including any later change to the global
theme and any auto light/dark switch. When a space has an override, that
named theme is used instead, resolved through the same pipeline as the global
theme so `[theme.custom]` token overrides apply consistently.

What the space theme paints:

- The pane frames of that space while it is the active space: split borders
  and pane titles drawn by the server, and the pane border drawn by the
  client.
- The tab bar while that space is active.
- The space's own row in the spaces sidebar at all times, whether or not the
  space is active: label, secondary text, and row highlight colours come from
  the space palette, and a one-column accent bar in the space palette's
  `accent` colour is drawn at the leading edge of the row. The bar occupies
  the leading column of the row in the expanded sidebar. In the collapsed
  sidebar, the space number is drawn bold in the space accent colour instead
  of a bar, because the two-column number leaves no spare column.

What stays on the global theme: sidebar background, header, footer, and
separator; the agents panel; status bars and the mode bar; every overlay and
modal; the mobile layout chrome.

Pane content is never affected. Terminal cell colours come from the host
terminal palette and are out of scope.

### Data and persistence

- `Workspace::theme: Option<String>`, canonical theme name, `None` by default.
- `WorkspaceSnapshot::theme: Option<String>` with
  `#[serde(default, skip_serializing_if = "Option::is_none")]`, so existing
  session files load and files without overrides do not change shape.
- Restore copies the field; snapshot writes it.
- `Workspace::new*` constructors and `from_existing_pane` initialise it to
  `None`. New spaces therefore inherit the global theme by following it.

### API

New method `WorkspaceSetTheme` with params `{ workspace_id: String, theme: Option<String> }`.
Handler:

1. Resolve the workspace by id, error if missing.
2. If `theme` is `Some`, canonicalise it with `canonical_theme_name` and reject
   with an invalid-params error if it is not in `THEME_NAMES`.
3. Store it, recompute the cached palette, mark the session for persistence,
   and bump the snapshot revision so clients re-render.

No CLI subcommand is added on this branch; the method is reachable through
the endpoint API and the client UI.

### Palette resolution

Server side, `AppState` gains a `workspace_theme_palettes: HashMap<String, Palette>`
keyed by canonical theme name, holding a resolved palette for every theme
some space overrides to. It is rebuilt when a space theme is set or cleared,
on config reload, and on host appearance change (wherever
`refresh_effective_app_theme` runs), and once after session restore. A helper
`AppState::palette_for_workspace(&self, workspace_id) -> &Palette` looks up the
space's theme name and returns that palette or `self.palette`.

Client side, the per-space snapshot entry (`ClientShellWorkspace` or its
equivalent) carries `theme: Option<String>`. `ClientShellState` gains a
`theme_palettes: HashMap<String, Palette>` cache keyed by theme name,
populated on demand with `client_palette_for_theme` and cleared on config
reload and appearance change. A helper `palette_for_workspace(&self, id) ->
&Palette` mirrors the server one. During theme preview (see below) the
overlay's preview palette takes precedence for the targeted space.

### Rendering changes

- `src/ui/panes.rs`: split borders and pane titles use
  `app.palette_for_workspace(&ws.id)` in place of `app.palette`. Pane frames
  are drawn only by the server; the client blits the surface it receives.
- `src/client/shell/tabs.rs`: the tab bar uses the active space palette.
- `src/client/shell/sidebar.rs`, expanded and collapsed: each row uses its own
  space palette for label, secondary, and highlight colours, and draws the
  accent bar when the space has an override. Rows without an override render
  exactly as today.

### Context menu and picker

- `ClientContextMenuAction::Theme` and a `Theme...` item in the workspace arm
  of `ClientContextMenuOverlay::items()`, placed after `Rename` for every
  workspace variant.
- Activating it opens the settings overlay with a new field
  `target: ClientSettingsTarget`, an enum of `Global` and
  `Workspace { workspace_id }`. Existing entry points use `Global`.
- With a workspace target: the section tabs show only `theme`; the theme list
  gains a leading entry `use global theme`; the cursor starts on the space's
  current override or on the global entry when it has none; the applied
  marker follows the same rule.
- Preview with a workspace target writes into a
  `preview_workspace_palette: Option<(String, Palette)>` on the overlay state
  rather than the global config palette; the client `palette_for_workspace`
  helper checks it first. Cancel clears it. Apply sends `WorkspaceSetTheme`
  with the chosen canonical name, or `None` for the global entry, clears the
  preview, and closes the overlay. It never writes the config file.
- Global-target behaviour is unchanged.

## Error handling

- Unknown theme names in a session file: restore keeps the name but
  `palette_for_workspace` falls back to the global palette when resolution
  fails, and the picker shows the global entry as current. Nothing panics.
- `WorkspaceSetTheme` with an unknown workspace or theme returns an API error
  and changes nothing.
- Config reload with an invalid `[ui]` section continues to skip the whole
  section, so `focus_follows_mouse` keeps its previous value, matching the
  existing rule.

## Testing

Focus follows mouse, in `src/client/shell/tests/`:

- moved event over an unfocused pane emits `PaneFocus` when enabled
- no request when disabled (default)
- no request when the hovered pane is already focused
- no request over the sidebar or tab bar
- no request while a split drag, selection drag, or pane gesture is active
- no request while an overlay is open
- no request outside `Terminal` mode
- the settings overlay `focus` section applies immediately and emits the
  config write plus `ServerReloadConfig`

Config, in `src/config/model.rs` and `src/config/io.rs` tests: default is
off, TOML parse of `true`, `ConfigEdit::FocusFollowsMouse` upserts under
`[ui]`.

Per-space theme:

- snapshot round-trip with and without a theme, and loading a snapshot
  written before the field existed
- `WorkspaceSetTheme` rejects unknown themes and unknown workspaces, and
  canonicalises aliases
- `palette_for_workspace` returns the override palette, falls back to global,
  and falls back on an unresolvable name
- server render test: split-border colour follows the active space palette
- client tests: context menu contains `Theme...` for a workspace target;
  opening it seeds the cursor correctly; the list includes `use global theme`;
  apply emits `WorkspaceSetTheme` with the right payload; cancel clears the
  preview; the sidebar row for a themed space exposes the accent bar cell in
  the composed buffer
- the existing built-in theme contrast assertions continue to pass

## Out of scope

- hover highlighting of panes
- hover-driven focus for sidebar or agent rows
- per-space terminal palettes for pane content
- persisting space themes in `config.toml`
- a plugin event for pane hover
