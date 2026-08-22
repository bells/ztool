export interface BingWallpaperItem {
  id: string;
  startDate: string;
  title: string;
  attribution: string;
  copyrightUrl?: string;
  remoteUrl: string;
  cacheFileName: string;
  previewFileName?: string;
  cached: boolean;
}

export interface WallpaperPlatformCapability {
  platform: string;
  supported: boolean;
  detail?: string;
}

export interface BingWallpaperError {
  code: string;
  message: string;
  retryable: boolean;
}

export interface BingWallpaperSnapshot {
  items: BingWallpaperItem[];
  refreshedAt?: string;
  market: string;
  stale: boolean;
  platform: WallpaperPlatformCapability;
  error?: BingWallpaperError;
}

export interface BingWallpaperPreview {
  wallpaperId: string;
  token: string;
  mimeType: string;
  byteLength: number;
  width: number;
  height: number;
  expiresAtMs: number;
}

export interface BingWallpaperPreviewResourceInput {
  token: string;
}

export interface BingWallpaperActionInput {
  wallpaperId: string;
}

export interface BingWallpaperActionResult {
  wallpaperId: string;
  path: string;
  message: string;
}
