use log::{debug, warn};
use windows::core::Error;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, RegisterClassW, CS_VREDRAW, CW_USEDEFAULT, HMENU, HWND_MESSAGE,
    WINDOW_EX_STYLE, WNDPROC, WS_MINIMIZE,
};

#[derive(Default)]
pub struct MessageOnlyWindow {
    hwnd: HWND,
}

impl MessageOnlyWindow {
    pub fn new(window_class_name: &str, wnd_proc: &WNDPROC) -> Result<MessageOnlyWindow, String> {
        let hinstance = { unsafe { GetModuleHandleW(PCWSTR::null()).unwrap() } };
        let utf16_class_name: Vec<u16> = window_class_name.encode_utf16().collect();

        let window_class = WNDCLASSW {
            style: CS_VREDRAW,
            lpfnWndProc: wnd_proc.to_owned(),
            hInstance: hinstance,
            lpszClassName: PCWSTR::from_raw(utf16_class_name.as_ptr()),

            ..Default::default()
        };

        let register_result = { unsafe { RegisterClassW(&window_class) } };
        if register_result != 0 {
            debug!("Register window class");
        } else {
            return Err(format!("RegisterClassW failed: {}", Error::from_win32()));
        }

        let hwnd = {
            unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    PCWSTR::from_raw(utf16_class_name.as_ptr()),
                    PCWSTR::null(),
                    WS_MINIMIZE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    HWND_MESSAGE,
                    HMENU::default(),
                    hinstance,
                    None,
                )
            }
        };

        if hwnd == HWND::default() {
            return Err(format!("CreateWindowExW failed: {}", Error::from_win32()));
        }

        Ok(MessageOnlyWindow { hwnd })
    }
}

impl Drop for MessageOnlyWindow {
    fn drop(&mut self) {
        let result = { unsafe { DestroyWindow(self.hwnd) } };
        if result.as_bool() {
            debug!("Destroy message-only window");
        } else {
            warn!("DestroyWindow failed: {}", Error::from_win32());
        }
    }
}
