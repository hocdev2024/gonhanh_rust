use crate::engine::ENGINE;
use crate::settings::{InputMethod}; // Removed Settings unused
use windows::core::{PCWSTR}; // Removed PWSTR unused
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM}; // Removed HINSTANCE, LRESULT unused if truly unused
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    CheckDlgButton, IsDlgButtonChecked, BST_CHECKED, BST_UNCHECKED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateDialogIndirectParamW, DestroyWindow, DispatchMessageW, GetMessageW, IsDialogMessageW,
    MessageBoxW, PostQuitMessage, SendDlgItemMessageW, ShowWindow, TranslateMessage, 
    DLGTEMPLATE, DS_CENTER, DS_MODALFRAME, DS_SETFONT, 
    MB_ICONERROR, MB_OK, MSG, SW_SHOW, 
    WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_INITDIALOG, 
    WS_CAPTION, WS_CHILD, WS_POPUP, WS_SYSMENU, WS_VISIBLE, WS_VSCROLL,
    BS_AUTOCHECKBOX, BS_GROUPBOX, WS_MINIMIZEBOX,
    CBS_DROPDOWNLIST, CB_ADDSTRING, CB_SETCURSEL, CB_GETCURSEL
};
use windows::Win32::System::Registry::{
    RegCreateKeyExW, RegSetValueExW, RegDeleteValueW, RegCloseKey,
    HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, REG_OPTION_NON_VOLATILE, HKEY
};

// Control IDs
const IDC_GRP_INPUT: i32 = 101;
// const IDC_RAD_TELEX: i32 = 102; // Removed
// const IDC_RAD_VNI: i32 = 103;   // Removed
const IDC_CHK_ENABLED: i32 = 104;
const IDC_CHK_MODERN: i32 = 105;
// const IDC_CHK_CAPS: i32 = 106; // Unused
// The user asked to KEEP functions.
// Let's re-map IDs cleanly.

// Section 1
const IDC_CHK_W_AS_U: i32 = 107;
const IDC_CHK_BRACKET: i32 = 108;
const IDC_COMBO_METHOD: i32 = 109;

// Section 2
const IDC_BTN_SHORTCUT: i32 = 110; // Phím tắt bật/tắt
const IDC_BTN_TABLE: i32 = 111;    // Bảng gõ tắt

// Section 3
const IDC_CHK_SYSTEM: i32 = 112;
const IDC_CHK_AUTO_SWITCH: i32 = 113;
const IDC_CHK_RESTORE_ENG: i32 = 114;

// Groups
const IDC_GRP_SHORTCUT: i32 = 115;
const IDC_GRP_SYSTEM: i32 = 116;

const IDC_CHK_AUTO_CAP: i32 = 117;
const IDC_CHK_DEBUG: i32 = 118;

use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NOTIFYICONDATAW, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIF_ICON, NIF_MESSAGE, NIF_TIP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, AppendMenuW, TrackPopupMenu, MF_STRING, TPM_RIGHTBUTTON,
    SendMessageW, LoadImageW, IMAGE_ICON, LR_DEFAULTSIZE, WM_SETICON, ICON_BIG, ICON_SMALL,
    WM_LBUTTONUP, WM_RBUTTONUP, HICON, DestroyIcon,
};


// use std::sync::atomic::{AtomicBool, Ordering}; // Removed unused atomics

const WM_TRAYICON: u32 = windows::Win32::UI::WindowsAndMessaging::WM_USER + 1;
const ID_TRAY_ICON: u32 = 1;

static mut WINDOW_HANDLE: HWND = HWND(0);

// Menu IDs
const IDM_EXIT: usize = 1001;
const IDM_SHOW: usize = 1002;

pub fn notify_update() {
    unsafe {
        if WINDOW_HANDLE.0 != 0 {
            use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
            const WM_UPDATE_ICON: u32 = windows::Win32::UI::WindowsAndMessaging::WM_USER + 2;
            let _ = PostMessageW(WINDOW_HANDLE, WM_UPDATE_ICON, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn run_ui() {
    unsafe {
        let instance = GetModuleHandleW(None).unwrap();
        
        let template = create_dialog_template(&"Gõ Nhanh");
        let template_ptr = template.as_ptr() as *const DLGTEMPLATE;

        let hwnd = CreateDialogIndirectParamW(
            instance,
            template_ptr,
            HWND(0),
            Some(dialog_proc),
            LPARAM(0),
        );

        if hwnd.0 == 0 {
            MessageBoxW(HWND(0), PCWSTR(encode_wide("Failed to create dialog").as_ptr()), PCWSTR(encode_wide("Error").as_ptr()), MB_OK | MB_ICONERROR);
            return;
        }

        WINDOW_HANDLE = hwnd;

        // Show main window on start
        ShowWindow(hwnd, SW_SHOW); 

        update_tray_icon(hwnd); 

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(0), 0, 0).0 > 0 {
            if !IsDialogMessageW(hwnd, &msg).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        
        remove_tray_icon(hwnd);
    }
}

unsafe extern "system" fn dialog_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    const WM_UPDATE_ICON: u32 = windows::Win32::UI::WindowsAndMessaging::WM_USER + 2;

    match msg {
        WM_INITDIALOG => {
            init_controls(hwnd);
            
            // Set window icon (Taskbar & Titlebar)
            // Resource ID 1 is default for first icon added by winres
            let instance = GetModuleHandleW(None).unwrap();
            let icon_id = PCWSTR(1 as *const u16);
            if let Ok(h_icon) = LoadImageW(instance, icon_id, IMAGE_ICON, 0, 0, LR_DEFAULTSIZE) {
                // ICON_SMALL = 0, ICON_BIG = 1
                SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_SMALL as usize), LPARAM(h_icon.0));
                SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), LPARAM(h_icon.0));
            }

            1
        }
        WM_UPDATE_ICON => {
            update_tray_icon(hwnd);
            let settings = ENGINE.lock().get_settings();
            check_dlg_button(hwnd, IDC_CHK_ENABLED, settings.enabled);
            check_dlg_button(hwnd, IDC_CHK_W_AS_U, settings.w_as_u_at_start);
            check_dlg_button(hwnd, IDC_CHK_BRACKET, settings.bracket_as_uo);
            check_dlg_button(hwnd, IDC_CHK_MODERN, settings.modern_tone);
            check_dlg_button(hwnd, IDC_CHK_SYSTEM, settings.run_with_system);
            check_dlg_button(hwnd, IDC_CHK_AUTO_SWITCH, settings.auto_switch_mode);
            check_dlg_button(hwnd, IDC_CHK_RESTORE_ENG, settings.auto_restore_english);
            check_dlg_button(hwnd, IDC_CHK_AUTO_CAP, settings.auto_capitalize);
            check_dlg_button(hwnd, IDC_CHK_DEBUG, settings.debug_enabled);
            0
        }
        WM_TRAYICON => {
            if lparam.0 as u32 == WM_LBUTTONUP {
                 // Toggle Enabled/Disabled
                 let mut settings = ENGINE.lock().get_settings();
                 settings.enabled = !settings.enabled;
                 ENGINE.lock().update_settings(settings.clone());
                 
                 // Configure UI to match new state
                 check_dlg_button(hwnd, IDC_CHK_ENABLED, settings.enabled);
                 update_tray_icon(hwnd);
                 
            } else if lparam.0 as u32 == WM_RBUTTONUP {
                 let hmenu = CreatePopupMenu().unwrap();
                 let show_str = encode_wide("Hiện bảng điều khiển");
                 let exit_str = encode_wide("Thoát");
                 let _ = AppendMenuW(hmenu, MF_STRING, IDM_SHOW, PCWSTR(show_str.as_ptr()));
                 let _ = AppendMenuW(hmenu, MF_STRING, IDM_EXIT, PCWSTR(exit_str.as_ptr()));
                 
                 let mut pt = windows::Win32::Foundation::POINT::default();
                 let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
                 let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
                 let _ = TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, None);
                 use windows::Win32::UI::WindowsAndMessaging::DestroyMenu;
                 let _ = DestroyMenu(hmenu);
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let code = (wparam.0 >> 16) as u16;
            
            if id == IDM_EXIT as i32 {
                let _ = DestroyWindow(hwnd);
                return 0;
            }
            if id == IDM_SHOW as i32 {
                ShowWindow(hwnd, SW_SHOW);
                use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
                let _ = SetForegroundWindow(hwnd);
                return 0;
            }

             if id == IDC_CHK_ENABLED || id == IDC_CHK_W_AS_U || id == IDC_CHK_BRACKET || 
                id == IDC_CHK_MODERN || id == IDC_CHK_SYSTEM || id == IDC_CHK_AUTO_SWITCH ||
                id == IDC_CHK_RESTORE_ENG || id == IDC_CHK_AUTO_CAP || id == IDC_CHK_DEBUG ||
                (id == IDC_COMBO_METHOD && code == 1) { 
                 
                 save_settings_from_ui(hwnd);
                 if id == IDC_CHK_ENABLED {
                     update_tray_icon(hwnd);
                 }
             }
             
             if id == IDC_BTN_TABLE || id == IDC_BTN_SHORTCUT {
                 MessageBoxW(hwnd, PCWSTR(encode_wide("Tính năng đang phát triển").as_ptr()), PCWSTR(encode_wide("Thông báo").as_ptr()), MB_OK);
             }
            
            0
        }
        WM_CLOSE => {
            let _ = ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => 0,
    }
}

// Helper to load icon from memory
// Helper removed as we now load from resources directly


// Global state tracking for tray to avoid re-adding
static mut TRAY_CREATED: bool = false;

unsafe fn update_tray_icon(hwnd: HWND) {
    let enabled = ENGINE.lock().get_settings().enabled;
    
    // Resource IDs defined in icons.rc
    // 101 ICON "icon/V.ico"
    // 102 ICON "icon/E.ico"
    let icon_id = if enabled { 101 } else { 102 };

    let instance = GetModuleHandleW(None).unwrap();
    let resource_name = PCWSTR(icon_id as *const u16);
    
    // Load from resources (shared icon, no need to destroy)
    let hicon = LoadImageW(
        instance, 
        resource_name, 
        IMAGE_ICON, 
        0, 
        0, 
        LR_DEFAULTSIZE 
    ).map(|h| HICON(h.0)).ok();

    if let Some(hicon) = hicon {
        let title = if enabled { "Gõ Nhanh: Tiếng Việt" } else { "Gõ Nhanh: Tiếng Anh" };
        
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = ID_TRAY_ICON;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = hicon; 
        
        let title_wide = encode_wide(title);
        for (i, c) in title_wide.iter().enumerate().take(127) {
            nid.szTip[i] = *c;
        }

        if !TRAY_CREATED {
            Shell_NotifyIconW(NIM_ADD, &nid);
            TRAY_CREATED = true;
        } else {
            Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
        // No DestroyIcon needed for shared icons loaded via LoadImageW without LR_LOADFROMFILE
        // But LoadImageW without LR_SHARED returns a copy?
        // documentation says: "If you want to create an icon or cursor that you can use more than once, use the LoadImage function without the LR_SHARED flag." 
        // Wait, regular LoadImage is creating a new icon if not shared.
        // Let's assume we should destroy it if we are just setting it to tray.
        // The tray copies the icon handle? Or takes ownership? 
        // Shell_NotifyIcon documentation: "The system makes a copy of the icon."
        // So we own hicon and should destroy it.
        let _ = DestroyIcon(hicon);
    }
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = ID_TRAY_ICON;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe fn init_controls(hwnd: HWND) {
    let settings = ENGINE.lock().get_settings();
    
    check_dlg_button(hwnd, IDC_CHK_ENABLED, settings.enabled);
    check_dlg_button(hwnd, IDC_CHK_MODERN, settings.modern_tone);
    
    check_dlg_button(hwnd, IDC_CHK_W_AS_U, settings.w_as_u_at_start);
    check_dlg_button(hwnd, IDC_CHK_BRACKET, settings.bracket_as_uo);
    check_dlg_button(hwnd, IDC_CHK_SYSTEM, settings.run_with_system);
    check_dlg_button(hwnd, IDC_CHK_AUTO_SWITCH, settings.auto_switch_mode);
    check_dlg_button(hwnd, IDC_CHK_RESTORE_ENG, settings.auto_restore_english);
    check_dlg_button(hwnd, IDC_CHK_AUTO_CAP, settings.auto_capitalize);

    // Combo initialization
    let telex = encode_wide("Telex");
    let vni = encode_wide("VNI");
    
    SendDlgItemMessageW(hwnd, IDC_COMBO_METHOD, CB_ADDSTRING, WPARAM(0), LPARAM(telex.as_ptr() as isize));
    SendDlgItemMessageW(hwnd, IDC_COMBO_METHOD, CB_ADDSTRING, WPARAM(0), LPARAM(vni.as_ptr() as isize));
    
    let sel = match settings.method {
        InputMethod::Telex => 0,
        InputMethod::Vni => 1,
    };
    SendDlgItemMessageW(hwnd, IDC_COMBO_METHOD, CB_SETCURSEL, WPARAM(sel), LPARAM(0));
}

unsafe fn save_settings_from_ui(hwnd: HWND) {
    let enabled = is_dlg_button_checked(hwnd, IDC_CHK_ENABLED);
    let modern = is_dlg_button_checked(hwnd, IDC_CHK_MODERN);
    // caps -> ignored
    let w_as_u = is_dlg_button_checked(hwnd, IDC_CHK_W_AS_U);
    let bracket = is_dlg_button_checked(hwnd, IDC_CHK_BRACKET);
    let system = is_dlg_button_checked(hwnd, IDC_CHK_SYSTEM);
    let auto_switch = is_dlg_button_checked(hwnd, IDC_CHK_AUTO_SWITCH);
    let restore_eng = is_dlg_button_checked(hwnd, IDC_CHK_RESTORE_ENG);
    let auto_cap = is_dlg_button_checked(hwnd, IDC_CHK_AUTO_CAP);
    
    let sel = SendDlgItemMessageW(hwnd, IDC_COMBO_METHOD, CB_GETCURSEL, WPARAM(0), LPARAM(0));
    let method = if sel.0 == 1 { InputMethod::Vni } else { InputMethod::Telex };
    
    let mut current = ENGINE.lock().get_settings();
    current.enabled = enabled;
    current.method = method;
    current.modern_tone = modern;
    current.w_as_u_at_start = w_as_u;
    current.bracket_as_uo = bracket;
    current.run_with_system = system;
    current.auto_switch_mode = auto_switch;
    current.auto_restore_english = restore_eng;
    current.auto_capitalize = auto_cap;
    
    ENGINE.lock().update_settings(current);

    // Apply Startup Setting
    unsafe {
        manage_startup(system);
    }
}

unsafe fn manage_startup(enable: bool) {
    let app_name = encode_wide("GoNhanh");
    let key_path = encode_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let mut hkey = HKEY(0);
    
    // Create/Open Key
    let res = RegCreateKeyExW(
        HKEY_CURRENT_USER,
        PCWSTR(key_path.as_ptr()),
        0,
        None,
        REG_OPTION_NON_VOLATILE,
        KEY_SET_VALUE,
        None,
        &mut hkey,
        None
    );

    if res.is_ok() {
        if enable {
             // Get current exe path
             if let Ok(path) = std::env::current_exe() {
                 let path_str = path.to_string_lossy();
                 let path_wide = encode_wide(&path_str);
                 
                 let _ = RegSetValueExW(
                     hkey,
                     PCWSTR(app_name.as_ptr()),
                     0,
                     REG_SZ,
                     Some(std::slice::from_raw_parts(path_wide.as_ptr() as *const u8, path_wide.len() * 2))
                 );
             }
        } else {
            let _ = RegDeleteValueW(hkey, PCWSTR(app_name.as_ptr()));
        }
        let _ = RegCloseKey(hkey);
    }
}

unsafe fn check_dlg_button(hwnd: HWND, id: i32, checked: bool) {
    let state = if checked { BST_CHECKED } else { BST_UNCHECKED };
    let _ = CheckDlgButton(hwnd, id, state);
}

unsafe fn is_dlg_button_checked(hwnd: HWND, id: i32) -> bool {
    // IsDlgButtonChecked returns u32 in older crates but DLG_BUTTON_CHECK_STATE in newer ones
    // BST_CHECKED is DLG_BUTTON_CHECK_STATE.
    // The compiler said u32 == DLG_BUTTON_CHECK_STATE failed means IsDlgButtonChecked returns u32?
    // Wait, the error is: "can't compare `u32` with `DLG_BUTTON_CHECK_STATE`"
    // -> IsDlgButtonChecked must be returning u32 (in windows 0.52 function signature might be u32)
    // -> BST_CHECKED is DLG_BUTTON_CHECK_STATE (enum/struct).
    // So we need to cast or access .0
    IsDlgButtonChecked(hwnd, id) == BST_CHECKED.0
}

fn create_dialog_template(title: &str) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(2048);

    let align = |buf: &mut Vec<u8>| {
        while buf.len() % 4 != 0 {
            buf.push(0);
        }
    };

    // Style: WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | DS_MODALFRAME | DS_CENTER | DS_SETFONT
    let style = WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 |
                DS_MODALFRAME as u32 | DS_CENTER as u32 | DS_SETFONT as u32;
                
    let ext_style = 0u32;
    let num_items: u16 = 17; // Increased count for Debug checkbox
    
    buffer.extend_from_slice(&style.to_le_bytes());
    buffer.extend_from_slice(&ext_style.to_le_bytes());
    buffer.extend_from_slice(&num_items.to_le_bytes());
    
    // x, y, w, h (Dialog Units)
    buffer.extend_from_slice(&100i16.to_le_bytes());
    buffer.extend_from_slice(&100i16.to_le_bytes());
    buffer.extend_from_slice(&200i16.to_le_bytes());
    buffer.extend_from_slice(&300i16.to_le_bytes()); // Increased height
    
    // Menu, Class
    buffer.extend_from_slice(&0u16.to_le_bytes());
    buffer.extend_from_slice(&0u16.to_le_bytes());
    
    // Title
    for c in title.encode_utf16() {
        buffer.extend_from_slice(&c.to_le_bytes());
    }
    buffer.extend_from_slice(&0u16.to_le_bytes());
    
    // Font (9pt)
    buffer.extend_from_slice(&9u16.to_le_bytes());
    for c in "Segoe UI".encode_utf16() {
        buffer.extend_from_slice(&c.to_le_bytes());
    }
    buffer.extend_from_slice(&0u16.to_le_bytes());

    // --- Items ---
    
    let mut add_item = |x: i16, y: i16, w: i16, h: i16, id: i16, style_flags: u32, class_id: u16, text: &str| {
        align(&mut buffer);
        
        // WS_CHILD | WS_VISIBLE is base
        let style = (WS_CHILD | WS_VISIBLE).0 | style_flags;
        let ext_style = 0u32;
        
        buffer.extend_from_slice(&style.to_le_bytes());
        buffer.extend_from_slice(&ext_style.to_le_bytes());
        buffer.extend_from_slice(&x.to_le_bytes());
        buffer.extend_from_slice(&y.to_le_bytes());
        buffer.extend_from_slice(&w.to_le_bytes());
        buffer.extend_from_slice(&h.to_le_bytes());
        buffer.extend_from_slice(&id.to_le_bytes());
        
        // Class means: 0xFFFF + Atom
        // Button: 0x0080
        // Edit: 0x0081
        // Static: 0x0082
        // ComboBox: 0x0085
        buffer.extend_from_slice(&0xFFFFu16.to_le_bytes());
        buffer.extend_from_slice(&class_id.to_le_bytes());
        
        // Text
        for c in text.encode_utf16() {
            buffer.extend_from_slice(&c.to_le_bytes());
        }
        buffer.extend_from_slice(&0u16.to_le_bytes());
        
        buffer.extend_from_slice(&0u16.to_le_bytes());
    };

    // SECTION 1: Input Attributes
    // GroupBox - Ends at 120
    add_item(5, 5, 190, 115, IDC_GRP_INPUT as i16, BS_GROUPBOX as u32, 0x0080, "Bộ gõ");
    
    // Enabled
    add_item(15, 20, 170, 14, IDC_CHK_ENABLED as i16, BS_AUTOCHECKBOX as u32, 0x0080, "Bộ gõ tiếng Việt");
    
    // Method (Label + Combo)
    add_item(15, 38, 40, 12, -1, 0, 0x0082, "Kiểu gõ");
    add_item(60, 36, 120, 100, IDC_COMBO_METHOD as i16, CBS_DROPDOWNLIST as u32 | WS_VSCROLL.0, 0x0085, "");
    
    // W as U
    add_item(15, 55, 170, 14, IDC_CHK_W_AS_U as i16, BS_AUTOCHECKBOX as u32, 0x0080, "Gõ W thành Ư ở đầu từ");
    
    // Brackets
    add_item(15, 70, 170, 14, IDC_CHK_BRACKET as i16, BS_AUTOCHECKBOX as u32, 0x0080, "Gõ ] thành Ư, [ thành Ơ");
    
    // Modern Tone
    add_item(15, 85, 170, 14, IDC_CHK_MODERN as i16, BS_AUTOCHECKBOX as u32, 0x0080, "Dấu thanh hiện đại (òa, úy)");

    // Auto Capitalize
    add_item(15, 100, 170, 14, IDC_CHK_AUTO_CAP as i16, BS_AUTOCHECKBOX as u32, 0x0080, "Tự động viết hoa (sau dấu .)");

    // SECTION 2: Shortcuts
    // Moved down to start at 125 (ends at 185) to avoid overlap with Section 1 (ends at 120)
    add_item(5, 125, 190, 60, IDC_GRP_SHORTCUT as i16, BS_GROUPBOX as u32, 0x0080, "Phím tắt");
    
    // Toggle Trigger
    add_item(15, 140, 80, 12, -1, 0, 0x0082, "Phím tắt bật/tắt");
    add_item(100, 138, 80, 14, IDC_BTN_SHORTCUT as i16, 0, 0x0080, "Ctrl + Shift"); 
    
    // Shortcut Table
    add_item(15, 160, 170, 14, IDC_BTN_TABLE as i16, 0, 0x0080, "Bảng gõ tắt > (3/6)"); 

    // SECTION 3: System
    // Moved down to start at 190 (ends at 290) to avoid overlap with Section 2 (ends at 185)
    // Height increased to 100 to fit Debug checkbox
    add_item(5, 190, 190, 100, IDC_GRP_SYSTEM as i16, BS_GROUPBOX as u32, 0x0080, "Hệ thống");
    
    // Run with system
    add_item(15, 205, 170, 14, IDC_CHK_SYSTEM as i16, BS_AUTOCHECKBOX as u32, 0x0080, "Khởi động cùng hệ thống");
    
    // Auto switch
    add_item(15, 220, 170, 14, IDC_CHK_AUTO_SWITCH as i16, BS_AUTOCHECKBOX as u32, 0x0080, "Tự chuyển chế độ theo ứng dụng");
    
    // Restore English
    add_item(15, 235, 170, 14, IDC_CHK_RESTORE_ENG as i16, BS_AUTOCHECKBOX as u32, 0x0080, "Tự khôi phục từ tiếng Anh");

    // Debug
    add_item(15, 250, 170, 14, IDC_CHK_DEBUG as i16, BS_AUTOCHECKBOX as u32, 0x0080, "Bật Log Debug (Log vào file)");
    
    buffer
}

fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
