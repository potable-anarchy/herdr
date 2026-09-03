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
    let latte =
        crate::app::client_palette_for_theme(&state.config.theme_runtime, "catppuccin-latte");
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
            assert_eq!(
                settings.theme_choices().len(),
                crate::config::THEME_NAMES.len()
            );
        }
        _ => panic!("settings overlay"),
    }
    state.cancel_settings_overlay();
}
