//! Encode crossterm key events into bytes for the exec session stdin (xterm).
use crossterm::event::{KeyCode, KeyModifiers};

const ESC: u8 = 0x1b;

/// Modifier parameter used by CSI sequences: 1 + Shift(1) + Alt(2) + Ctrl(4).
fn modifier_param(modifiers: KeyModifiers) -> u8 {
    let mut value = 1;
    if modifiers.contains(KeyModifiers::SHIFT) {
        value += 1;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        value += 2;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        value += 4;
    }
    value
}

fn csi(final_byte: u8, modifiers: KeyModifiers) -> Vec<u8> {
    if modifiers.is_empty() {
        vec![ESC, b'[', final_byte]
    } else {
        format!(
            "{}[1;{}{}",
            ESC as char,
            modifier_param(modifiers),
            final_byte as char
        )
        .into_bytes()
    }
}

fn csi_tilde(number: u8, modifiers: KeyModifiers) -> Vec<u8> {
    if modifiers.is_empty() {
        format!("{}[{}~", ESC as char, number).into_bytes()
    } else {
        format!("{}[{};{}~", ESC as char, number, modifier_param(modifiers)).into_bytes()
    }
}

fn control_byte(c: char) -> Option<u8> {
    match c {
        'a'..='z' => Some((c as u8) & 0x1f),
        'A'..='Z' => Some((c as u8) & 0x1f),
        '@' => Some(0x00),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' => Some(0x1e),
        '_' => Some(0x1f),
        _ => None,
    }
}

/// Translate a key event into terminal input bytes; `None` = not translatable.
pub fn encode_key(code: KeyCode, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    match code {
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                control_byte(c).map(|byte| vec![byte])
            } else if modifiers.contains(KeyModifiers::ALT) {
                let mut bytes = vec![ESC];
                let mut buffer = [0u8; 4];
                bytes.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
                Some(bytes)
            } else {
                let mut buffer = [0u8; 4];
                Some(c.encode_utf8(&mut buffer).as_bytes().to_vec())
            }
        }
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::BackTab => Some(vec![ESC, b'[', b'Z']),
        KeyCode::Esc => Some(vec![ESC]),
        KeyCode::Up => Some(csi(b'A', modifiers)),
        KeyCode::Down => Some(csi(b'B', modifiers)),
        KeyCode::Right => Some(csi(b'C', modifiers)),
        KeyCode::Left => Some(csi(b'D', modifiers)),
        KeyCode::Home => Some(csi(b'H', modifiers)),
        KeyCode::End => Some(csi(b'F', modifiers)),
        KeyCode::Insert => Some(csi_tilde(2, modifiers)),
        KeyCode::Delete => Some(csi_tilde(3, modifiers)),
        KeyCode::PageUp => Some(csi_tilde(5, modifiers)),
        KeyCode::PageDown => Some(csi_tilde(6, modifiers)),
        KeyCode::F(1) => Some(vec![ESC, b'O', b'P']),
        KeyCode::F(2) => Some(vec![ESC, b'O', b'Q']),
        KeyCode::F(3) => Some(vec![ESC, b'O', b'R']),
        KeyCode::F(4) => Some(vec![ESC, b'O', b'S']),
        KeyCode::F(n @ 5..=12) => {
            let number = match n {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                _ => 24,
            };
            Some(format!("{}[{}~", ESC as char, number).into_bytes())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::encode_key;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn plain_characters_encode_as_utf8() {
        assert_eq!(
            encode_key(KeyCode::Char('a'), KeyModifiers::NONE).unwrap(),
            b"a"
        );
        assert_eq!(
            encode_key(KeyCode::Char('ñ'), KeyModifiers::NONE).unwrap(),
            "ñ".as_bytes()
        );
        assert_eq!(
            encode_key(KeyCode::Char('🦀'), KeyModifiers::NONE).unwrap(),
            "🦀".as_bytes()
        );
    }

    #[test]
    fn ctrl_letters_encode_as_control_bytes() {
        assert_eq!(
            encode_key(KeyCode::Char('c'), KeyModifiers::CONTROL).unwrap(),
            vec![0x03]
        );
        assert_eq!(
            encode_key(KeyCode::Char('d'), KeyModifiers::CONTROL).unwrap(),
            vec![0x04]
        );
        assert_eq!(
            encode_key(KeyCode::Char(']'), KeyModifiers::CONTROL).unwrap(),
            vec![0x1d]
        );
    }

    #[test]
    fn editing_and_navigation_keys_match_xterm() {
        let none = KeyModifiers::NONE;
        assert_eq!(encode_key(KeyCode::Enter, none).unwrap(), b"\r");
        assert_eq!(encode_key(KeyCode::Backspace, none).unwrap(), vec![0x7f]);
        assert_eq!(encode_key(KeyCode::Tab, none).unwrap(), b"\t");
        assert_eq!(encode_key(KeyCode::Esc, none).unwrap(), vec![0x1b]);
        assert_eq!(encode_key(KeyCode::Up, none).unwrap(), b"\x1b[A");
        assert_eq!(encode_key(KeyCode::Down, none).unwrap(), b"\x1b[B");
        assert_eq!(encode_key(KeyCode::Right, none).unwrap(), b"\x1b[C");
        assert_eq!(encode_key(KeyCode::Left, none).unwrap(), b"\x1b[D");
        assert_eq!(encode_key(KeyCode::Home, none).unwrap(), b"\x1b[H");
        assert_eq!(encode_key(KeyCode::End, none).unwrap(), b"\x1b[F");
        assert_eq!(encode_key(KeyCode::Delete, none).unwrap(), b"\x1b[3~");
        assert_eq!(encode_key(KeyCode::PageUp, none).unwrap(), b"\x1b[5~");
        assert_eq!(encode_key(KeyCode::PageDown, none).unwrap(), b"\x1b[6~");
        assert_eq!(
            encode_key(KeyCode::BackTab, KeyModifiers::SHIFT).unwrap(),
            b"\x1b[Z"
        );
    }

    #[test]
    fn modified_arrows_use_csi_modifier_parameters() {
        assert_eq!(
            encode_key(KeyCode::Up, KeyModifiers::CONTROL).unwrap(),
            b"\x1b[1;5A"
        );
        assert_eq!(
            encode_key(KeyCode::Left, KeyModifiers::SHIFT).unwrap(),
            b"\x1b[1;2D"
        );
    }

    #[test]
    fn alt_prefixes_escape() {
        assert_eq!(
            encode_key(KeyCode::Char('b'), KeyModifiers::ALT).unwrap(),
            b"\x1bb"
        );
    }

    #[test]
    fn function_keys_have_xterm_sequences() {
        let none = KeyModifiers::NONE;
        assert_eq!(encode_key(KeyCode::F(1), none).unwrap(), b"\x1bOP");
        assert_eq!(encode_key(KeyCode::F(4), none).unwrap(), b"\x1bOS");
        assert_eq!(encode_key(KeyCode::F(5), none).unwrap(), b"\x1b[15~");
        assert_eq!(encode_key(KeyCode::F(12), none).unwrap(), b"\x1b[24~");
    }

    #[test]
    fn untranslatable_keys_are_ignored() {
        assert!(encode_key(KeyCode::CapsLock, KeyModifiers::NONE).is_none());
        assert!(encode_key(KeyCode::Null, KeyModifiers::NONE).is_none());
    }
}
