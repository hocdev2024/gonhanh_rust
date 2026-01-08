use crate::engine::{ImeResult, ENGINE};
use crate::key_map::map_vk_to_core;
use gonhanh_core::engine::Action;
use log::{error, info};
use std::mem::size_of;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_BACK, VK_CAPITAL, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, HHOOK, KBDLLHOOKSTRUCT,
    MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

static mut HOOK_HANDLE: HHOOK = HHOOK(0);
static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn install() {
    if HOOK_INSTALLED.load(Ordering::SeqCst) {
        return;
    }

    thread::spawn(|| {
        info!("Starting hook thread");
        unsafe {
            let module = GetModuleHandleW(None).unwrap();
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                HINSTANCE(module.0),
                0,
            );

            match hook {
                Ok(h) => {
                    HOOK_HANDLE = h;
                    HOOK_INSTALLED.store(true, Ordering::SeqCst);
                    info!("Hook installed successfully");

                    let mut msg = MSG::default();
                    // Message loop to keep hook alive
                    while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                        // Just pump messages
                    }
                }
                Err(e) => {
                    error!("Failed to install hook: {:?}", e);
                }
            }
        }
    });
}

// Track if any non-modifier key was pressed while modifiers were held
static mut OTHER_KEY_PRESSED: bool = false;

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(HOOK_HANDLE, code, wparam, lparam);
    }

    let pkb = lparam.0 as *const KBDLLHOOKSTRUCT;
    let flags = (*pkb).flags;

    // Ignore injected keys
    if (flags.0 & 0x10) != 0 {
         return CallNextHookEx(HOOK_HANDLE, code, wparam, lparam);
    }

    let vk = VIRTUAL_KEY((*pkb).vkCode as u16);
    let is_keydown = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
    
    // Check modifiers state using GetKeyState
    let shift_down = (GetKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
    let ctrl_down = (GetKeyState(windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
    let caps_on = (GetKeyState(VK_CAPITAL.0 as i32) as u16 & 0x0001) != 0;

    // Logic for Ctrl + Shift toggle
    // We want to toggle when Ctrl+Shift are pressed and then released, WITHOUT other keys.
    // Simpler approach often used: Trigger on KeyDown if both are down? No, that conflicts.
    // Standard Windows behavior: Toggle on KeyUp of the modifier, if ONLY modifiers were pressed.
    
    let is_ctrl = vk == windows::Win32::UI::Input::KeyboardAndMouse::VK_LCONTROL || vk == windows::Win32::UI::Input::KeyboardAndMouse::VK_RCONTROL;
    let is_shift = vk == VK_SHIFT || vk == windows::Win32::UI::Input::KeyboardAndMouse::VK_LSHIFT || vk == windows::Win32::UI::Input::KeyboardAndMouse::VK_RSHIFT;

    if is_keydown {
        if !is_ctrl && !is_shift {
            OTHER_KEY_PRESSED = true;
        } else {
            // If only modifiers are down so far, reset flag if this is the start of a sequence?
            // Actually, just track: if current key is mod, don't set flag.
            // But we need to reset flag when ALL mods are up. 
            // Simplified: If Ctrl and Shift are BOTH down, we prep for toggle.
        }
    } else {
        // KeyUp
        // If it's a modifier key up, and we had Ctrl+Shift, and no other keys...
        if is_ctrl || is_shift {
            // Check if we just released a modifier, and the OTHER modifier is still down (or just released?)
            // Windows IME switching is tricky.
            // Let's implement a simpler version: 
            // if (Ctrl is down AND Shift is pressed) OR (Shift is down AND Ctrl is pressed).
            // BUT we must avoid masking shortcuts like Ctrl+Shift+S.
            // So we toggle ONLY on KeyUp of Ctrl or Shift, if:
            // 1. Both Ctrl and Shift WERE down.
            // 2. No other key was pressed in between.
            
            // For now, let's try a robust heuristic:
            // If KeyUp(Ctrl) and Shift is down, and !OTHER_KEY_PRESSED -> Toggle
            // If KeyUp(Shift) and Ctrl is down, and !OTHER_KEY_PRESSED -> Toggle
            
            // We need to track OTHER_KEY_PRESSED reliably.
            // Reset OTHER_KEY_PRESSED when neither Ctrl nor Shift is down.
            
            if is_ctrl && shift_down && !OTHER_KEY_PRESSED {
                 ENGINE.lock().toggle_enabled();
                 // Notify UI to update icon (via message or callback? Hook is in thread.)
                 // We can use PostMessage to the main window if we knew its HWND. 
                 // For now, let's let the UI poll or use a shared event, OR just find the window.
                 crate::ui::notify_update();
            } else if is_shift && ctrl_down && !OTHER_KEY_PRESSED {
                 ENGINE.lock().toggle_enabled();
                 crate::ui::notify_update();
            }
        }
    }
    
    // Reset "Other key" flag if no modifiers are down
    if !ctrl_down && !shift_down {
        OTHER_KEY_PRESSED = false;
    }


    // Map key and process
    if let Some(core_key) = map_vk_to_core(vk) {
        // Only process if enabled and KeyDown
        if is_keydown {
             let alt_down = (flags.0 & 0x20) != 0;
             // If Ctrl or Alt is held, skip engine processing to avoid shortcut conflicts (like Ctrl+A)
             if ctrl_down || alt_down {
                 return CallNextHookEx(HOOK_HANDLE, code, wparam, lparam);
             }

             let mut engine = ENGINE.lock();
             // Important: Pass modifiers to engine if needed, or handle locally.
             // If Ctrl is down, we generally bypass processing in engine (it returns None), 
             // but we still call it to clear state.
             
             let result = engine.process_key(core_key, shift_down, caps_on);
             
             match result.action {
                 x if x == Action::None as u8 => {}
                 x if x == Action::Send as u8 || x == Action::Restore as u8 => {
                      drop(engine);
                      send_replacement(&result);
                      return LRESULT(1);
                 }
                 _ => {}
             }
        }
    }

    CallNextHookEx(HOOK_HANDLE, code, wparam, lparam)
}

unsafe fn send_replacement(res: &ImeResult) {
    let mut inputs: Vec<INPUT> = Vec::new();

    // 1. Backspace
    for _ in 0..res.backspace {
        inputs.push(create_key_input(VK_BACK, false)); // Down
        inputs.push(create_key_input(VK_BACK, true));  // Up
    }

    // 2. Text
    for i in 0..res.count as usize {
        let c = res.chars[i];
        if c != 0 {
             inputs.push(create_unicode_input(c, false));
             inputs.push(create_unicode_input(c, true));
        }
    }

    if !inputs.is_empty() {
        SendInput(&inputs, size_of::<INPUT>() as i32);
    }
}

fn create_key_input(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let flags = if up { KEYEVENTF_KEYUP } else { windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0) };
    
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn create_unicode_input(c: u32, up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if up {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: c as u16, // Assuming BMP
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
