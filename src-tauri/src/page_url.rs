/// Read the current URL from a browser window using Windows UI Automation.
/// No keyboard simulation — reads the address bar value directly via accessibility API.

const BROWSERS: &[&str] = &["chrome.exe", "msedge.exe", "brave.exe", "vivaldi.exe"];

pub fn is_browser(process: &str) -> bool {
    let p = process.to_lowercase();
    BROWSERS.iter().any(|b| p.contains(b))
}

/// Try to read the URL from the browser's address bar via UI Automation.
/// Falls back to None if the browser doesn't expose the URL.
#[cfg(target_os = "windows")]
pub fn read_browser_url(hwnd: isize) -> Option<String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement,
        IUIAutomationValuePattern, TreeScope_Children, UIA_ValuePatternId,
    };
    use windows::Win32::System::Com::{CoInitializeEx, CoCreateInstance, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
    use windows::core::Interface;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let automation: IUIAutomation = match CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) {
            Ok(a) => a,
            Err(_) => return None,
        };

        let element = match automation.ElementFromHandle(HWND(hwnd as _)) {
            Ok(e) => e,
            Err(_) => return None,
        };

        find_url_in_tree(&automation, &element, 0)
    }
}

/// Recursively search the UIA tree for an edit control containing a URL.
#[cfg(target_os = "windows")]
unsafe fn find_url_in_tree(
    automation: &IUIAutomation,
    element: &IUIAutomationElement,
    depth: u32,
) -> Option<String> {
    use windows::Win32::UI::Accessibility::{IUIAutomationValuePattern, TreeScope_Children, UIA_ValuePatternId};
    use windows::core::Interface;

    if depth > 8 {
        return None;
    }

    // Check if this element is an edit control with a URL value
    if let Ok(control_type) = element.CurrentControlType() {
        if control_type == 50004 { // UIA_EditControlTypeId
            if let Ok(pattern) = element.GetCurrentPattern(UIA_ValuePatternId) {
                if let Ok(value_pattern) = pattern.cast::<IUIAutomationValuePattern>() {
                    if let Ok(value) = value_pattern.CurrentValue() {
                        let url = value.to_string();
                        if url.starts_with("http://") || url.starts_with("https://") {
                            return Some(url);
                        }
                    }
                }
            }
        }
    }

    // Recurse into children
    if let Ok(condition) = automation.CreateTrueCondition() {
        if let Ok(children) = element.FindAll(TreeScope_Children, &condition) {
            let len = children.Length().unwrap_or(0);
            for i in 0..len {
                if let Ok(child) = children.GetElement(i) {
                    if let Some(url) = find_url_in_tree(automation, &child, depth + 1) {
                        return Some(url);
                    }
                }
            }
        }
    }

    None
}
