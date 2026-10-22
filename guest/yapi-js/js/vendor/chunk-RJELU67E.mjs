import {
  startOAuthCallbackServer,
  waitForCallbackOrManualInput
} from "./chunk-32IVTROX.mjs";
import {
  generatePKCE
} from "./chunk-WDUQRV2C.mjs";
import "./chunk-WPQGAY53.mjs";
import {
  getProviderEnvValue
} from "./chunk-EVPRFFV6.mjs";
import "./chunk-XGB3TDIC.mjs";

// target/vendor-pi/pi-ai/dist/auth/oauth/openrouter.js
var AUTHORIZE_URL = "https://openrouter.ai/auth";
var TOKEN_URL = "https://openrouter.ai/api/v1/auth/keys";
var LOGIN_TIMEOUT_MS = 5 * 60 * 1e3;
var TOKEN_EXCHANGE_TIMEOUT_MS = 3e4;
function getCallbackHost() {
  return getProviderEnvValue("PI_OAUTH_CALLBACK_HOST") || "127.0.0.1";
}
function parseAuthorizationInput(input) {
  const value = input.trim();
  if (!value)
    return void 0;
  try {
    return new URL(value).searchParams.get("code") ?? void 0;
  } catch {
  }
  if (value.includes("code=")) {
    return new URLSearchParams(value).get("code") ?? void 0;
  }
  return value;
}
function errorDetail(body) {
  if (typeof body.error_description === "string")
    return body.error_description;
  if (typeof body.message === "string")
    return body.message;
  if (typeof body.error === "string")
    return body.error;
  if (body.error && typeof body.error === "object" && !Array.isArray(body.error)) {
    const message = body.error.message;
    if (typeof message === "string")
      return message;
  }
  return void 0;
}
async function exchangeAuthorizationCode(code, verifier, signal) {
  if (signal.aborted)
    throw new Error("Login cancelled");
  const controller = new AbortController();
  const onAbort = () => controller.abort(signal.reason);
  signal.addEventListener("abort", onAbort, { once: true });
  const timeout = setTimeout(() => controller.abort(new Error("OpenRouter OAuth token exchange timed out")), TOKEN_EXCHANGE_TIMEOUT_MS);
  let response;
  let body = {};
  try {
    response = await fetch(TOKEN_URL, {
      method: "POST",
      headers: { accept: "application/json", "content-type": "application/json" },
      body: JSON.stringify({ code, code_verifier: verifier, code_challenge_method: "S256" }),
      signal: controller.signal
    });
    try {
      const parsed = await response.json();
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed))
        body = parsed;
    } catch {
      if (response.ok)
        throw new Error("OpenRouter OAuth returned invalid JSON");
    }
  } catch (error) {
    if (signal.aborted)
      throw new Error("Login cancelled");
    if (controller.signal.aborted)
      throw new Error("OpenRouter OAuth token exchange timed out");
    throw error;
  } finally {
    clearTimeout(timeout);
    signal.removeEventListener("abort", onAbort);
  }
  if (!response.ok) {
    const detail = errorDetail(body);
    throw new Error(`OpenRouter OAuth key exchange failed (HTTP ${response.status})${detail ? `: ${detail}` : ""}`);
  }
  if (typeof body.key !== "string" || body.key.length === 0) {
    throw new Error('OpenRouter OAuth response carries no "key"');
  }
  return {
    type: "oauth",
    access: body.key,
    refresh: "",
    expires: Number.MAX_SAFE_INTEGER
  };
}
async function loginOpenRouter(interaction) {
  const { verifier, challenge } = await generatePKCE();
  const callback = await startOAuthCallbackServer({
    providerName: "OpenRouter",
    host: getCallbackHost(),
    port: 0,
    path: `/oauth/callback/${crypto.randomUUID()}`,
    complete: (code) => exchangeAuthorizationCode(code, verifier, interaction.signal),
    signal: interaction.signal,
    timeoutMs: LOGIN_TIMEOUT_MS
  });
  try {
    const authorizeUrl = new URL(AUTHORIZE_URL);
    authorizeUrl.search = new URLSearchParams({
      callback_url: callback.redirectUri,
      code_challenge: challenge,
      code_challenge_method: "S256"
    }).toString();
    interaction.notify({
      type: "progress",
      message: `Listening for OpenRouter OAuth callback on ${callback.redirectUri}`
    });
    interaction.notify({
      type: "auth_url",
      url: authorizeUrl.toString(),
      instructions: "Complete sign-in in your browser. If the browser is on another machine, paste the final redirect URL here."
    });
    const result = await waitForCallbackOrManualInput(interaction, callback, {
      message: "Complete sign-in in your browser, or paste the authorization code / redirect URL here:",
      placeholder: callback.redirectUri
    });
    if (result.type === "callback")
      return result.value;
    const code = parseAuthorizationInput(result.input);
    if (!code)
      throw new Error("Missing authorization code");
    interaction.notify({ type: "progress", message: "Exchanging authorization code for an API key..." });
    return await exchangeAuthorizationCode(code, verifier, interaction.signal);
  } finally {
    callback.close();
  }
}
var openRouterOAuth = {
  name: "OpenRouter OAuth",
  loginLabel: "Sign in with OpenRouter",
  login: loginOpenRouter,
  async refresh(credential, _signal) {
    return credential;
  },
  async toAuth(credential) {
    return { apiKey: credential.access };
  }
};
export {
  openRouterOAuth
};
