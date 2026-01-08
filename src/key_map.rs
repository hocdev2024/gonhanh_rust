use gonhanh_core::data::keys;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_0, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9,
    VK_A, VK_B, VK_C, VK_D, VK_E, VK_F, VK_G, VK_H, VK_I, VK_J, VK_K, VK_L, VK_M,
    VK_N, VK_O, VK_P, VK_Q, VK_R, VK_S, VK_T, VK_U, VK_V, VK_W, VK_X, VK_Y, VK_Z,
    VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7, 
    VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS,
    VK_BACK, VK_ESCAPE, VK_RETURN, VK_SPACE, VK_TAB,
    VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN,
    VIRTUAL_KEY,
};

pub fn map_vk_to_core(vk: VIRTUAL_KEY) -> Option<u16> {
    // Map Windows VK to Core (macOS) keycodes
    match vk {
        VK_A => Some(keys::A),
        VK_S => Some(keys::S),
        VK_D => Some(keys::D),
        VK_F => Some(keys::F),
        VK_H => Some(keys::H),
        VK_G => Some(keys::G),
        VK_Z => Some(keys::Z),
        VK_X => Some(keys::X),
        VK_C => Some(keys::C),
        VK_V => Some(keys::V),
        VK_B => Some(keys::B),
        VK_Q => Some(keys::Q),
        VK_W => Some(keys::W),
        VK_E => Some(keys::E),
        VK_R => Some(keys::R),
        VK_Y => Some(keys::Y),
        VK_T => Some(keys::T),
        VK_O => Some(keys::O),
        VK_U => Some(keys::U),
        VK_I => Some(keys::I),
        VK_P => Some(keys::P),
        VK_L => Some(keys::L),
        VK_J => Some(keys::J),
        VK_K => Some(keys::K),
        VK_N => Some(keys::N),
        VK_M => Some(keys::M),

        // VK_0 and VK_OEM_1 handled below

        VK_2 => Some(keys::N2),
        VK_3 => Some(keys::N3),
        VK_4 => Some(keys::N4),
        VK_5 => Some(keys::N5),
        VK_6 => Some(keys::N6),
        VK_7 => Some(keys::N7),
        VK_8 => Some(keys::N8),
        VK_9 => Some(keys::N9),
        VK_0 => Some(keys::N0),

        VK_SPACE => Some(keys::SPACE),
        VK_RETURN => Some(keys::RETURN), // or ENTER
        VK_BACK => Some(keys::DELETE), // MacOS 'Delete' is Backspace
        VK_ESCAPE => Some(keys::ESC),
        VK_TAB => Some(keys::TAB),
        
        VK_LEFT => Some(keys::LEFT),
        VK_RIGHT => Some(keys::RIGHT),
        VK_UP => Some(keys::UP),
        VK_DOWN => Some(keys::DOWN),

        // Punctuation
        VK_OEM_PERIOD => Some(keys::DOT),
        VK_OEM_COMMA => Some(keys::COMMA),
        VK_OEM_1 => Some(keys::SEMICOLON), // ;:
        VK_OEM_7 => Some(keys::QUOTE),     // '"
        VK_OEM_4 => Some(keys::LBRACKET),  // [{
        VK_OEM_6 => Some(keys::RBRACKET),  // ]}
        VK_OEM_5 => Some(keys::BACKSLASH), // \|
        VK_OEM_MINUS => Some(keys::MINUS), // -_
        VK_OEM_PLUS => Some(keys::EQUAL),  // =+
        VK_OEM_3 => Some(keys::BACKQUOTE), // `~
        VK_OEM_2 => Some(keys::SLASH),     // /?
        
        _ => None,
    }
}
