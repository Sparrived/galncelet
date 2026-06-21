/// Read the current URL from a browser window using Windows UI Automation.

const BROWSERS: &[&str] = &["chrome.exe", "msedge.exe", "brave.exe", "vivaldi.exe"];

pub fn is_browser(process: &str) -> bool {
    let p = process.to_lowercase();
    BROWSERS.iter().any(|b| p.contains(b))
}

#[cfg(target_os = "windows")]
pub fn read_browser_url(hwnd: isize) -> Option<String> {
    use uiautomation::UIAutomation;
    use uiautomation::types::Handle;
    use uiautomation::controls::ControlType;
    use uiautomation::patterns::UIValuePattern;
    use uiautomation::types::TreeScope;

    let automation = UIAutomation::new().ok()?;
    let root = automation.element_from_handle(Handle::from(hwnd)).ok()?;

    find_url(&automation, &root, 0)
}

fn find_url(
    automation: &uiautomation::UIAutomation,
    element: &uiautomation::UIElement,
    depth: u32,
) -> Option<String> {
    use uiautomation::controls::ControlType;
    use uiautomation::patterns::UIValuePattern;
    use uiautomation::types::TreeScope;

    if depth > 8 {
        return None;
    }

    // Check if this is an edit control with a URL value
    if let Ok(ct) = element.get_control_type() {
        if ct == ControlType::Edit {
            if let Ok(pattern) = element.get_pattern::<UIValuePattern>() {
                if let Ok(value) = pattern.get_value() {
                    if value.starts_with("http://") || value.starts_with("https://") {
                        return Some(value);
                    }
                }
            }
        }
    }

    // Recurse into children
    let condition = match automation.create_true_condition() {
        Ok(c) => c,
        Err(_) => return None,
    };
    let children = match element.find_all(TreeScope::Children, &condition) {
        Ok(c) => c,
        Err(_) => return None,
    };

    for child in children {
        if let Some(url) = find_url(automation, &child, depth + 1) {
            return Some(url);
        }
    }

    None
}
