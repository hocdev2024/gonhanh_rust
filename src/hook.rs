use crate::engine::{ImeResult, ENGINE};
use crate::key_map::map_vk_to_core;
use gonhanh_core::engine::Action;

use std::mem::size_of;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{CloseHandle, HINSTANCE, LPARAM, LRESULT, WPARAM, HWND};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_BACK, VK_CAPITAL, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, HHOOK, KBDLLHOOKSTRUCT,
    MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    GetForegroundWindow, GetWindowThreadProcessId,
    GetGUIThreadInfo, GUITHREADINFO,
};
use windows::Win32::System::DataExchange::{OpenClipboard, EmptyClipboard, SetClipboardData, CloseClipboard};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::Foundation::{HANDLE};


static mut HOOK_HANDLE: HHOOK = HHOOK(0);
static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);


fn set_clipboard_text(text: &str) {
    unsafe {
        if OpenClipboard(HWND(0)).is_ok() {
            let _ = EmptyClipboard();
            
            // Convert to UTF-16
            let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
            let bytes = wide.len() * 2;
            
            if let Ok(h_mem) = GlobalAlloc(GMEM_MOVEABLE, bytes) {
                let p_mem = GlobalLock(h_mem);
                if !p_mem.is_null() {
                    std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, p_mem as *mut u8, bytes);
                    let _ = GlobalUnlock(h_mem);
                    
                    // CF_UNICODETEXT = 13
                    let _ = SetClipboardData(13, HANDLE(h_mem.0 as isize));
                }
            }
            let _ = CloseClipboard();
        }
    }
}

fn log_to_file(msg: &str) {
    if !ENGINE.lock().get_settings().debug_enabled {
        return;
    }
    use std::fs::OpenOptions;
    use std::io::Write;
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("debug.log") {
        let _ = writeln!(file, "{}", msg);
    }
}

pub fn install() {
    if HOOK_INSTALLED.load(Ordering::SeqCst) {
        return;
    }

    thread::spawn(|| {
        // info!("Starting hook thread");
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
                    // info!("Hook installed successfully");

                    let mut msg = MSG::default();
                    // Message loop to keep hook alive
                    while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                        // Just pump messages
                    }
                }
                Err(_e) => {
                    // error!("Failed to install hook: {:?}", e);
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
    
    // Debug Log
    if is_keydown {
        log_to_file(&format!("Key: {:?}, Scan: {}, Flags: {:X}", vk, (*pkb).scanCode, flags.0));
    }

    // Check modifiers state using GetKeyState
    let shift_down = (GetKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
    let ctrl_down = (GetKeyState(windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
    let caps_on = (GetKeyState(VK_CAPITAL.0 as i32) as u16 & 0x0001) != 0;

    let is_ctrl = vk == windows::Win32::UI::Input::KeyboardAndMouse::VK_LCONTROL || vk == windows::Win32::UI::Input::KeyboardAndMouse::VK_RCONTROL;
    let is_shift = vk == VK_SHIFT || vk == windows::Win32::UI::Input::KeyboardAndMouse::VK_LSHIFT || vk == windows::Win32::UI::Input::KeyboardAndMouse::VK_RSHIFT;

    if is_keydown {
        if !is_ctrl && !is_shift {
            OTHER_KEY_PRESSED = true;
        }
    } else {
        if is_ctrl || is_shift {
            if is_ctrl && shift_down && !OTHER_KEY_PRESSED {
                 ENGINE.lock().toggle_enabled();
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
    // Check if we are targeting Warp
    let is_warp = get_foreground_process_name()
        .map(|n| n.to_lowercase() == "warp.exe")
        .unwrap_or(false);

    let hwnd_foreground = if is_warp { GetForegroundWindow() } else { HWND(0) };
    
    // Find focused window within the foreground thread
    let mut hwnd_target = hwnd_foreground;
    if is_warp && hwnd_foreground.0 != 0 {
        let mut pid = 0;
        let thread_id = GetWindowThreadProcessId(hwnd_foreground, Some(&mut pid));
        if thread_id != 0 {
             let mut gui_info = GUITHREADINFO::default();
             gui_info.cbSize = size_of::<GUITHREADINFO>() as u32;
             if GetGUIThreadInfo(thread_id, &mut gui_info).is_ok() {
                 if gui_info.hwndFocus.0 != 0 {
                     hwnd_target = gui_info.hwndFocus;
                 }
             }
        }
    }

    let use_clipboard = is_warp && hwnd_target.0 != 0;

    if is_warp {
        log_to_file(&format!("Target Warp. Clipboard Injection: {}", use_clipboard));
    }
    
    if use_clipboard {
        // Warp Specific Logic: Clipboard Injection
        
        // 1. Send Backspaces
        let mut inputs: Vec<INPUT> = Vec::new();
        for _ in 0..res.backspace {
            inputs.push(create_key_input(VK_BACK, false));
            inputs.push(create_key_input(VK_BACK, true));
        }
        if !inputs.is_empty() {
             SendInput(&inputs, size_of::<INPUT>() as i32);
        }

        // 2. Insert text via Clipboard
        let mut text = String::new();
        for i in 0..res.count as usize {
            if let Some(ch) = char::from_u32(res.chars[i]) {
                text.push(ch);
            }
        }
        
        if !text.is_empty() {
            set_clipboard_text(&text);
            
            // Send Ctrl + V
            let mut paste_inputs: Vec<INPUT> = Vec::new();
            // Ctrl Down
            paste_inputs.push(create_key_input(windows::Win32::UI::Input::KeyboardAndMouse::VK_LCONTROL, false));
            // V Down
            paste_inputs.push(create_key_input(VIRTUAL_KEY(0x56), false)); // V
            // V Up
            paste_inputs.push(create_key_input(VIRTUAL_KEY(0x56), true));
            // Ctrl Up
            paste_inputs.push(create_key_input(windows::Win32::UI::Input::KeyboardAndMouse::VK_LCONTROL, true));
            
            SendInput(&paste_inputs, size_of::<INPUT>() as i32);
        }
        return;
    } 

    // Standard Logic (Non-Warp)
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


unsafe fn get_foreground_process_name() -> Option<String> {
    let hwnd = GetForegroundWindow();
    if hwnd.0 == 0 {
        return None;
    }

    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return None;
    }

    let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);
    match process_handle {
        Ok(handle) => {
            let mut buffer = [0u16; 260];
            let len = GetModuleBaseNameW(handle, None, &mut buffer);
            let _ = CloseHandle(handle);

            if len > 0 {
                let name = String::from_utf16_lossy(&buffer[..len as usize]);
                return Some(name);
            }
            None
        }
        Err(_) => None,
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
