use std::sync::Arc;

use super::contracts::{
    FileConversionCapabilitySnapshot, FileConversionDirection, FileConversionDirectionCapability,
    FileConversionError, FileConversionErrorCode, FileConversionProvider,
    FileConversionProviderAvailability, FileConversionProviderId,
};
use super::discovery::LibreOfficeDiscovery;
use super::libreoffice::LibreOfficeProvider;
use super::provider::{
    provider_error, FileConversionProvider as FileConversionProviderAdapter,
    FileConversionProviderRegistry, ProviderPlatform,
};
use super::word_macos::MicrosoftWordMacosProvider;
use super::word_windows::MicrosoftWordWindowsProvider;

pub const PDF_TO_DOCX_PROVIDER_APPROVED: bool = false;
pub const BUNDLED_FILE_CONVERSION_ENGINE_APPROVED: bool = false;

pub fn default_provider_registry() -> FileConversionProviderRegistry {
    let libreoffice_discovery = Arc::new(LibreOfficeDiscovery::default());
    let providers: Vec<Arc<dyn FileConversionProviderAdapter>> = vec![
        Arc::new(LibreOfficeProvider::new(libreoffice_discovery)),
        Arc::new(MicrosoftWordMacosProvider::default()),
        Arc::new(MicrosoftWordWindowsProvider::default()),
    ];
    FileConversionProviderRegistry::new(providers)
}

pub fn capability_snapshot(
    registry: &FileConversionProviderRegistry,
    platform: ProviderPlatform,
    refreshed_at_ms: u64,
) -> FileConversionCapabilitySnapshot {
    let providers = registry.snapshots();
    capability_snapshot_from_providers(providers, platform, refreshed_at_ms)
}

pub fn capability_snapshot_from_providers(
    providers: Vec<FileConversionProvider>,
    platform: ProviderPlatform,
    refreshed_at_ms: u64,
) -> FileConversionCapabilitySnapshot {
    FileConversionCapabilitySnapshot {
        directions: [
            FileConversionDirection::PdfToDocx,
            FileConversionDirection::DocxToPdf,
        ]
        .into_iter()
        .map(|direction| direction_capability(direction, &providers, platform))
        .collect(),
        refreshed_at_ms,
    }
}

fn direction_capability(
    direction: FileConversionDirection,
    providers: &[FileConversionProvider],
    platform: ProviderPlatform,
) -> FileConversionDirectionCapability {
    if direction == FileConversionDirection::PdfToDocx && !PDF_TO_DOCX_PROVIDER_APPROVED {
        return FileConversionDirectionCapability {
            direction,
            available: false,
            selected_provider_id: None,
            providers: Vec::new(),
            unavailability: Some(provider_error(
                FileConversionErrorCode::EngineUnavailable,
                "No PDF-to-DOCX provider passed Zero's local quality and redistribution gate.",
                false,
                None,
            )),
        };
    }

    let provider_ids = provider_priority(direction, platform);
    let relevant = provider_ids
        .iter()
        .filter_map(|provider_id| {
            providers
                .iter()
                .find(|provider| provider.id == *provider_id)
        })
        .filter(|provider| provider.directions.contains(&direction))
        .cloned()
        .collect::<Vec<_>>();
    let selected_provider_id = relevant.iter().find_map(|provider| {
        matches!(
            provider.availability,
            FileConversionProviderAvailability::Available
        )
        .then_some(provider.id)
    });
    let unavailability = selected_provider_id
        .is_none()
        .then(|| best_unavailability(&relevant, direction));

    FileConversionDirectionCapability {
        direction,
        available: selected_provider_id.is_some(),
        selected_provider_id,
        providers: relevant,
        unavailability,
    }
}

fn provider_priority(
    direction: FileConversionDirection,
    platform: ProviderPlatform,
) -> Vec<FileConversionProviderId> {
    match (direction, platform) {
        (FileConversionDirection::DocxToPdf, ProviderPlatform::Macos) => vec![
            FileConversionProviderId::LibreOffice,
            FileConversionProviderId::MicrosoftWordMacos,
        ],
        (FileConversionDirection::DocxToPdf, ProviderPlatform::Windows) => vec![
            FileConversionProviderId::LibreOffice,
            FileConversionProviderId::MicrosoftWordWindows,
        ],
        _ => Vec::new(),
    }
}

fn best_unavailability(
    providers: &[FileConversionProvider],
    direction: FileConversionDirection,
) -> FileConversionError {
    providers
        .iter()
        .find_map(|provider| match &provider.availability {
            FileConversionProviderAvailability::Unavailable { error }
                if error.code != FileConversionErrorCode::EngineUnavailable =>
            {
                Some(error.clone())
            }
            _ => None,
        })
        .or_else(|| {
            providers.iter().find_map(|provider| match &provider.availability {
                FileConversionProviderAvailability::Unavailable { error } => Some(error.clone()),
                FileConversionProviderAvailability::Available => None,
            })
        })
        .unwrap_or_else(|| {
            provider_error(
                FileConversionErrorCode::EngineUnavailable,
                match direction {
                    FileConversionDirection::PdfToDocx => {
                        "No approved local PDF-to-DOCX provider is available."
                    }
                    FileConversionDirection::DocxToPdf => {
                        "Install a supported LibreOffice or Microsoft Word provider for DOCX-to-PDF conversion."
                    }
                },
                true,
                None,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_to_docx_stays_explicitly_unavailable_without_a_detected_or_bundled_provider() {
        let snapshot = capability_snapshot_from_providers(Vec::new(), ProviderPlatform::Macos, 10);
        let pdf = snapshot
            .directions
            .iter()
            .find(|capability| capability.direction == FileConversionDirection::PdfToDocx)
            .unwrap();

        const {
            assert!(!PDF_TO_DOCX_PROVIDER_APPROVED);
            assert!(!BUNDLED_FILE_CONVERSION_ENGINE_APPROVED);
        }
        assert!(!pdf.available);
        assert!(pdf.providers.is_empty());
        assert_eq!(
            pdf.unavailability.as_ref().unwrap().code,
            FileConversionErrorCode::EngineUnavailable
        );
    }

    #[test]
    fn docx_to_pdf_selects_libreoffice_then_the_platform_word_fallback() {
        let unavailable_libreoffice = provider(
            FileConversionProviderId::LibreOffice,
            FileConversionProviderAvailability::Unavailable {
                error: provider_error(
                    FileConversionErrorCode::EngineUnavailable,
                    "LibreOffice missing.",
                    true,
                    Some(FileConversionProviderId::LibreOffice),
                ),
            },
        );
        let available_word = provider(
            FileConversionProviderId::MicrosoftWordMacos,
            FileConversionProviderAvailability::Available,
        );
        let snapshot = capability_snapshot_from_providers(
            vec![available_word, unavailable_libreoffice],
            ProviderPlatform::Macos,
            20,
        );
        let docx = snapshot
            .directions
            .iter()
            .find(|capability| capability.direction == FileConversionDirection::DocxToPdf)
            .unwrap();

        assert!(docx.available);
        assert_eq!(
            docx.selected_provider_id,
            Some(FileConversionProviderId::MicrosoftWordMacos)
        );
        assert_eq!(docx.providers[0].id, FileConversionProviderId::LibreOffice);
        assert_eq!(
            docx.providers[1].id,
            FileConversionProviderId::MicrosoftWordMacos
        );
        assert!(docx.unavailability.is_none());
    }

    #[test]
    fn unsupported_or_permission_failures_are_more_actionable_than_missing_provider_reasons() {
        let providers = vec![
            provider(
                FileConversionProviderId::LibreOffice,
                FileConversionProviderAvailability::Unavailable {
                    error: provider_error(
                        FileConversionErrorCode::EngineVersionUnsupported,
                        "Unsupported version.",
                        false,
                        Some(FileConversionProviderId::LibreOffice),
                    ),
                },
            ),
            provider(
                FileConversionProviderId::MicrosoftWordMacos,
                FileConversionProviderAvailability::Unavailable {
                    error: provider_error(
                        FileConversionErrorCode::EngineUnavailable,
                        "Word missing.",
                        true,
                        Some(FileConversionProviderId::MicrosoftWordMacos),
                    ),
                },
            ),
        ];

        let snapshot = capability_snapshot_from_providers(providers, ProviderPlatform::Macos, 30);
        let docx = snapshot
            .directions
            .iter()
            .find(|capability| capability.direction == FileConversionDirection::DocxToPdf)
            .unwrap();
        assert_eq!(
            docx.unavailability.as_ref().unwrap().code,
            FileConversionErrorCode::EngineVersionUnsupported
        );
    }

    fn provider(
        id: FileConversionProviderId,
        availability: FileConversionProviderAvailability,
    ) -> FileConversionProvider {
        FileConversionProvider {
            id,
            display_name: format!("{id:?}"),
            version: None,
            directions: vec![FileConversionDirection::DocxToPdf],
            availability,
        }
    }
}
