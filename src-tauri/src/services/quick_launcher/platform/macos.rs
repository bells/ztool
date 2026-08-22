#![allow(deprecated, unexpected_cfgs)]

use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::{class, msg_send, sel, sel_impl};
use plist::Value;

use super::{PlatformActivation, PlatformIcon, RunningStates, ScanResult};
use crate::services::quick_launcher::contracts::{
    launcher_error, QuickLauncherActivationAction, QuickLauncherDiagnostic, QuickLauncherItemKind,
    QuickLauncherRunningState,
};
use crate::services::quick_launcher::model::{stable_item_id, IndexedItem, LaunchTarget};
use crate::services::quick_launcher::search::{build_search_fields, bundled_aliases};

pub fn application_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        roots.insert(1, PathBuf::from(home).join("Applications"));
    }
    roots
}

pub fn scan() -> ScanResult {
    let mut diagnostics = Vec::new();
    let mut by_identity = HashMap::<String, IndexedItem>::new();
    for bundle in super::recursive_candidates(&application_roots(), &["app"], true) {
        match parse_bundle(&bundle) {
            Ok((identity, item)) => {
                by_identity.entry(identity).or_insert(item);
            }
            Err(message) => diagnostics.push(QuickLauncherDiagnostic {
                code: "launcher.macos.bundle_skipped".into(),
                message,
            }),
        }
    }
    let mut items = by_identity.into_values().collect::<Vec<_>>();
    items.sort_by_key(|item| item.title.to_lowercase());
    ScanResult { items, diagnostics }
}

fn parse_bundle(bundle: &Path) -> Result<(String, IndexedItem), String> {
    let canonical = bundle
        .canonicalize()
        .unwrap_or_else(|_| bundle.to_path_buf());
    let plist_path = canonical.join("Contents/Info.plist");
    let dictionary = Value::from_file(&plist_path)
        .map_err(|error| format!("{}: {error}", plist_path.display()))?
        .into_dictionary()
        .ok_or_else(|| format!("{} is not a plist dictionary", plist_path.display()))?;
    let display_name = plist_string(&dictionary, "CFBundleDisplayName")
        .or_else(|| plist_string(&dictionary, "CFBundleName"))
        .or_else(|| {
            canonical
                .file_stem()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| format!("{} has no display name", canonical.display()))?;
    let bundle_id = plist_string(&dictionary, "CFBundleIdentifier");
    let executable_path = plist_string(&dictionary, "CFBundleExecutable")
        .map(|name| canonical.join("Contents/MacOS").join(name));
    if executable_path.as_ref().is_some_and(|path| !path.exists()) {
        return Err(format!("{} has no valid executable", canonical.display()));
    }
    let identity = bundle_id
        .clone()
        .unwrap_or_else(|| canonical.to_string_lossy().to_lowercase());
    let modified_at = std::fs::metadata(&canonical)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    let id = stable_item_id("macos", QuickLauncherItemKind::Application, &identity);
    let icon_key = Some(format!("{id}:{}", modified_at.unwrap_or_default()));
    Ok((
        identity,
        IndexedItem {
            id,
            kind: QuickLauncherItemKind::Application,
            title: display_name.clone(),
            subtitle: canonical.to_string_lossy().into_owned(),
            search: build_search_fields(&display_name, bundled_aliases(&display_name)),
            target: LaunchTarget::Application {
                path: canonical.clone(),
                bundle_id,
                executable_path,
            },
            icon_source: Some(canonical),
            icon_key,
            source_modified_at: modified_at,
            running: QuickLauncherRunningState::Unknown,
        },
    ))
}

fn plist_string(dictionary: &plist::Dictionary, key: &str) -> Option<String> {
    dictionary
        .get(key)
        .and_then(Value::as_string)
        .map(str::to_string)
}

pub fn probe_running(items: &[IndexedItem]) -> RunningStates {
    let running_bundle_ids = running_bundle_ids();
    items
        .iter()
        .map(|item| {
            let state = match &item.target {
                LaunchTarget::Application {
                    bundle_id: Some(bundle_id),
                    ..
                } => {
                    if running_bundle_ids.contains(bundle_id) {
                        QuickLauncherRunningState::Running
                    } else {
                        QuickLauncherRunningState::NotRunning
                    }
                }
                LaunchTarget::Application { .. } => QuickLauncherRunningState::Unknown,
                LaunchTarget::SystemSetting { .. } => QuickLauncherRunningState::NotApplicable,
            };
            (item.id.clone(), state)
        })
        .collect()
}

fn running_bundle_ids() -> HashSet<String> {
    unsafe {
        let pool: id = msg_send![class!(NSAutoreleasePool), new];
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let applications: id = msg_send![workspace, runningApplications];
        let count: usize = msg_send![applications, count];
        let mut bundle_ids = HashSet::new();
        for index in 0..count {
            let application: id = msg_send![applications, objectAtIndex: index];
            let bundle_id: id = msg_send![application, bundleIdentifier];
            if bundle_id != nil {
                let bytes: *const std::os::raw::c_char = msg_send![bundle_id, UTF8String];
                if !bytes.is_null() {
                    bundle_ids.insert(CStr::from_ptr(bytes).to_string_lossy().into_owned());
                }
            }
        }
        let _: () = msg_send![pool, drain];
        bundle_ids
    }
}

pub fn activate(item: &IndexedItem) -> PlatformActivation {
    let LaunchTarget::Application {
        path, bundle_id, ..
    } = &item.target
    else {
        return Err(launcher_error(
            "launcher.launchOrFocus",
            "launcher.item_kind_invalid",
            "The selected item is not an application.",
            false,
        ));
    };
    unsafe {
        let pool: id = msg_send![class!(NSAutoreleasePool), new];
        if let Some(bundle_id) = bundle_id {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let applications: id = msg_send![workspace, runningApplications];
            let count: usize = msg_send![applications, count];
            for index in 0..count {
                let application: id = msg_send![applications, objectAtIndex: index];
                let native_bundle_id: id = msg_send![application, bundleIdentifier];
                if nsstring_equals(native_bundle_id, bundle_id) {
                    let activated: bool = msg_send![application, activateWithOptions: 2usize];
                    let _: () = msg_send![pool, drain];
                    return if activated {
                        Ok(QuickLauncherActivationAction::Focused)
                    } else {
                        Err(launcher_error(
                            "launcher.launchOrFocus",
                            "launcher.focus_denied",
                            format!("macOS did not focus {}.", item.title),
                            true,
                        ))
                    };
                }
            }
        }
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let native_path = NSString::alloc(nil).init_str(&path.to_string_lossy());
        let opened: bool = msg_send![workspace, openFile: native_path];
        let _: () = msg_send![native_path, release];
        let _: () = msg_send![pool, drain];
        if opened {
            Ok(QuickLauncherActivationAction::Launched)
        } else {
            Err(launcher_error(
                "launcher.launchOrFocus",
                "launcher.launch_failed",
                format!("macOS could not launch {}.", item.title),
                true,
            ))
        }
    }
}

pub fn open_setting(uri: &str) -> PlatformActivation {
    unsafe {
        let pool: id = msg_send![class!(NSAutoreleasePool), new];
        let native_uri = NSString::alloc(nil).init_str(uri);
        let url: id = msg_send![class!(NSURL), URLWithString: native_uri];
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let opened: bool = msg_send![workspace, openURL: url];
        let _: () = msg_send![native_uri, release];
        let _: () = msg_send![pool, drain];
        if opened {
            Ok(QuickLauncherActivationAction::OpenedSetting)
        } else {
            Err(launcher_error(
                "launcher.openSystemSetting",
                "launcher.setting_open_failed",
                "macOS did not accept the system setting destination.",
                true,
            ))
        }
    }
}

pub fn load_icon(item: &IndexedItem) -> PlatformIcon {
    let Some(path) = item.icon_source.as_ref() else {
        return Ok(None);
    };
    unsafe {
        let pool: id = msg_send![class!(NSAutoreleasePool), new];
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let native_path = NSString::alloc(nil).init_str(&path.to_string_lossy());
        let image: id = msg_send![workspace, iconForFile: native_path];
        let tiff: id = msg_send![image, TIFFRepresentation];
        let bitmap: id = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
        let properties: id = msg_send![class!(NSDictionary), dictionary];
        let data: id = msg_send![bitmap, representationUsingType: 4usize properties: properties];
        let result = if data == nil {
            None
        } else {
            let length: usize = msg_send![data, length];
            let bytes: *const u8 = msg_send![data, bytes];
            if bytes.is_null() || length == 0 || length > 2 * 1024 * 1024 {
                None
            } else {
                Some(std::slice::from_raw_parts(bytes, length).to_vec())
            }
        };
        let _: () = msg_send![native_path, release];
        let _: () = msg_send![pool, drain];
        Ok(result)
    }
}

unsafe fn nsstring_equals(value: id, expected: &str) -> bool {
    if value == nil {
        return false;
    }
    let bytes: *const std::os::raw::c_char = msg_send![value, UTF8String];
    !bytes.is_null() && CStr::from_ptr(bytes).to_string_lossy() == expected
}
