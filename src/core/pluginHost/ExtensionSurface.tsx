import { buildExtensionSurfacePolicy } from "./extensionBridge";

interface ExtensionSurfaceProps {
  title: string;
  assetUrl: string;
  onLoadError: (message: string) => void;
}

export function ExtensionSurface({
  title,
  assetUrl,
  onLoadError,
}: ExtensionSurfaceProps) {
  const policy = buildExtensionSurfacePolicy();

  return (
    <iframe
      className="extension-surface"
      title={title}
      src={assetUrl}
      sandbox={policy.sandbox}
      referrerPolicy="no-referrer"
      onError={() => onLoadError(`Failed to load extension surface: ${title}`)}
    />
  );
}
