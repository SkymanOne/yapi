// target/vendor-pi/pi-ai/dist/auth/oauth/device-code.js
var CANCEL_MESSAGE = "Login cancelled";
var TIMEOUT_MESSAGE = "Device flow timed out";
var SLOW_DOWN_TIMEOUT_MESSAGE = "Device flow timed out after one or more slow_down responses. This is often caused by clock drift in WSL or VM environments. Please sync or restart the VM clock and try again.";
var MINIMUM_INTERVAL_MS = 1e3;
var DEFAULT_POLL_INTERVAL_SECONDS = 5;
var SLOW_DOWN_INTERVAL_INCREMENT_MS = 5e3;
function abortableSleep(ms, signal, cancelMessage) {
  return new Promise((resolve, reject) => {
    if (signal.aborted) {
      reject(new Error(cancelMessage));
      return;
    }
    const onAbort = () => {
      clearTimeout(timeout);
      reject(new Error(cancelMessage));
    };
    const timeout = setTimeout(() => {
      signal.removeEventListener("abort", onAbort);
      resolve();
    }, ms);
    signal.addEventListener("abort", onAbort, { once: true });
  });
}
async function pollOAuthDeviceCodeFlow(options) {
  const deadline = typeof options.expiresInSeconds === "number" ? Date.now() + options.expiresInSeconds * 1e3 : Number.POSITIVE_INFINITY;
  let intervalMs = Math.max(MINIMUM_INTERVAL_MS, Math.floor((options.intervalSeconds ?? DEFAULT_POLL_INTERVAL_SECONDS) * 1e3));
  let slowDownResponses = 0;
  if (options.waitBeforeFirstPoll) {
    const remainingMs = deadline - Date.now();
    if (remainingMs > 0) {
      await abortableSleep(Math.min(intervalMs, remainingMs), options.signal, CANCEL_MESSAGE);
    }
  }
  while (Date.now() < deadline) {
    if (options.signal.aborted) {
      throw new Error(CANCEL_MESSAGE);
    }
    const result = await options.poll();
    if (result.status === "complete") {
      return result.value;
    }
    if (result.status === "failed") {
      throw new Error(result.message);
    }
    if (result.status === "slow_down") {
      slowDownResponses += 1;
      intervalMs = typeof result.intervalSeconds === "number" && Number.isFinite(result.intervalSeconds) && result.intervalSeconds > 0 ? Math.max(MINIMUM_INTERVAL_MS, Math.floor(result.intervalSeconds * 1e3)) : Math.max(MINIMUM_INTERVAL_MS, intervalMs + SLOW_DOWN_INTERVAL_INCREMENT_MS);
    }
    const remainingMs = deadline - Date.now();
    if (remainingMs <= 0) {
      break;
    }
    await abortableSleep(Math.min(intervalMs, remainingMs), options.signal, CANCEL_MESSAGE);
  }
  throw new Error(slowDownResponses > 0 ? SLOW_DOWN_TIMEOUT_MESSAGE : TIMEOUT_MESSAGE);
}

export {
  pollOAuthDeviceCodeFlow
};
