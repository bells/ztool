if (typeof Promise.withResolvers !== "function") {
  Promise.withResolvers = function withResolvers() {
    let resolve;
    let reject;
    const promise = new Promise((resolvePromise, rejectPromise) => {
      resolve = resolvePromise;
      reject = rejectPromise;
    });
    return { promise, resolve, reject };
  };
}

if (typeof URL.parse !== "function") {
  URL.parse = function parse(value, base) {
    try {
      return new URL(value, base);
    } catch {
      return null;
    }
  };
}

await import("./pdf.worker-core.min.mjs");
