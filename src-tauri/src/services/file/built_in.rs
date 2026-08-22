use std::sync::Arc;

use tauri::Manager;

use crate::brand::ZERO_FILE_PLUGIN_ID;
use crate::plugins::registry::PluginRegistryState;

use super::contracts::{
    FileConversionDirection, FileConversionError, FileConversionErrorCode,
    FileConversionProvider as FileConversionProviderSnapshot, FileConversionProviderAvailability,
    FileConversionProviderId, FileConversionProviderOrigin, FileConversionQualityProfile,
};
use super::engine_bridge::{FileEngineBridge, FILE_ENGINE_VERSION};
use super::provider::{
    provider_error, FileConversionCancellationToken, FileConversionProgressSink,
    FileConversionProvider, ProviderConversionOutput, ProviderConversionRequest,
};

const PDF_TO_DOCX: [FileConversionDirection; 1] = [FileConversionDirection::PdfToDocx];
const DOCX_TO_PDF: [FileConversionDirection; 1] = [FileConversionDirection::DocxToPdf];

pub struct ZeroFileBuiltInProvider {
    app: tauri::AppHandle,
    bridge: Arc<FileEngineBridge>,
    provider_id: FileConversionProviderId,
}

impl ZeroFileBuiltInProvider {
    pub fn pdf_to_docx(app: tauri::AppHandle, bridge: Arc<FileEngineBridge>) -> Self {
        Self {
            app,
            bridge,
            provider_id: FileConversionProviderId::ZeroFilePdfToDocx,
        }
    }

    pub fn docx_to_pdf_macos(app: tauri::AppHandle, bridge: Arc<FileEngineBridge>) -> Self {
        Self {
            app,
            bridge,
            provider_id: FileConversionProviderId::ZeroFileDocxToPdfMacos,
        }
    }

    fn is_platform_supported(&self) -> bool {
        match self.provider_id {
            FileConversionProviderId::ZeroFilePdfToDocx => {
                cfg!(any(target_os = "macos", target_os = "windows"))
            }
            FileConversionProviderId::ZeroFileDocxToPdfMacos => {
                super::native_print::native_pdf_export_available()
            }
            _ => false,
        }
    }

    fn active_engine_version(&self) -> Result<String, String> {
        self.app
            .state::<PluginRegistryState>()
            .with_registry(|registry| registry.active_engine(ZERO_FILE_PLUGIN_ID))
            .map(|engine| engine.package_version)
            .or_else(|error| {
                super::engine_bridge::development_assets_enabled()
                    .then(|| FILE_ENGINE_VERSION.to_string())
                    .ok_or(error)
            })
    }
}

impl FileConversionProvider for ZeroFileBuiltInProvider {
    fn id(&self) -> FileConversionProviderId {
        self.provider_id
    }

    fn supported_directions(&self) -> &[FileConversionDirection] {
        match self.provider_id {
            FileConversionProviderId::ZeroFilePdfToDocx => &PDF_TO_DOCX,
            FileConversionProviderId::ZeroFileDocxToPdfMacos => &DOCX_TO_PDF,
            _ => &[],
        }
    }

    fn probe(&self) -> FileConversionProviderSnapshot {
        let supported = self.is_platform_supported();
        let engine_version = self.active_engine_version();
        let available = supported && engine_version.is_ok();
        FileConversionProviderSnapshot {
            id: self.provider_id,
            display_name: match self.provider_id {
                FileConversionProviderId::ZeroFilePdfToDocx => "Zero File PDF Engine",
                FileConversionProviderId::ZeroFileDocxToPdfMacos => "Zero File macOS Print Engine",
                _ => "Zero File Engine",
            }
            .into(),
            version: engine_version.clone().ok(),
            origin: FileConversionProviderOrigin::BuiltIn,
            engine_version: engine_version.clone().ok(),
            package_version: engine_version.clone().ok(),
            platform_minimum: match self.provider_id {
                FileConversionProviderId::ZeroFileDocxToPdfMacos => Some("macOS 11".into()),
                _ => None,
            },
            quality_profiles: match self.provider_id {
                FileConversionProviderId::ZeroFilePdfToDocx => vec![
                    FileConversionQualityProfile::EditableReconstruction,
                    FileConversionQualityProfile::LayoutPreserving,
                ],
                FileConversionProviderId::ZeroFileDocxToPdfMacos => {
                    vec![FileConversionQualityProfile::WebRenderedPdf]
                }
                _ => Vec::new(),
            },
            directions: self.supported_directions().to_vec(),
            availability: if available {
                FileConversionProviderAvailability::Available
            } else {
                FileConversionProviderAvailability::Unavailable {
                    error: {
                        let mut error = provider_error(
                            FileConversionErrorCode::EngineUnavailable,
                            if supported {
                                "Install or repair the signed Zero File plugin to activate its offline engine."
                            } else {
                                "This built-in conversion direction is not available on the current platform."
                            },
                            supported,
                            Some(self.provider_id),
                        );
                        error.diagnostic = engine_version.err();
                        error
                    },
                }
            },
        }
    }

    fn convert(
        &self,
        request: &ProviderConversionRequest,
        progress: &dyn FileConversionProgressSink,
        cancellation: &FileConversionCancellationToken,
    ) -> Result<ProviderConversionOutput, FileConversionError> {
        if !self.is_platform_supported()
            || !self.supported_directions().contains(&request.direction)
        {
            return Err(provider_error(
                FileConversionErrorCode::UnsupportedInput,
                "The selected built-in provider does not support this direction on the current platform.",
                false,
                Some(self.provider_id),
            ));
        }
        self.bridge
            .convert(&self.app, request, progress, cancellation)
    }
}
