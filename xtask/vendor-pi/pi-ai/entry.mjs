// The parts of pi-ai that yapi's facade, `guest/yapi-js/js/pi/ai.mjs`,
// re-exports, bundled from a copy of pi-ai's `dist` in which the wire API
// modules stream through yapi's host and the generated catalogs read yapi's
// (see `xtask/src/vendor_pi.rs`).
import { setBedrockProviderModule } from "./dist/api/bedrock-converse-stream.lazy.js";
import * as bedrock from "./dist/api/bedrock-converse-stream.js";
import { registerBundledOAuthFlowLoaders } from "./dist/auth/oauth/load.js";

export * from "./dist/api/lazy.js";
export * from "./dist/auth/context.js";
export * from "./dist/auth/credential-store.js";
export * from "./dist/auth/helpers.js";
export * from "./dist/env-api-keys.js";
export * from "./dist/legacy-api-aliases.js";
export * from "./dist/image-models.js";
export * from "./dist/images.js";
export * from "./dist/images-api-registry.js";
export * from "./dist/models.js";
export * from "./dist/models-store.js";
export * from "./dist/providers/images/register-builtins.js";
export * from "./dist/session-resources.js";
export * from "./dist/utils/abort.js";
export * from "./dist/utils/abort-signals.js";
export * from "./dist/utils/assistant-message-frame.js";
export * from "./dist/utils/diagnostics.js";
export * from "./dist/utils/error-body.js";
export * from "./dist/utils/estimate.js";
export * from "./dist/utils/event-stream.js";
export * from "./dist/utils/hash.js";
export * from "./dist/utils/headers.js";
export * from "./dist/utils/model-operations.js";
export * from "./dist/utils/node-http-proxy.js";
export * from "./dist/utils/oauth-page.js";
export * from "./dist/utils/overflow.js";
export * from "./dist/utils/pi-user-agent.js";
export * from "./dist/utils/provider-env.js";
export * from "./dist/utils/provider-retry.js";
export * from "./dist/utils/retry.js";
export * from "./dist/utils/sanitize-unicode.js";
export * from "./dist/utils/sleep.js";
export * from "./dist/utils/text.js";
export * from "./dist/utils/transcript.js";
export * from "./dist/utils/typebox-helpers.js";
export * from "./dist/utils/uuid.js";

export * from "./dist/api/anthropic-messages.lazy.js";
export * from "./dist/api/azure-openai-responses.lazy.js";
export * from "./dist/api/bedrock-converse-stream.lazy.js";
export * from "./dist/api/cloudflare.js";
export * from "./dist/api/cloudflare-ai-binding.js";
export * from "./dist/api/cloudflare-workers-ai-system-one.lazy.js";
export * from "./dist/api/constrained-sampling.js";
export * from "./dist/api/github-copilot-headers.js";
export * from "./dist/api/google-generative-ai.lazy.js";
export * from "./dist/api/google-shared.js";
export * from "./dist/api/google-vertex.lazy.js";
export * from "./dist/api/llama-cpp-classify.js";
export * from "./dist/api/llama-cpp-classify.lazy.js";
export * from "./dist/api/mistral-conversations.lazy.js";
export * from "./dist/api/openai-codex-responses.lazy.js";
export * from "./dist/api/openai-completions.lazy.js";
export * from "./dist/api/openai-prompt-cache.js";
export * from "./dist/api/openai-responses.lazy.js";
export * from "./dist/api/openai-responses-shared.js";
export * from "./dist/api/openrouter-images.lazy.js";
export * from "./dist/api/pi-messages.js";
export * from "./dist/api/pi-messages.lazy.js";
export * from "./dist/api/simple-options.js";
export * from "./dist/api/system-one-shared.js";
export * from "./dist/api/transform-messages.js";
export * from "./dist/api/typesafe-system-one.lazy.js";

// Every module of `dist/providers`, listed by `cargo xtask vendor-pi`.
export * from "./providers.mjs";

// pi's Bun build registers its Node-only modules the same way, as bundlers
// cannot follow pi-ai's variable import specifiers. Flows load on first use.
setBedrockProviderModule(bedrock);
registerBundledOAuthFlowLoaders({
	anthropic: async () => (await import("./dist/auth/oauth/anthropic.js")).anthropicOAuth,
	openaiCodex: async () => (await import("./dist/auth/oauth/openai-codex.js")).openaiCodexOAuth,
	openaiChatGPT: async () => (await import("./dist/auth/oauth/openai-chatgpt.js")).openaiChatGPTOAuth,
	githubCopilot: async () => (await import("./dist/auth/oauth/github-copilot.js")).githubCopilotOAuth,
	openrouter: async () => (await import("./dist/auth/oauth/openrouter.js")).openRouterOAuth,
	kimiCoding: async () => (await import("./dist/auth/oauth/kimi-coding.js")).kimiCodingOAuth,
	meta: async () => (await import("./dist/auth/oauth/meta.js")).metaOAuth,
	xai: async () => (await import("./dist/auth/oauth/xai.js")).xaiOAuth,
	radius: async (options) => (await import("./dist/auth/oauth/radius.js")).createRadiusOAuth(options),
});
