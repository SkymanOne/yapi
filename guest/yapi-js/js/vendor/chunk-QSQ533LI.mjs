import {
  pollOAuthDeviceCodeFlow
} from "./chunk-ENADOOAY.mjs";
import "./chunk-XGB3TDIC.mjs";

// target/vendor-pi/pi-ai/dist/auth/oauth/meta.js
var CLIENT_ID = "1031625952748946";
var AUTH_HOST = "https://auth.meta.com";
var DEVICE_AUTHORIZATION_URL = `${AUTH_HOST}/oidc/device/authorization/`;
var DEVICE_TOKEN_URL = `${AUTH_HOST}/oidc/device/token/`;
var API_KEY_MINT_URL = "https://api.meta.ai/muse-code/key";
var API_KEY_LIFETIME_MS = 24 * 60 * 60 * 1e3;
var REQUEST_TIMEOUT_MS = 30 * 1e3;
function requestSignal(signal) {
  return AbortSignal.any([AbortSignal.timeout(REQUEST_TIMEOUT_MS), signal]);
}
async function readJson(response) {
  try {
    const json = await response.json();
    return json && typeof json === "object" ? json : null;
  } catch {
    return null;
  }
}
function errorDetail(json) {
  for (const key of ["error_description", "detail", "message", "error"]) {
    const value = json?.[key];
    if (typeof value === "string" && value.trim())
      return `: ${value.trim()}`;
  }
  return "";
}
function trustedHttpUrl(value) {
  if (typeof value !== "string" || !value)
    return null;
  try {
    const url = new URL(value);
    if (url.protocol !== "https:" && url.protocol !== "http:")
      return null;
    return url.href;
  } catch {
    return null;
  }
}
function positiveNumber(value) {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : void 0;
}
async function startDeviceAuthorization(signal) {
  const response = await fetch(DEVICE_AUTHORIZATION_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
      Accept: "application/json"
    },
    body: new URLSearchParams({ client_id: CLIENT_ID }).toString(),
    signal: requestSignal(signal)
  });
  const json = await readJson(response);
  if (!response.ok) {
    throw new Error(`Meta device authorization failed with status ${response.status}${errorDetail(json)}`);
  }
  const deviceCode = json?.device_code;
  const userCode = json?.user_code;
  const verificationUri = trustedHttpUrl(json?.verification_uri_complete) ?? trustedHttpUrl(json?.verification_uri);
  if (typeof deviceCode !== "string" || !deviceCode || typeof userCode !== "string" || !userCode || !verificationUri) {
    throw new Error(`Invalid Meta device authorization response: ${JSON.stringify(json)}`);
  }
  return {
    deviceCode,
    userCode,
    verificationUri,
    intervalSeconds: positiveNumber(json?.interval),
    expiresInSeconds: positiveNumber(json?.expires_in)
  };
}
async function pollForIdentityToken(device, signal) {
  return pollOAuthDeviceCodeFlow({
    intervalSeconds: device.intervalSeconds,
    expiresInSeconds: device.expiresInSeconds,
    waitBeforeFirstPoll: true,
    signal,
    poll: async () => {
      const response = await fetch(DEVICE_TOKEN_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
          Accept: "application/json"
        },
        body: new URLSearchParams({
          grant_type: "urn:ietf:params:oauth:grant-type:device_code",
          device_code: device.deviceCode,
          client_id: CLIENT_ID
        }).toString(),
        signal: requestSignal(signal)
      });
      const json = await readJson(response);
      if (response.ok && typeof json?.access_token === "string" && json.access_token) {
        return { status: "complete", value: json.access_token };
      }
      switch (json?.error) {
        case "authorization_pending":
          return { status: "pending" };
        case "slow_down":
          return { status: "slow_down", intervalSeconds: positiveNumber(json?.interval) };
        case "access_denied":
          return { status: "failed", message: "Meta login was denied." };
        case "expired_token":
          return { status: "failed", message: "Meta device authorization expired. Please restart login." };
        default:
          return {
            status: "failed",
            message: `Meta device token request failed with status ${response.status}${errorDetail(json)}`
          };
      }
    }
  });
}
async function mintApiKey(identityToken, signal) {
  const response = await fetch(API_KEY_MINT_URL, {
    method: "POST",
    headers: {
      Accept: "application/json",
      Authorization: `Bearer ${identityToken}`,
      "Content-Type": "application/json",
      "x-api-version": "1.0.0"
    },
    body: "{}",
    signal: requestSignal(signal)
  });
  const json = await readJson(response);
  if (response.status === 401 || response.status === 403) {
    throw new Error(`Meta session expired (status ${response.status}). Run \`/login meta\` to sign in again.${errorDetail(json)}`);
  }
  if (!response.ok) {
    throw new Error(`Meta API key mint failed with status ${response.status}${errorDetail(json)}`);
  }
  const apiKey = json?.api_key;
  if (typeof apiKey !== "string" || !apiKey) {
    const actionUrl = trustedHttpUrl(json?.action_url);
    throw new Error(`Meta did not issue an API key.${actionUrl ? ` Complete setup at ${actionUrl}` : ""}`);
  }
  return { type: "oauth", refresh: identityToken, access: apiKey, expires: Date.now() + API_KEY_LIFETIME_MS };
}
async function loginMeta(interaction) {
  try {
    const device = await startDeviceAuthorization(interaction.signal);
    interaction.notify({
      type: "device_code",
      userCode: device.userCode,
      verificationUri: device.verificationUri,
      intervalSeconds: device.intervalSeconds,
      expiresInSeconds: device.expiresInSeconds
    });
    const identityToken = await pollForIdentityToken(device, interaction.signal);
    interaction.notify({ type: "progress", message: "Enabling Meta Model API access..." });
    return await mintApiKey(identityToken, interaction.signal);
  } catch (e) {
    if (interaction.signal.aborted)
      throw new Error("Login cancelled");
    throw e;
  }
}
var metaOAuth = {
  name: "Meta (Muse subscription)",
  isSubscription: true,
  loginLabel: "Sign in with Meta",
  login: loginMeta,
  refresh: (credential, signal) => mintApiKey(credential.refresh, signal),
  async toAuth(credential) {
    return { apiKey: credential.access };
  }
};
export {
  metaOAuth
};
