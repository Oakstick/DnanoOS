use winit::keyboard::KeyCode;

use super::*;

fn pad(buttons: &[PadButton], kind: Device) -> PadState {
    PadState { buttons: buttons.iter().copied().collect(), kind: Some(kind), ..Default::default() }
}

/// One update's reading: the pad set, polled, and what it pressed (then flushed).
fn step(i: &mut Input, p: Option<PadState>) -> (Vec<Action>, Option<Dir>) {
    i.set_pad(p);
    i.poll(1.0 / 120.0);
    let mut got: Vec<Action> = i.presses().collect();
    got.sort_by_key(|a| format!("{a:?}"));
    let nav = i.nav;
    i.flush();
    (got, nav)
}

#[test]
fn keys_combine_into_eight_directions() {
    let mut i = Input::new();
    i.hold_dir(Dir::Up, true);
    i.hold_dir(Dir::Right, true);
    let (x, y) = i.move_vector();
    let d = std::f64::consts::FRAC_1_SQRT_2;
    assert!((x - d).abs() < 1e-3 && (y + d).abs() < 1e-3, "up and right run diagonally: {x},{y}");
    i.hold_dir(Dir::Down, true);
    assert_eq!(i.move_vector(), (1.0, 0.0), "up and down cancel");
    i.clear_direction();
    assert_eq!(i.move_vector(), (0.0, 0.0));
}

#[test]
fn in_play_the_d_pad_chooses_potions_opens_the_map_and_switches_the_camera_and_never_runs() {
    let mut i = Input::new();
    i.set_context(Context::Play);
    for (b, want) in [
        (PadButton::DPadLeft, Action::PotionPrev),
        (PadButton::DPadRight, Action::PotionNext),
        (PadButton::DPadUp, Action::Map),
        (PadButton::DPadDown, Action::Camera),
    ] {
        let (got, nav) = step(&mut i, Some(pad(&[b], Device::Xbox)));
        assert_eq!(got, vec![want], "{b:?}");
        assert_eq!(nav, None, "{b:?} moves no highlight in play");
        assert_eq!(i.direction(), Dir::None, "{b:?} does not run");
        assert_eq!(i.move_vector(), (0.0, 0.0));
        // Held, it pressed once.
        assert!(step(&mut i, Some(pad(&[b], Device::Xbox))).0.is_empty());
        step(&mut i, Some(pad(&[], Device::Xbox)));
    }
    // A reading with the D-pad as a direction (no buttons) is the same D-pad.
    let p = PadState { dpad: Some(Dir::Right), ..Default::default() };
    assert_eq!(step(&mut i, Some(p)).0, vec![Action::PotionNext]);
    assert_eq!(i.direction(), Dir::None);
    // The left stick still runs.
    step(&mut i, None);
    let p = PadState { left: (0.0, -1.0), ..Default::default() };
    step(&mut i, Some(p));
    assert_eq!(i.direction(), Dir::Up);
}

#[test]
fn in_menus_the_d_pad_and_the_stick_move_and_the_face_buttons_choose() {
    let mut i = Input::new();
    i.set_context(Context::Menu);
    let (got, nav) = step(&mut i, Some(pad(&[PadButton::DPadDown], Device::Xbox)));
    assert_eq!((got, nav), (vec![], Some(Dir::Down)));
    assert_eq!(step(&mut i, Some(pad(&[PadButton::DPadDown], Device::Xbox))).1, None, "a held D-pad moves once");
    step(&mut i, Some(pad(&[], Device::Xbox)));
    assert_eq!(step(&mut i, Some(pad(&[PadButton::South], Device::Xbox))).0, vec![Action::Confirm]);
    step(&mut i, Some(pad(&[], Device::Xbox)));
    assert_eq!(step(&mut i, Some(pad(&[PadButton::East], Device::Xbox))).0, vec![Action::Back, Action::Pause]);
    step(&mut i, Some(pad(&[], Device::Xbox)));
    assert_eq!(step(&mut i, Some(pad(&[PadButton::Menu], Device::Xbox))).0, vec![Action::Back, Action::Pause]);
    step(&mut i, Some(pad(&[], Device::Xbox)));
    assert_eq!(step(&mut i, Some(pad(&[PadButton::LeftBumper], Device::Xbox))).0, vec![Action::TabPrev]);
    step(&mut i, Some(pad(&[], Device::Xbox)));
    assert_eq!(step(&mut i, Some(pad(&[PadButton::RightBumper], Device::Xbox))).0, vec![Action::TabNext]);
    step(&mut i, Some(pad(&[], Device::Xbox)));
    assert_eq!(step(&mut i, Some(pad(&[PadButton::North], Device::Xbox))).0, vec![Action::Secondary]);
    step(&mut i, Some(pad(&[], Device::Xbox)));
    // The X button and the triggers do nothing in a menu.
    assert!(step(&mut i, Some(pad(&[PadButton::West, PadButton::LeftTrigger, PadButton::RightTrigger], Device::Xbox))).0.is_empty());
    step(&mut i, Some(pad(&[], Device::Xbox)));
    // The left stick moves the highlight.
    let p = PadState { left: (1.0, 0.0), kind: Some(Device::Xbox), ..Default::default() };
    assert_eq!(step(&mut i, Some(p)).1, Some(Dir::Right));
}

#[test]
fn a_button_held_through_a_change_of_context_presses_nothing_new() {
    let mut i = Input::new();
    i.set_context(Context::Play);
    assert_eq!(step(&mut i, Some(pad(&[PadButton::South], Device::Xbox))).0, vec![Action::Jump]);
    i.set_context(Context::Menu);
    assert!(step(&mut i, Some(pad(&[PadButton::South], Device::Xbox))).0.is_empty(), "the jump's A does not choose in the menu");
    assert!(i.held(Action::Confirm) && !i.held(Action::Jump));
    step(&mut i, Some(pad(&[], Device::Xbox)));
    assert!(!i.held(Action::Confirm));
    // A key the same: it lets go of what it pressed, whatever the context now.
    i.set_context(Context::Play);
    i.mouse_button(winit::event::MouseButton::Right, true);
    assert!(i.held(Action::Block));
    i.set_context(Context::Menu);
    i.mouse_button(winit::event::MouseButton::Right, false);
    assert!(!i.held(Action::Block));
}

#[test]
fn the_triggers_and_bumpers_in_play() {
    let mut i = Input::new();
    i.set_context(Context::Play);
    let all = [PadButton::LeftTrigger, PadButton::RightTrigger, PadButton::LeftBumper, PadButton::RightBumper];
    let (mut got, _) = step(&mut i, Some(pad(&all, Device::Xbox)));
    got.sort_by_key(|a| format!("{a:?}"));
    assert_eq!(got, vec![Action::Angel, Action::Block, Action::Drink, Action::Step]);
    assert!(i.held(Action::Block), "the shield is held while LB is");
}

#[test]
fn the_keyboard_reads_through_the_layer_of_the_moment() {
    let mut i = Input::new();
    let tap = |i: &mut Input, k: KeyCode| {
        i.key_down(k, false);
        i.key_up(k);
        let mut got: Vec<Action> = i.presses().collect();
        got.sort_by_key(|a| format!("{a:?}"));
        let nav = i.nav;
        i.flush();
        (got, nav)
    };
    i.set_context(Context::Play);
    assert_eq!(tap(&mut i, KeyCode::Space).0, vec![Action::Jump]);
    assert_eq!(tap(&mut i, KeyCode::KeyQ).0, vec![Action::Drink]);
    assert_eq!(tap(&mut i, KeyCode::KeyE).0, vec![Action::Call]);
    assert_eq!(tap(&mut i, KeyCode::KeyM).0, vec![Action::Map]);
    assert_eq!(tap(&mut i, KeyCode::Tab).0, vec![Action::Map]);
    assert_eq!(tap(&mut i, KeyCode::ShiftLeft).0, vec![Action::Step]);
    assert_eq!(tap(&mut i, KeyCode::KeyR).0, vec![Action::Angel]);
    assert_eq!(tap(&mut i, KeyCode::Digit3).0, vec![Action::Slot(2)]);
    assert_eq!(tap(&mut i, KeyCode::Escape).0, vec![Action::Pause]);
    assert_eq!(tap(&mut i, KeyCode::F1).0, vec![Action::Cheat]);
    i.set_context(Context::Menu);
    assert_eq!(tap(&mut i, KeyCode::Space).0, vec![Action::Confirm]);
    assert_eq!(tap(&mut i, KeyCode::Enter).0, vec![Action::Confirm]);
    assert_eq!(tap(&mut i, KeyCode::KeyQ).0, vec![Action::TabPrev]);
    assert_eq!(tap(&mut i, KeyCode::KeyE).0, vec![Action::TabNext]);
    assert_eq!(tap(&mut i, KeyCode::Tab).0, vec![Action::TabNext]);
    assert_eq!(tap(&mut i, KeyCode::Escape).0, vec![Action::Back, Action::Pause]);
    assert_eq!(tap(&mut i, KeyCode::KeyS), (vec![], Some(Dir::Down)));
    assert_eq!(tap(&mut i, KeyCode::F1).0, vec![Action::Cheat], "the testing keys never move");
    // Ctrl+M mutes, in either layer, and opens no map.
    for ctx in [Context::Play, Context::Menu] {
        i.set_context(ctx);
        i.key_down(KeyCode::ControlLeft, false);
        assert_eq!(tap(&mut i, KeyCode::KeyM).0, vec![Action::Mute]);
        i.key_up(KeyCode::ControlLeft);
    }
    // W A S D run in play (and a key held through a menu keeps running after it).
    i.set_context(Context::Play);
    i.key_down(KeyCode::KeyW, false);
    i.set_context(Context::Menu);
    i.set_context(Context::Play);
    assert_eq!(i.direction(), Dir::Up);
    i.key_up(KeyCode::KeyW);
    assert_eq!(i.direction(), Dir::None);
    // Rebound, the key does what it is bound to.
    i.bindings.bind(Action::Jump, Control::Key(KeyCode::KeyJ));
    assert_eq!(tap(&mut i, KeyCode::KeyJ).0, vec![Action::Jump]);
}

#[test]
fn the_mouse_attacks_shields_and_turns_the_belt_in_play_and_taps_in_menus() {
    let mut i = Input::new();
    i.cursor_moved_to(10.0, 10.0);
    i.set_context(Context::Play);
    i.mouse_button(winit::event::MouseButton::Left, true);
    assert!(i.was_pressed(&[Action::Attack]) && !i.was_pressed(&[Action::Tap]));
    i.mouse_button(winit::event::MouseButton::Left, false);
    i.flush();
    i.wheel(1.0);
    assert!(i.was_pressed(&[Action::PotionPrev]) && !i.held(Action::PotionPrev), "a notch presses, and holds nothing");
    i.flush();
    i.wheel(-0.5);
    assert!(!i.was_pressed(&[Action::PotionNext]), "half a notch is not a notch");
    i.wheel(-0.5);
    assert!(i.was_pressed(&[Action::PotionNext]));
    i.flush();
    i.set_context(Context::Menu);
    i.mouse_button(winit::event::MouseButton::Left, true);
    assert!(i.was_pressed(&[Action::Tap]) && !i.was_pressed(&[Action::Attack]));
    assert_eq!(i.click, Some((10.0, 10.0)));
    i.mouse_button(winit::event::MouseButton::Left, false);
    assert!(!i.held(Action::Tap));
}

#[test]
fn the_last_device_follows_what_was_used() {
    let mut i = Input::new();
    assert_eq!(i.last_device(), Device::KeyboardMouse);
    step(&mut i, Some(pad(&[PadButton::South], Device::PlayStation)));
    assert_eq!(i.last_device(), Device::PlayStation);
    // A small nudge of the mouse is not use; a real move is.
    i.mouse_motion(1.0, 2.0);
    assert_eq!(i.last_device(), Device::PlayStation);
    i.mouse_motion(8.0, 3.0);
    assert_eq!(i.last_device(), Device::KeyboardMouse);
    // A stick pushed out past its deadzone; resting inside it is not use.
    let stick = |x: f32| PadState { left: (x, 0.0), kind: Some(Device::Xbox), ..Default::default() };
    step(&mut i, Some(stick(0.2)));
    assert_eq!(i.last_device(), Device::KeyboardMouse);
    step(&mut i, Some(stick(0.9)));
    assert_eq!(i.last_device(), Device::Xbox);
    i.key_down(KeyCode::KeyW, false);
    assert_eq!(i.last_device(), Device::KeyboardMouse);
    // Held on, the stick does not take it back; pushed out again, it does.
    step(&mut i, Some(stick(0.9)));
    assert_eq!(i.last_device(), Device::KeyboardMouse);
    step(&mut i, Some(stick(0.0)));
    step(&mut i, Some(stick(-1.0)));
    assert_eq!(i.last_device(), Device::Xbox);
    i.wheel(1.0);
    assert_eq!(i.last_device(), Device::KeyboardMouse);
    step(&mut i, Some(pad(&[PadButton::West], Device::Generic)));
    assert_eq!(i.last_device(), Device::Generic);
    i.touch(1, winit::event::TouchPhase::Started, 5.0, 5.0);
    assert_eq!(i.last_device(), Device::KeyboardMouse);
    // A bot's play is no device used.
    let bot = PadState { actions: [Action::Attack].into_iter().collect(), left: (1.0, 0.0), ..Default::default() };
    step(&mut i, None);
    step(&mut i, Some(bot));
    assert_eq!(i.last_device(), Device::KeyboardMouse);
    i.set_last_device(Device::PlayStation);
    assert_eq!(i.last_device(), Device::PlayStation);
}

#[test]
fn a_bots_pad_presses_as_it_always_did() {
    let mut i = Input::new();
    for ctx in [Context::Play, Context::Menu] {
        i.set_context(ctx);
        let p = PadState { actions: [Action::Attack, Action::Block].into_iter().collect(), ..Default::default() };
        let (got, _) = step(&mut i, Some(p.clone()));
        assert_eq!(got, vec![Action::Attack, Action::Block], "{ctx:?}");
        assert!(step(&mut i, Some(p)).0.is_empty());
        assert!(i.held(Action::Block));
        step(&mut i, Some(PadState::default()));
        assert!(!i.held(Action::Block));
    }
}

#[test]
fn prompts_name_the_control_on_the_device_used_last() {
    let mut i = Input::new();
    assert_eq!(i.prompt(Action::Angel).as_deref(), Some("R"));
    assert_eq!(i.prompt(Action::Attack).as_deref(), Some("Left click"));
    i.set_last_device(Device::Xbox);
    assert_eq!(i.prompt(Action::Angel).as_deref(), Some("RT"));
    i.set_last_device(Device::PlayStation);
    assert_eq!(i.prompt(Action::Angel).as_deref(), Some("R2"));
    assert_eq!(i.prompt(Action::Confirm).as_deref(), Some("Cross"));
    assert_eq!(i.prompt(Action::MoveUp), None, "the stick runs");
}

#[test]
fn the_shield_by_toggle_is_raised_by_one_press_and_lowered_by_the_next() {
    use crate::settings::Settings;
    let mut i = Input::new();
    i.set_context(Context::Play);
    // Held (the default): up while the right button is down.
    i.mouse_button(winit::event::MouseButton::Right, true);
    assert!(i.held(Action::Block));
    i.mouse_button(winit::event::MouseButton::Right, false);
    assert!(!i.held(Action::Block));
    // By toggle: a click raises it and it stays; the next lowers it.
    i.apply(&Settings { shield_toggle: true, ..Settings::default() });
    i.mouse_button(winit::event::MouseButton::Right, true);
    i.mouse_button(winit::event::MouseButton::Right, false);
    assert!(i.held(Action::Block), "raised by the press, and kept");
    i.mouse_button(winit::event::MouseButton::Right, true);
    assert!(!i.held(Action::Block), "lowered by the next");
    i.mouse_button(winit::event::MouseButton::Right, false);
    // The pad's LB the same, a press at a time (held across polls, it flips once).
    step(&mut i, Some(pad(&[PadButton::LeftBumper], Device::Xbox)));
    step(&mut i, Some(pad(&[PadButton::LeftBumper], Device::Xbox)));
    assert!(i.held(Action::Block));
    step(&mut i, Some(pad(&[], Device::Xbox)));
    assert!(i.held(Action::Block));
    step(&mut i, Some(pad(&[PadButton::LeftBumper], Device::Xbox)));
    assert!(!i.held(Action::Block));
    // A bot's guard (or a script's) is held as ever, the toggle or not.
    let bot = PadState { actions: [Action::Block].into_iter().collect(), ..Default::default() };
    step(&mut i, Some(bot));
    assert!(i.held(Action::Block));
    step(&mut i, Some(PadState::default()));
    assert!(!i.held(Action::Block));
    // A rule that lowers the guard lowers the latch too; switched back to held, it is down.
    i.mouse_button(winit::event::MouseButton::Right, true);
    i.mouse_button(winit::event::MouseButton::Right, false);
    i.inject_release(Action::Block);
    assert!(!i.held(Action::Block));
    i.mouse_button(winit::event::MouseButton::Right, true);
    i.mouse_button(winit::event::MouseButton::Right, false);
    i.apply(&Settings::default());
    assert!(!i.held(Action::Block));
}

#[test]
fn the_mouse_and_sticks_follow_the_settings() {
    use crate::settings::Settings;
    let mut i = Input::new();
    i.mouse_captured = true;
    i.mouse_motion(10.0, 10.0);
    assert_eq!(i.take_look(), (10.0, 10.0));
    i.apply(&Settings { mouse_sensitivity: 200, invert_mouse_y: true, ..Settings::default() });
    i.mouse_motion(10.0, 10.0);
    assert_eq!(i.take_look(), (20.0, -20.0));
    // The right stick: up turned over on its own setting.
    let look = |i: &mut Input, y: f32| {
        step(i, Some(PadState { right: (0.0, y), kind: Some(Device::Xbox), ..Default::default() }));
        i.take_look().1
    };
    assert!(look(&mut i, 1.0) > 0.0, "the stick is not turned over by the mouse's setting");
    i.apply(&Settings { invert_stick_y: true, ..Settings::default() });
    assert!(look(&mut i, 1.0) < 0.0);
    // The deadzones: a push of 0.3 runs only once the deadzone is under it.
    let push = PadState { left: (0.3, 0.0), kind: Some(Device::Xbox), ..Default::default() };
    i.apply(&Settings::default());
    step(&mut i, Some(push.clone()));
    assert_eq!(i.direction(), Dir::None);
    i.apply(&Settings { move_deadzone: 25, ..Settings::default() });
    step(&mut i, Some(push));
    assert_eq!(i.direction(), Dir::Right);
    // And the look's: a nudge of 0.2 turns nothing at the default, something at 0.1.
    i.apply(&Settings::default());
    assert_eq!(look(&mut i, 0.15), 0.0);
    i.apply(&Settings { look_deadzone: 10, ..Settings::default() });
    assert!(look(&mut i, 0.15) > 0.0);
}

#[test]
fn every_control_pressed_is_heard_as_it_is_for_the_rebinding_page() {
    let mut i = Input::new();
    i.set_context(Context::Menu);
    i.key_down(KeyCode::KeyQ, false);
    i.key_down(KeyCode::F1, false);
    i.mouse_button(winit::event::MouseButton::Back, true);
    i.wheel(-1.0);
    assert_eq!(
        i.raw_presses(),
        &[Control::Key(KeyCode::KeyQ), Control::Key(KeyCode::F1), Control::Mouse(MouseButton::Back), Control::WheelDown]
    );
    i.flush();
    assert!(i.raw_presses().is_empty());
    i.set_pad(Some(pad(&[PadButton::RightTrigger], Device::Xbox)));
    i.poll(1.0 / 120.0);
    assert_eq!(i.raw_presses(), &[Control::Pad(PadButton::RightTrigger)]);
    i.flush();
    i.poll(1.0 / 120.0);
    assert!(i.raw_presses().is_empty(), "held, it is not pressed again");
}
