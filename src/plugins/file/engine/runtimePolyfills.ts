declare global {
  interface PromiseConstructor {
    withResolvers<T>(): {
      promise: Promise<T>;
      resolve: (value: T | PromiseLike<T>) => void;
      reject: (reason?: unknown) => void;
    };
  }
}

export function installFileEngineRuntimePolyfills() {
  if (typeof Promise.withResolvers !== "function") {
    Promise.withResolvers = function withResolvers<T>() {
      let resolve!: (value: T | PromiseLike<T>) => void;
      let reject!: (reason?: unknown) => void;
      const promise = new Promise<T>((resolvePromise, rejectPromise) => {
        resolve = resolvePromise;
        reject = rejectPromise;
      });
      return { promise, resolve, reject };
    };
  }

  const urlConstructor = URL as typeof URL & {
    parse?: (value: string, base?: string | URL) => URL | null;
  };
  if (typeof urlConstructor.parse !== "function") {
    urlConstructor.parse = (value, base) => {
      try {
        return new URL(value, base);
      } catch {
        return null;
      }
    };
  }

  installReadableStreamAsyncIterator();
}

interface FileEngineReadableStreamIterator<T> extends AsyncIterableIterator<T> {
  return(): Promise<IteratorResult<T>>;
}

type FileEngineReadableStreamPrototype = ReadableStream<unknown> & {
  values?: <T>(
    this: ReadableStream<T>,
    options?: { preventCancel?: boolean },
  ) => FileEngineReadableStreamIterator<T>;
  [Symbol.asyncIterator]?: <T>(
    this: ReadableStream<T>,
  ) => FileEngineReadableStreamIterator<T>;
};

function installReadableStreamAsyncIterator() {
  if (typeof ReadableStream === "undefined") return;
  const prototype = ReadableStream.prototype as FileEngineReadableStreamPrototype;
  if (typeof prototype.values !== "function") {
    Object.defineProperty(prototype, "values", {
      configurable: true,
      writable: true,
      value: function values<T>(
        this: ReadableStream<T>,
        options?: { preventCancel?: boolean },
      ): FileEngineReadableStreamIterator<T> {
        const reader = this.getReader();
        let finished = false;
        const release = () => {
          if (!finished) {
            finished = true;
            reader.releaseLock();
          }
        };
        return {
          async next() {
            const result = await reader.read();
            if (result.done) {
              release();
              return { done: true, value: undefined };
            }
            return { done: false, value: result.value };
          },
          async return() {
            if (!finished) {
              if (!options?.preventCancel) await reader.cancel();
              release();
            }
            return { done: true, value: undefined };
          },
          [Symbol.asyncIterator]() {
            return this;
          },
        };
      },
    });
  }
  if (typeof prototype[Symbol.asyncIterator] !== "function") {
    Object.defineProperty(prototype, Symbol.asyncIterator, {
      configurable: true,
      writable: true,
      value: function asyncIterator<T>(this: ReadableStream<T>) {
        return (ReadableStream.prototype as FileEngineReadableStreamPrototype).values!.call(this);
      },
    });
  }
}
