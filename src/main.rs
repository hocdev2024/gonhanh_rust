#![windows_subsystem = "windows"]

mod engine;
mod hook;
mod key_map;
mod settings;
mod ui;



fn main() {
    unsafe {
        use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        // Single Instance Check
        use windows::Win32::System::Threading::CreateMutexW;
        use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
        use windows::core::PCWSTR;

        // Local helper to avoid accessing private functions
        fn encode_wide_local(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }
        
        let mutex_name_wide = encode_wide_local("Global\\GoNhanhAppMutex");
        // CreateMutexW returns Result<HANDLE>. We unwrap or ignore because we mainly care about GetLastError if it succeeds.
        // Even if it returns Ok, the mutex might exist.
        let _mutex_result = CreateMutexW(None, true, PCWSTR(mutex_name_wide.as_ptr()));
        
        // Check if the mutex already existed
        // GetLastError returns Result in this version.
        if let Err(error) = GetLastError() {
            // ERROR_ALREADY_EXISTS is a WIN32_ERROR. Convert to HRESULT to compare.
            if error.code() == ERROR_ALREADY_EXISTS.to_hresult() {
                // Already running
                return;
            }
        }



    }

    // Initialize logger
    // Logger removed for size optimization
    // env_logger::init();
    // info!("Gõ Nhanh (Rust) Starting...");



    // Start Hook in background thread
    hook::install();

    // Start UI (Main thread)
    // In strict Win32, UI usually runs on main thread.
    ui::run_ui();
}
