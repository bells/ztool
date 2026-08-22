use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use super::contracts::{
    FileConversionDirection, FileConversionErrorCode,
    FileConversionProvider as FileConversionProviderSnapshot, FileConversionProviderAvailability,
    FileConversionProviderId, FileConversionProviderOrigin, FileConversionQualityProfile,
};
use super::provider::provider_error;

const LIBREOFFICE_MINIMUM: SemanticVersion = SemanticVersion::new(25, 8, 0);
const LIBREOFFICE_MAXIMUM_EXCLUSIVE: SemanticVersion = SemanticVersion::new(27, 0, 0);
const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const VERSION_OUTPUT_LIMIT: usize = 4 * 1024;

pub trait ExecutableResolver: Send + Sync {
    fn resolve(&self) -> Option<PathBuf>;
}

pub trait ExecutableVersionProbe: Send + Sync {
    fn version_output(&self, executable: &Path) -> Result<String, VersionProbeFailure>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionProbeFailure {
    PermissionDenied,
    Timeout,
    Failed,
}

#[derive(Debug, Default)]
pub struct LibreOfficeExecutableResolver;

impl ExecutableResolver for LibreOfficeExecutableResolver {
    fn resolve(&self) -> Option<PathBuf> {
        resolve_first_executable(libreoffice_candidates())
    }
}

#[derive(Debug, Default)]
pub struct DirectVersionProbe;

impl ExecutableVersionProbe for DirectVersionProbe {
    fn version_output(&self, executable: &Path) -> Result<String, VersionProbeFailure> {
        let mut command = Command::new(executable);
        command
            .args(["--headless", "--version"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(parent) = executable.parent() {
            command.current_dir(parent);
        }

        let mut child = command.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                VersionProbeFailure::PermissionDenied
            } else {
                VersionProbeFailure::Failed
            }
        })?;
        let deadline = Instant::now() + VERSION_PROBE_TIMEOUT;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(VersionProbeFailure::Timeout);
                }
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(VersionProbeFailure::Failed);
                }
            }
        };
        let stdout = read_bounded(child.stdout.take(), VERSION_OUTPUT_LIMIT);
        let stderr = read_bounded(child.stderr.take(), VERSION_OUTPUT_LIMIT);
        if !status.success() {
            return Err(VersionProbeFailure::Failed);
        }
        let output = if stdout.trim().is_empty() {
            stderr
        } else {
            stdout
        };
        if output.trim().is_empty() {
            Err(VersionProbeFailure::Failed)
        } else {
            Ok(output)
        }
    }
}

pub struct LibreOfficeDiscovery {
    resolver: Box<dyn ExecutableResolver>,
    version_probe: Box<dyn ExecutableVersionProbe>,
    cache: Mutex<Option<CachedProbe>>,
}

impl Default for LibreOfficeDiscovery {
    fn default() -> Self {
        Self::new(
            Box::<LibreOfficeExecutableResolver>::default(),
            Box::<DirectVersionProbe>::default(),
        )
    }
}

impl LibreOfficeDiscovery {
    pub fn new(
        resolver: Box<dyn ExecutableResolver>,
        version_probe: Box<dyn ExecutableVersionProbe>,
    ) -> Self {
        Self {
            resolver,
            version_probe,
            cache: Mutex::new(None),
        }
    }

    pub fn probe(&self) -> FileConversionProviderSnapshot {
        let executable = self.resolver.resolve();
        let fingerprint = executable.as_deref().and_then(executable_fingerprint);

        if let Ok(cache) = self.cache.lock() {
            if let Some(cached) = cache.as_ref() {
                if cached.fingerprint == fingerprint {
                    return cached.snapshot.clone();
                }
            }
        }

        let snapshot = match executable {
            Some(executable) => self.probe_executable(&executable),
            None => unavailable_snapshot(
                FileConversionErrorCode::EngineUnavailable,
                "LibreOffice was not found in an approved installed location.",
                true,
            ),
        };
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some(CachedProbe {
                fingerprint,
                snapshot: snapshot.clone(),
            });
        }
        snapshot
    }

    pub fn invalidate(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = None;
        }
    }

    pub fn resolved_executable(&self) -> Option<PathBuf> {
        self.resolver.resolve()
    }

    fn probe_executable(&self, executable: &Path) -> FileConversionProviderSnapshot {
        match self.version_probe.version_output(executable) {
            Ok(output) => match parse_libreoffice_version(&output) {
                Some(version) if approved_libreoffice_version(&version, &output) => {
                    FileConversionProviderSnapshot {
                        id: FileConversionProviderId::LibreOffice,
                        display_name: "LibreOffice".into(),
                        version: Some(version.to_string()),
                        origin: FileConversionProviderOrigin::Compatibility,
                        engine_version: None,
                        package_version: None,
                        platform_minimum: None,
                        quality_profiles: vec![FileConversionQualityProfile::CompatibilityProvider],
                        directions: vec![FileConversionDirection::DocxToPdf],
                        availability: FileConversionProviderAvailability::Available,
                    }
                }
                Some(version) => unavailable_snapshot_with_version(
                    FileConversionErrorCode::EngineVersionUnsupported,
                    "The installed LibreOffice version is outside Zero's approved stable range.",
                    false,
                    Some(version.to_string()),
                ),
                None => unavailable_snapshot(
                    FileConversionErrorCode::EngineVersionUnsupported,
                    "The installed LibreOffice version could not be identified safely.",
                    false,
                ),
            },
            Err(VersionProbeFailure::PermissionDenied) => unavailable_snapshot(
                FileConversionErrorCode::PermissionDenied,
                "Zero does not have permission to start the installed LibreOffice provider.",
                true,
            ),
            Err(VersionProbeFailure::Timeout) => unavailable_snapshot(
                FileConversionErrorCode::Timeout,
                "LibreOffice did not finish its local version check in time.",
                true,
            ),
            Err(VersionProbeFailure::Failed) => unavailable_snapshot(
                FileConversionErrorCode::ProviderFailed,
                "LibreOffice could not complete its local version check.",
                true,
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedProbe {
    fingerprint: Option<ExecutableFingerprint>,
    snapshot: FileConversionProviderSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExecutableFingerprint {
    path: PathBuf,
    length: u64,
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SemanticVersion {
    const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn libreoffice_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    #[cfg(target_os = "macos")]
    candidates.extend([
        PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice"),
        PathBuf::from("/Applications/LibreOfficeDev.app/Contents/MacOS/soffice"),
    ]);
    #[cfg(target_os = "windows")]
    {
        for root in [
            env::var_os("ProgramFiles"),
            env::var_os("ProgramFiles(x86)"),
        ]
        .into_iter()
        .flatten()
        {
            candidates.push(
                PathBuf::from(root)
                    .join("LibreOffice")
                    .join("program")
                    .join("soffice.exe"),
            );
        }
    }

    if let Some(search_path) = env::var_os("PATH") {
        let executable_names: &[&str] = if cfg!(windows) {
            &["soffice.exe", "soffice.com", "soffice"]
        } else {
            &["soffice"]
        };
        for directory in env::split_paths(&search_path) {
            for name in executable_names {
                candidates.push(directory.join(name));
            }
        }
    }
    candidates
}

fn resolve_first_executable(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find_map(|candidate| {
        let canonical = candidate.canonicalize().ok()?;
        let metadata = fs::metadata(&canonical).ok()?;
        if !metadata.is_file() || !is_executable(&metadata) {
            return None;
        }
        Some(canonical)
    })
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    true
}

fn executable_fingerprint(path: &Path) -> Option<ExecutableFingerprint> {
    let canonical = path.canonicalize().ok()?;
    let metadata = fs::metadata(&canonical).ok()?;
    if !metadata.is_file() || !is_executable(&metadata) {
        return None;
    }
    Some(ExecutableFingerprint {
        path: canonical,
        length: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

fn parse_libreoffice_version(output: &str) -> Option<SemanticVersion> {
    output.split_whitespace().find_map(|token| {
        let numeric = token
            .trim_matches(|character: char| !character.is_ascii_digit() && character != '.')
            .split('.')
            .take(3)
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        (numeric.len() == 3).then(|| SemanticVersion::new(numeric[0], numeric[1], numeric[2]))
    })
}

fn approved_libreoffice_version(version: &SemanticVersion, output: &str) -> bool {
    let normalized = output.to_ascii_lowercase();
    *version >= LIBREOFFICE_MINIMUM
        && *version < LIBREOFFICE_MAXIMUM_EXCLUSIVE
        && !["libreofficedev", "alpha", "beta", "release candidate"]
            .iter()
            .any(|marker| normalized.contains(marker))
        && !normalized
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| token.starts_with("rc") && token[2..].chars().all(|c| c.is_ascii_digit()))
}

fn read_bounded<R: Read>(reader: Option<R>, limit: usize) -> String {
    let Some(reader) = reader else {
        return String::new();
    };
    let mut bytes = Vec::new();
    let _ = reader.take(limit as u64).read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

fn unavailable_snapshot(
    code: FileConversionErrorCode,
    message: &str,
    retryable: bool,
) -> FileConversionProviderSnapshot {
    unavailable_snapshot_with_version(code, message, retryable, None)
}

fn unavailable_snapshot_with_version(
    code: FileConversionErrorCode,
    message: &str,
    retryable: bool,
    version: Option<String>,
) -> FileConversionProviderSnapshot {
    FileConversionProviderSnapshot {
        id: FileConversionProviderId::LibreOffice,
        display_name: "LibreOffice".into(),
        version,
        origin: FileConversionProviderOrigin::Compatibility,
        engine_version: None,
        package_version: None,
        platform_minimum: None,
        quality_profiles: vec![FileConversionQualityProfile::CompatibilityProvider],
        directions: vec![FileConversionDirection::DocxToPdf],
        availability: FileConversionProviderAvailability::Unavailable {
            error: provider_error(
                code,
                message,
                retryable,
                Some(FileConversionProviderId::LibreOffice),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct FixedResolver(PathBuf);

    impl ExecutableResolver for FixedResolver {
        fn resolve(&self) -> Option<PathBuf> {
            Some(self.0.clone())
        }
    }

    struct FakeVersionProbe {
        calls: Arc<AtomicUsize>,
        output: Mutex<Result<String, VersionProbeFailure>>,
    }

    impl ExecutableVersionProbe for FakeVersionProbe {
        fn version_output(&self, _executable: &Path) -> Result<String, VersionProbeFailure> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.output.lock().unwrap().clone()
        }
    }

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "zero-file-discovery-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn parses_and_enforces_the_recorded_stable_libreoffice_range() {
        for (output, approved) in [
            ("LibreOffice 25.8.0.0 00(Build:0)", true),
            ("LibreOffice 26.2.5.2 00(Build:2)", true),
            ("LibreOffice 25.7.9.0 00(Build:0)", false),
            ("LibreOffice 27.0.0.0 00(Build:0)", false),
            ("LibreOfficeDev 26.8.0.0.alpha0 00(Build:0)", false),
            ("LibreOffice 26.2.0.0.beta1 00(Build:0)", false),
            ("LibreOffice 26.2.0.0 rc1 00(Build:0)", false),
        ] {
            let version = parse_libreoffice_version(output).unwrap();
            assert_eq!(
                approved_libreoffice_version(&version, output),
                approved,
                "{output}"
            );
        }
    }

    #[test]
    fn resolves_only_real_executable_files_in_candidate_order() {
        let root = TestRoot::new();
        let directory = root.0.join("not-an-executable");
        let executable = root.0.join("LibreOffice 空格").join("soffice");
        fs::create_dir_all(&directory).unwrap();
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        }

        assert_eq!(
            resolve_first_executable(vec![root.0.join("missing"), directory, executable.clone()]),
            Some(executable.canonicalize().unwrap())
        );
    }

    #[test]
    fn returns_structured_unavailable_and_unsupported_reasons() {
        let root = TestRoot::new();
        let executable = executable_fixture(&root.0);
        for (result, expected) in [
            (
                Ok("LibreOffice 24.2.0.0 00(Build:0)".into()),
                FileConversionErrorCode::EngineVersionUnsupported,
            ),
            (
                Err(VersionProbeFailure::PermissionDenied),
                FileConversionErrorCode::PermissionDenied,
            ),
            (
                Err(VersionProbeFailure::Timeout),
                FileConversionErrorCode::Timeout,
            ),
            (
                Err(VersionProbeFailure::Failed),
                FileConversionErrorCode::ProviderFailed,
            ),
        ] {
            let discovery = discovery(executable.clone(), result, Arc::new(AtomicUsize::new(0)));
            let snapshot = discovery.probe();
            assert!(matches!(
                snapshot.availability,
                FileConversionProviderAvailability::Unavailable {
                    error: super::super::contracts::FileConversionError { code, .. }
                } if code == expected
            ));
        }
    }

    #[test]
    fn caches_unchanged_probes_and_supports_explicit_invalidation() {
        let root = TestRoot::new();
        let executable = executable_fixture(&root.0);
        let calls = Arc::new(AtomicUsize::new(0));
        let discovery = discovery(
            executable,
            Ok("LibreOffice 26.2.5.2 00(Build:2)".into()),
            Arc::clone(&calls),
        );

        assert!(matches!(
            discovery.probe().availability,
            FileConversionProviderAvailability::Available
        ));
        discovery.probe();
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        discovery.invalidate();
        discovery.probe();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    fn discovery(
        executable: PathBuf,
        output: Result<String, VersionProbeFailure>,
        calls: Arc<AtomicUsize>,
    ) -> LibreOfficeDiscovery {
        LibreOfficeDiscovery::new(
            Box::new(FixedResolver(executable)),
            Box::new(FakeVersionProbe {
                calls,
                output: Mutex::new(output),
            }),
        )
    }

    fn executable_fixture(root: &Path) -> PathBuf {
        let executable = root.join("soffice");
        fs::write(&executable, b"fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        }
        executable
    }
}
