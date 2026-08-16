use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Cursor;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use windows::core::{ComInterface, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    STGM_READ,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{
    IShellLinkW, SHGetFileInfoW, ShellExecuteW, ShellLink, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, DrawIconEx, EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    SetForegroundWindow, DI_NORMAL, SW_SHOWNORMAL,
};

use super::{PlatformActivation, PlatformIcon, RunningStates, ScanResult};
use crate::services::quick_launcher::contracts::{
    launcher_error, QuickLauncherActivationAction, QuickLauncherDiagnostic, QuickLauncherItemKind,
    QuickLauncherRunningState,
};
use crate::services::quick_launcher::model::{stable_item_id, IndexedItem, LaunchTarget};
use crate::services::quick_launcher::search::{build_search_fields, bundled_aliases};

pub fn application_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(program_data) = std::env::var_os("PROGRAMDATA") {
        roots.push(
            PathBuf::from(program_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        roots.push(
            PathBuf::from(app_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    roots
}

pub fn scan() -> ScanResult {
    let mut diagnostics = Vec::new();
    let mut by_identity = HashMap::<String, IndexedItem>::new();
    for entry in super::recursive_candidates(&application_roots(), &["lnk", "exe"], false) {
        match parse_entry(&entry) {
            Ok((identity, item)) => {
                by_identity.insert(identity, item);
            }
            Err(message) => diagnostics.push(QuickLauncherDiagnostic {
                code: "launcher.windows.entry_skipped".into(),
                message,
            }),
        }
    }
    let mut items = by_identity.into_values().collect::<Vec<_>>();
    items.sort_by(|left, right| left.title.to_lowercase().cmp(&right.title.to_lowercase()));
    ScanResult { items, diagnostics }
}

fn parse_entry(entry: &Path) -> Result<(String, IndexedItem), String> {
    let extension = entry
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let canonical = entry.canonicalize().unwrap_or_else(|_| entry.to_path_buf());
    let executable_path = if extension.eq_ignore_ascii_case("lnk") {
        resolve_shortcut_target(&canonical).ok()
    } else {
        Some(canonical.clone())
    };
    if extension.eq_ignore_ascii_case("lnk") && executable_path.is_none() {
        return Err(format!(
            "{} could not be resolved safely",
            canonical.display()
        ));
    }
    let identity_path = executable_path.as_ref().unwrap_or(&canonical);
    let identity = identity_path.to_string_lossy().to_lowercase();
    let title = canonical
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{} has no display name", canonical.display()))?
        .to_string();
    let modified_at = std::fs::metadata(&canonical)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    let id = stable_item_id("windows", QuickLauncherItemKind::Application, &identity);
    let icon_key = Some(format!("{id}:{}", modified_at.unwrap_or_default()));
    Ok((
        identity,
        IndexedItem {
            id,
            kind: QuickLauncherItemKind::Application,
            title: title.clone(),
            subtitle: canonical.to_string_lossy().into_owned(),
            search: build_search_fields(&title, bundled_aliases(&title)),
            target: LaunchTarget::Application {
                path: canonical.clone(),
                bundle_id: None,
                executable_path,
            },
            icon_source: Some(canonical),
            icon_key,
            source_modified_at: modified_at,
            running: QuickLauncherRunningState::Unknown,
        },
    ))
}

fn resolve_shortcut_target(shortcut: &Path) -> Result<PathBuf, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("failed to create ShellLink: {error}"))?;
        let persist: IPersistFile = shell_link
            .cast()
            .map_err(|error| format!("failed to access shortcut persistence: {error}"))?;
        let shortcut_wide = wide(shortcut.as_os_str());
        persist
            .Load(PCWSTR(shortcut_wide.as_ptr()), STGM_READ)
            .map_err(|error| format!("failed to load shortcut: {error}"))?;
        let mut target = vec![0_u16; 32_768];
        let mut metadata = WIN32_FIND_DATAW::default();
        shell_link
            .GetPath(&mut target, &mut metadata, 0)
            .map_err(|error| format!("failed to resolve shortcut: {error}"))?;
        let length = target
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(target.len());
        if length == 0 {
            return Err("shortcut target is empty".into());
        }
        let path = PathBuf::from(String::from_utf16_lossy(&target[..length]));
        if !path.is_file() {
            return Err(format!(
                "shortcut target does not exist: {}",
                path.display()
            ));
        }
        Ok(path)
    }
}

pub fn probe_running(items: &[IndexedItem]) -> RunningStates {
    items
        .iter()
        .map(|item| {
            let state = match executable_path(item) {
                Some(path) if find_window_for_executable(path).is_some() => {
                    QuickLauncherRunningState::Running
                }
                Some(_) => QuickLauncherRunningState::NotRunning,
                None => QuickLauncherRunningState::Unknown,
            };
            (item.id.clone(), state)
        })
        .collect()
}

pub fn activate(item: &IndexedItem) -> PlatformActivation {
    if let Some(executable) = executable_path(item) {
        if let Some(window) = find_window_for_executable(executable) {
            let focused = unsafe { SetForegroundWindow(window).as_bool() };
            if focused {
                return Ok(QuickLauncherActivationAction::Focused);
            }
        }
    }
    let LaunchTarget::Application { path, .. } = &item.target else {
        return Err(launcher_error(
            "launcher.launchOrFocus",
            "launcher.item_kind_invalid",
            "The selected item is not an application.",
            false,
        ));
    };
    shell_execute(path.as_os_str(), "launcher.launchOrFocus")?;
    Ok(
        if executable_path(item)
            .and_then(find_window_for_executable)
            .is_some()
        {
            QuickLauncherActivationAction::LaunchedFallback
        } else {
            QuickLauncherActivationAction::Launched
        },
    )
}

pub fn open_setting(uri: &str) -> PlatformActivation {
    shell_execute(OsStr::new(uri), "launcher.openSystemSetting")?;
    Ok(QuickLauncherActivationAction::OpenedSetting)
}

pub fn load_icon(item: &IndexedItem) -> PlatformIcon {
    let source = item.icon_source.as_deref().ok_or_else(|| {
        launcher_error(
            "launcher.icon",
            "launcher.icon_source_missing",
            "The indexed Windows item has no icon source.",
            false,
        )
    })?;
    let source_wide = wide(source.as_os_str());
    let mut info = SHFILEINFOW::default();
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(source_wide.as_ptr()),
            Default::default(),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if result == 0 || info.hIcon.0 == 0 {
        return Ok(None);
    }
    let png = unsafe { icon_to_png(info.hIcon) };
    let _ = unsafe { DestroyIcon(info.hIcon) };
    png.map(Some).map_err(|message| {
        launcher_error(
            "launcher.icon",
            "launcher.icon_extract_failed",
            message,
            true,
        )
    })
}

unsafe fn icon_to_png(
    icon: windows::Win32::UI::WindowsAndMessaging::HICON,
) -> Result<Vec<u8>, String> {
    const SIZE: i32 = 64;
    let screen_dc = GetDC(HWND(0));
    if screen_dc.0 == 0 {
        return Err("GetDC returned no device context.".into());
    }
    let memory_dc = CreateCompatibleDC(screen_dc);
    if memory_dc.0 == 0 {
        let _ = ReleaseDC(HWND(0), screen_dc);
        return Err("CreateCompatibleDC returned no device context.".into());
    }
    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: SIZE,
            biHeight: -SIZE,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = std::ptr::null_mut();
    let bitmap = CreateDIBSection(
        screen_dc,
        &bitmap_info,
        DIB_RGB_COLORS,
        &mut pixels,
        None,
        0,
    )
    .map_err(|error| format!("CreateDIBSection failed: {error}"))?;
    let previous = SelectObject(memory_dc, HGDIOBJ(bitmap.0));
    let draw_result = DrawIconEx(memory_dc, 0, 0, icon, SIZE, SIZE, 0, None, DI_NORMAL);
    let byte_len = (SIZE * SIZE * 4) as usize;
    let mut rgba = if draw_result.is_ok() && !pixels.is_null() {
        std::slice::from_raw_parts(pixels.cast::<u8>(), byte_len).to_vec()
    } else {
        Vec::new()
    };
    let _ = SelectObject(memory_dc, previous);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(memory_dc);
    let _ = ReleaseDC(HWND(0), screen_dc);
    draw_result.map_err(|error| format!("DrawIconEx failed: {error}"))?;
    if rgba.len() != byte_len {
        return Err("Shell icon produced an invalid pixel buffer.".into());
    }
    let has_alpha = rgba.chunks_exact(4).any(|pixel| pixel[3] != 0);
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        if !has_alpha {
            pixel[3] = 255;
        }
    }
    let image = image::RgbaImage::from_raw(SIZE as u32, SIZE as u32, rgba)
        .ok_or_else(|| "Shell icon pixel dimensions were invalid.".to_string())?;
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(|error| format!("PNG encoding failed: {error}"))?;
    Ok(encoded.into_inner())
}

fn shell_execute(
    target: &OsStr,
    operation: &str,
) -> Result<(), crate::services::quick_launcher::contracts::QuickLauncherError> {
    let target = wide(target);
    let open = wide(OsStr::new("open"));
    let result = unsafe {
        ShellExecuteW(
            HWND(0),
            PCWSTR(open.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 <= 32 {
        return Err(launcher_error(
            operation,
            "launcher.shell_execute_failed",
            format!("Windows ShellExecuteW failed with code {}.", result.0),
            true,
        ));
    }
    Ok(())
}

fn executable_path(item: &IndexedItem) -> Option<&Path> {
    match &item.target {
        LaunchTarget::Application {
            executable_path, ..
        } => executable_path.as_deref(),
        LaunchTarget::SystemSetting { .. } => None,
    }
}

struct WindowSearch {
    target: String,
    found: Option<HWND>,
}

fn find_window_for_executable(executable: &Path) -> Option<HWND> {
    let target = executable.to_string_lossy().to_lowercase();
    let mut search = WindowSearch {
        target,
        found: None,
    };
    unsafe {
        let _ = EnumWindows(
            Some(enum_window),
            LPARAM((&mut search as *mut WindowSearch) as isize),
        );
    }
    search.found
}

unsafe extern "system" fn enum_window(window: HWND, parameter: LPARAM) -> BOOL {
    let search = &mut *(parameter.0 as *mut WindowSearch);
    if !IsWindowVisible(window).as_bool() {
        return BOOL(1);
    }
    let mut process_id = 0_u32;
    GetWindowThreadProcessId(window, Some(&mut process_id));
    let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) else {
        return BOOL(1);
    };
    let mut path = vec![0_u16; 32_768];
    let mut length = path.len() as u32;
    let queried = QueryFullProcessImageNameW(
        process,
        PROCESS_NAME_WIN32,
        windows::core::PWSTR(path.as_mut_ptr()),
        &mut length,
    )
    .is_ok();
    let _ = CloseHandle(process);
    if queried && String::from_utf16_lossy(&path[..length as usize]).to_lowercase() == search.target
    {
        search.found = Some(window);
        return BOOL(0);
    }
    BOOL(1)
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}
