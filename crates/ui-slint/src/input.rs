use petunia_config::keybinds::winit_keys::KeyCode;

pub(crate) fn key_code_from_slint(text: &str) -> Option<KeyCode> {
    let normalized = text.trim();
    if normalized.len() == 1 {
        let character = normalized.chars().next()?.to_ascii_uppercase();
        return match character {
            'A' => Some(KeyCode::KeyA),
            'B' => Some(KeyCode::KeyB),
            'C' => Some(KeyCode::KeyC),
            'D' => Some(KeyCode::KeyD),
            'E' => Some(KeyCode::KeyE),
            'F' => Some(KeyCode::KeyF),
            'G' => Some(KeyCode::KeyG),
            'H' => Some(KeyCode::KeyH),
            'I' => Some(KeyCode::KeyI),
            'J' => Some(KeyCode::KeyJ),
            'K' => Some(KeyCode::KeyK),
            'L' => Some(KeyCode::KeyL),
            'M' => Some(KeyCode::KeyM),
            'N' => Some(KeyCode::KeyN),
            'O' => Some(KeyCode::KeyO),
            'P' => Some(KeyCode::KeyP),
            'Q' => Some(KeyCode::KeyQ),
            'R' => Some(KeyCode::KeyR),
            'S' => Some(KeyCode::KeyS),
            'T' => Some(KeyCode::KeyT),
            'U' => Some(KeyCode::KeyU),
            'V' => Some(KeyCode::KeyV),
            'W' => Some(KeyCode::KeyW),
            'X' => Some(KeyCode::KeyX),
            'Y' => Some(KeyCode::KeyY),
            'Z' => Some(KeyCode::KeyZ),
            '0' => Some(KeyCode::Digit0),
            '1' => Some(KeyCode::Digit1),
            '2' => Some(KeyCode::Digit2),
            '3' => Some(KeyCode::Digit3),
            '4' => Some(KeyCode::Digit4),
            '5' => Some(KeyCode::Digit5),
            '6' => Some(KeyCode::Digit6),
            '7' => Some(KeyCode::Digit7),
            '8' => Some(KeyCode::Digit8),
            '9' => Some(KeyCode::Digit9),
            '[' => Some(KeyCode::BracketLeft),
            ']' => Some(KeyCode::BracketRight),
            _ => None,
        };
    }
    match normalized {
        "Delete" => Some(KeyCode::Delete),
        "Backspace" => Some(KeyCode::Backspace),
        "Home" => Some(KeyCode::Home),
        "Enter" => Some(KeyCode::Enter),
        _ => None,
    }
}
