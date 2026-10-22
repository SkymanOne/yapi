import {
  oauthErrorHtml,
  oauthSuccessHtml
} from "./chunk-WPQGAY53.mjs";

// target/vendor-pi/pi-ai/dist/auth/oauth/callback-server.js
import { createServer } from "node:http";
function sendPage(response, status, html) {
  response.writeHead(status, { "content-type": "text/html; charset=utf-8", "cache-control": "no-store" });
  response.end(html);
}
async function startOAuthCallbackServer(options) {
  const { providerName, signal } = options;
  if (signal?.aborted)
    throw new Error("Login cancelled");
  let resolveWait = () => {
  };
  let rejectWait = () => {
  };
  const waitPromise = new Promise((resolve, reject) => {
    resolveWait = resolve;
    rejectWait = reject;
  });
  waitPromise.catch(() => void 0);
  let claimed = false;
  let settled = false;
  let timer;
  const onAbort = () => finish({ error: new Error("Login cancelled") });
  const finish = (result) => {
    if (settled)
      return;
    settled = true;
    if (timer)
      clearTimeout(timer);
    signal?.removeEventListener("abort", onAbort);
    if ("error" in result)
      rejectWait(result.error);
    else
      resolveWait(result.value);
  };
  const server = createServer((request, response) => {
    void (async () => {
      const url = new URL(request.url ?? "/", "http://localhost");
      if (request.method !== "GET" || url.pathname !== options.path) {
        sendPage(response, 404, oauthErrorHtml("Callback route not found."));
        return;
      }
      if (options.state !== void 0 && url.searchParams.get("state") !== options.state) {
        sendPage(response, 400, oauthErrorHtml("State mismatch."));
        return;
      }
      if (claimed || settled) {
        sendPage(response, 409, oauthErrorHtml("This sign-in has already been handled."));
        return;
      }
      const error = url.searchParams.get("error");
      if (error) {
        const description = url.searchParams.get("error_description") ?? error;
        sendPage(response, 400, oauthErrorHtml(`${providerName} authorization failed.`, description));
        finish({ error: new Error(`${providerName} authorization failed: ${description}`) });
        return;
      }
      const code = url.searchParams.get("code");
      if (!code) {
        sendPage(response, 400, oauthErrorHtml("Missing authorization code."));
        return;
      }
      claimed = true;
      try {
        const value = await options.complete(code);
        sendPage(response, 200, oauthSuccessHtml(`Signed in to ${providerName}. You may now close this page.`));
        finish({ value });
      } catch (error2) {
        const failure = error2 instanceof Error ? error2 : new Error(String(error2));
        sendPage(response, 502, oauthErrorHtml(`${providerName} sign-in failed.`, failure.message));
        finish({ error: failure });
      }
    })();
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(options.port, options.host, () => {
      server.off("error", reject);
      resolve();
    });
  });
  const address = server.address();
  if (!address || typeof address === "string") {
    server.close();
    throw new Error("OAuth callback server did not bind to TCP");
  }
  server.on("error", (error) => finish({ error }));
  signal?.addEventListener("abort", onAbort, { once: true });
  if (options.timeoutMs !== void 0) {
    timer = setTimeout(() => finish({ error: new Error(`${providerName} sign-in timed out`) }), options.timeoutMs);
  }
  const redirectHost = options.redirectHost ?? options.host;
  return {
    redirectUri: `http://${redirectHost.includes(":") ? `[${redirectHost}]` : redirectHost}:${address.port}${options.path}`,
    wait: () => waitPromise,
    cancel: () => {
      if (!claimed)
        finish({ value: void 0 });
    },
    close: () => {
      finish({ error: new Error("OAuth callback server closed") });
      server.close();
    }
  };
}
async function waitForCallbackOrManualInput(interaction, callback, prompt) {
  const manualAbort = new AbortController();
  let manualError;
  const manual = interaction.prompt({ type: "manual_code", ...prompt, signal: manualAbort.signal }).then((input) => {
    callback?.cancel();
    return input;
  }).catch((error) => {
    manualError = error instanceof Error ? error : new Error(String(error));
    callback?.cancel();
    return void 0;
  });
  try {
    const value = await callback?.wait();
    if (manualError)
      throw manualError;
    if (value !== void 0)
      return { type: "callback", value };
    const input = await manual;
    if (manualError)
      throw manualError;
    return { type: "manual", input: input ?? "" };
  } finally {
    manualAbort.abort();
  }
}

export {
  startOAuthCallbackServer,
  waitForCallbackOrManualInput
};
