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
