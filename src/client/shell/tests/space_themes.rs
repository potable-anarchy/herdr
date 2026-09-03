use super::*;

fn themed_snapshot(theme: Option<&str>) -> ClientShellSnapshot {
    let mut projected = snapshot();
    projected.workspaces[0].theme = theme.map(str::to_owned);
    projected
}

fn cell_at(frame: &FrameData, x: u16, y: u16) -> &crate::protocol::CellData {
    &frame.cells[usize::from(y) * usize::from(frame.width) + usize::from(x)]
}

fn packed(color: ratatui::style::Color) -> u32 {
    crate::protocol::color_to_u32(color)
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
    let bar = cell_at(&frame, row.x, row.y);
    assert_eq!(bar.symbol, "▌");
    assert_eq!(bar.fg, packed(nord.accent));
}

#[test]
fn untitled_space_row_has_no_accent_bar() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(None)));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 20).expect("frame");
    let row = state.hits.workspaces[0].rect;
    assert_ne!(cell_at(&frame, row.x, row.y).symbol, "▌");
}

#[test]
fn tab_bar_uses_the_active_space_palette() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(Some("nord"))));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 20).expect("frame");
    let (tab, _) = state.hits.tabs[0].clone();
    let nord = crate::app::client_palette_for_theme(&state.config.theme_runtime, "nord");
    assert_eq!(cell_at(&frame, tab.x, tab.y).bg, packed(nord.accent));
}

#[test]
fn unknown_space_theme_falls_back_to_the_global_palette() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(themed_snapshot(Some("not-a-theme"))));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("frame");
    assert!(state.workspace_palettes().is_empty());
}
