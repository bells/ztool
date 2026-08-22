import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ScreenshotError, ScreenshotMediaDescriptor } from "./captureTypes";
import { releaseObjectUrl } from "./captureExport";

interface PinPayload {
  media: ScreenshotMediaDescriptor;
}

function screenshotErrorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = (error as ScreenshotError).message;
    if (typeof message === "string") {
      return message;
    }
  }
  return String(error);
}

export function PinApp() {
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const imageRef = useRef<HTMLImageElement | null>(null);

  useEffect(() => {
    let disposed = false;
    let objectUrl: string | null = null;
    let receivedBytes: Uint8Array | null = null;

    void (async () => {
      try {
        const payload = await invoke<PinPayload>("init_pin_window");
        const value = await invoke<ArrayBuffer>("read_screenshot_media", {
          input: { token: payload.media.token },
        });
        receivedBytes = new Uint8Array(value);
        if (
          receivedBytes.byteLength === 0 ||
          receivedBytes.byteLength !== payload.media.byteLength
        ) {
          throw new Error("The pinned screenshot size does not match its descriptor");
        }
        objectUrl = URL.createObjectURL(new Blob([value], { type: payload.media.mimeType }));
        if (disposed) {
          objectUrl = releaseObjectUrl(objectUrl);
          return;
        }
        setImageSrc(objectUrl);
      } catch (reason) {
        objectUrl = releaseObjectUrl(objectUrl);
        if (!disposed) {
          setError(screenshotErrorMessage(reason));
        }
      } finally {
        receivedBytes?.fill(0);
        receivedBytes = null;
      }
    })();

    return () => {
      disposed = true;
      if (imageRef.current) {
        imageRef.current.src = "";
      }
      objectUrl = releaseObjectUrl(objectUrl);
    };
  }, []);

  const close = () => {
    invoke("close_current_surface").catch(() => undefined);
  };

  return (
    <main className="pin-shell">
      <header className="pin-titlebar" data-tauri-drag-region>
        <span data-tauri-drag-region>Pin</span>
        <button type="button" className="pin-close" onClick={close} aria-label="Close pinned image">
          x
        </button>
      </header>
      {error ? <p className="pin-error">{error}</p> : null}
      {imageSrc ? (
        <img
          ref={imageRef}
          className="pin-image"
          src={imageSrc}
          alt="Pinned screenshot"
          draggable={false}
        />
      ) : null}
    </main>
  );
}

export default PinApp;
