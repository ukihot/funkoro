use leafwing_input_manager::prelude::Buttonlike;

/// TOML設定文字列からの読み込み用（doc例："KeyA"・"Space"・"MouseLeft"・
/// "GamepadSouth"）。網羅的なパーサではなく、実用上よく使うキー・ボタンを
/// カバーする（未対応の名前は`None`を返し、呼び出し側が警告ログを出す）。
///
/// leafwing-input-managerの`InputMap::insert_boxed`にそのまま渡せる
/// `Box<dyn Buttonlike>`を返す（`KeyCode`/`MouseButton`/`GamepadButton`は
/// いずれも`Buttonlike`を実装している）。
pub(crate) fn parse_buttonlike(name: &str) -> Option<Box<dyn Buttonlike>> {
    if let Some(rest) = name.strip_prefix("Mouse") {
        return parse_mouse_button(rest).map(|button| Box::new(button) as Box<dyn Buttonlike>);
    }
    if let Some(rest) = name.strip_prefix("Gamepad") {
        return parse_gamepad_button(rest).map(|button| Box::new(button) as Box<dyn Buttonlike>);
    }
    parse_key_code(name).map(|key| Box::new(key) as Box<dyn Buttonlike>)
}

fn parse_mouse_button(name: &str) -> Option<bevy::input::mouse::MouseButton> {
    use bevy::input::mouse::MouseButton;
    Some(match name {
        "Left" => MouseButton::Left,
        "Right" => MouseButton::Right,
        "Middle" => MouseButton::Middle,
        "Back" => MouseButton::Back,
        "Forward" => MouseButton::Forward,
        other => {
            let index: u16 = other.parse().ok()?;
            MouseButton::Other(index)
        }
    })
}

fn parse_gamepad_button(name: &str) -> Option<bevy::input::gamepad::GamepadButton> {
    use bevy::input::gamepad::GamepadButton;
    Some(match name {
        "South" => GamepadButton::South,
        "East" => GamepadButton::East,
        "North" => GamepadButton::North,
        "West" => GamepadButton::West,
        "LeftTrigger" => GamepadButton::LeftTrigger,
        "LeftTrigger2" => GamepadButton::LeftTrigger2,
        "RightTrigger" => GamepadButton::RightTrigger,
        "RightTrigger2" => GamepadButton::RightTrigger2,
        "Select" => GamepadButton::Select,
        "Start" => GamepadButton::Start,
        "Mode" => GamepadButton::Mode,
        "LeftThumb" => GamepadButton::LeftThumb,
        "RightThumb" => GamepadButton::RightThumb,
        "DPadUp" => GamepadButton::DPadUp,
        "DPadDown" => GamepadButton::DPadDown,
        "DPadLeft" => GamepadButton::DPadLeft,
        "DPadRight" => GamepadButton::DPadRight,
        _ => return None,
    })
}

/// アルファベット・数字・よく使う特殊キー・矢印キー・ファンクションキーを
/// カバーする。`KeyCode`は変種が非常に多く全網羅はしない
/// （実際に必要になったキーをここへ追記していく運用でよい）。
fn parse_key_code(name: &str) -> Option<bevy::input::keyboard::KeyCode> {
    use bevy::input::keyboard::KeyCode;

    if let Some(letter) = name.strip_prefix("Key") {
        return match letter {
            "A" => Some(KeyCode::KeyA),
            "B" => Some(KeyCode::KeyB),
            "C" => Some(KeyCode::KeyC),
            "D" => Some(KeyCode::KeyD),
            "E" => Some(KeyCode::KeyE),
            "F" => Some(KeyCode::KeyF),
            "G" => Some(KeyCode::KeyG),
            "H" => Some(KeyCode::KeyH),
            "I" => Some(KeyCode::KeyI),
            "J" => Some(KeyCode::KeyJ),
            "K" => Some(KeyCode::KeyK),
            "L" => Some(KeyCode::KeyL),
            "M" => Some(KeyCode::KeyM),
            "N" => Some(KeyCode::KeyN),
            "O" => Some(KeyCode::KeyO),
            "P" => Some(KeyCode::KeyP),
            "Q" => Some(KeyCode::KeyQ),
            "R" => Some(KeyCode::KeyR),
            "S" => Some(KeyCode::KeyS),
            "T" => Some(KeyCode::KeyT),
            "U" => Some(KeyCode::KeyU),
            "V" => Some(KeyCode::KeyV),
            "W" => Some(KeyCode::KeyW),
            "X" => Some(KeyCode::KeyX),
            "Y" => Some(KeyCode::KeyY),
            "Z" => Some(KeyCode::KeyZ),
            _ => None,
        };
    }
    if let Some(digit) = name.strip_prefix("Digit") {
        return match digit {
            "0" => Some(KeyCode::Digit0),
            "1" => Some(KeyCode::Digit1),
            "2" => Some(KeyCode::Digit2),
            "3" => Some(KeyCode::Digit3),
            "4" => Some(KeyCode::Digit4),
            "5" => Some(KeyCode::Digit5),
            "6" => Some(KeyCode::Digit6),
            "7" => Some(KeyCode::Digit7),
            "8" => Some(KeyCode::Digit8),
            "9" => Some(KeyCode::Digit9),
            _ => None,
        };
    }
    if let Some(f) = name.strip_prefix('F')
        && let Ok(n) = f.parse::<u8>()
        && (1..=12).contains(&n)
    {
        return Some(match n {
            1 => KeyCode::F1,
            2 => KeyCode::F2,
            3 => KeyCode::F3,
            4 => KeyCode::F4,
            5 => KeyCode::F5,
            6 => KeyCode::F6,
            7 => KeyCode::F7,
            8 => KeyCode::F8,
            9 => KeyCode::F9,
            10 => KeyCode::F10,
            11 => KeyCode::F11,
            _ => KeyCode::F12,
        });
    }
    Some(match name {
        "Space" => KeyCode::Space,
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Escape,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        _ => return None,
    })
}
