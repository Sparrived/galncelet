#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWM_SYSTEMBACKDROP_TYPE,
};

/// Enable acrylic/mica-like backdrop on Windows 11.
/// Falls back gracefully if not supported.
#[allow(dead_code)]
#[cfg(target_os = "windows")]
pub fn enable_backdrop(hwnd_raw: isize) -> Result<(), String> {
    let hwnd = HWND(hwnd_raw as *mut _);

    // Try to set system backdrop type (Windows 11 22H2+)
    // DWM_SYSTEMBACKDROP_TYPE(2) = DWMSBT_TRANSIENTWINDOW = Acrylic
    let backdrop_type = DWM_SYSTEMBACKDROP_TYPE(2);

    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop_type as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        )
    }
    .map_err(|e| format!("DwmSetWindowAttribute failed: {e}"))?;

    Ok(())
}

/// Enable backdrop on non-Windows (no-op).
#[cfg(not(target_os = "windows"))]
pub fn enable_backdrop(_hwnd: isize) -> Result<(), String> {
    Ok(())
}
