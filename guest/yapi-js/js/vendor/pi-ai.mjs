import {
  DEFAULT_RADIUS_GATEWAY,
  getRadiusCredentialConfig,
  getRadiusModels,
  getRadiusModelsFromConfig,
  loadRadiusGatewayConfig,
  normalizeRadiusGatewayUrl
} from "./chunk-ZVMXFEG4.mjs";
import {
  answerFromProbabilities,
  classify,
  labelProbabilities,
  llamaServerRoot,
  peakConfidence,
  renderQuestion
} from "./chunk-JCNKJFKF.mjs";
import {
  PiMessagesResponseError,
  stream,
  streamSimple
} from "./chunk-PTYLW4C7.mjs";
import {
  GITHUB_COPILOT_CLASSIFIER_MODELS,
  GITHUB_COPILOT_IMAGE_MODELS,
  GITHUB_COPILOT_MODELS,
  flattenChatModelCatalog,
  flattenClassifierModelCatalog,
  flattenImageModelCatalog
} from "./chunk-YARZUAXD.mjs";
import {
  sleep
} from "./chunk-K7XUP4LN.mjs";
import {
  oauthErrorHtml,
  oauthSuccessHtml
} from "./chunk-WPQGAY53.mjs";
import {
  typebox_exports
} from "./chunk-XSZB7GXF.mjs";
import "./chunk-HSYWYWI2.mjs";
import {
  DEFAULT_THINKING_BUDGETS,
  MIN_ANSWER_TOKENS,
  OPENAI_PROMPT_CACHE_KEY_MAX_LENGTH,
  adjustMaxTokensForThinking,
  appendGrammarToolInputJsonDelta,
  buildBaseOptions,
  buildCopilotDynamicHeaders,
  calculateContextTokens,
  clampMaxTokensToContext,
  clampOpenAIPromptCacheKey,
  clampReasoning,
  clampThinkingBudgetToAnswerRoom,
  createGrammarToolInputProperties,
  estimateContextTokens,
  estimateMessageTokens,
  estimateTextAndImageContentTokens,
  estimateTextTokens,
  getGrammarToolInput,
  getJsonSchemaToolParameters,
  getPiUserAgent,
  hasCopilotVisionInput,
  inferCopilotInitiator,
  makeStrictJsonSchema,
  resolveGrammarConstrainedSampling,
  resolveJsonSchemaStrictSampling,
  sanitizeSurrogates,
  shortHash,
  thinkingBudgetForLevel,
  transformMessages
} from "./chunk-6PTCZY6C.mjs";
import {
  parseStreamingJson
} from "./chunk-SOWCS55L.mjs";
import {
  getProviderEnvValue
} from "./chunk-EVPRFFV6.mjs";
import {
  classifySystemOne,
  isRecord
} from "./chunk-6BSHW5CE.mjs";
import {
  InMemoryCredentialStore,
  InMemoryModelsStore,
  ModelsError,
  assertChatModel,
  assertClassifierModel,
  assertImageModel,
  calculateCost,
  clampThinkingLevel,
  classifierErrorResult,
  collapseSystemMessages,
  contentText,
  createInitialSystemMessage,
  createModels,
  createProvider,
  declarationsEqual,
  defaultProviderAuthContext,
  getCurrentSystemMessage,
  getCurrentSystemPrompt,
  getCurrentTools,
  getDeclaredTools,
  getInitialSystemMessage,
  getModelType,
  getSupportedThinkingLevels,
  getSystemMessageText,
  getToolStateChanges,
  hasApi,
  hasNonAdditiveToolChanges,
  hasToolRedefinitions,
  imageErrorResult,
  isModelType,
  lazyApi,
  lazyStream,
  modelsAreEqual,
  normalizeContext,
  operationSignal,
  raceWithAbortSignal,
  renderSystemMessageUpdate,
  resolveTranscript,
  resolveTranscriptTools,
  toToolDeclaration,
  withoutInitialSystemMessage
} from "./chunk-ICPUEOJC.mjs";
import {
  MAX_PROVIDER_ERROR_BODY_CHARS,
  formatProviderError,
  normalizeProviderError,
  retryProviderRequest,
  safeJsonStringify,
  truncateErrorText
} from "./chunk-TRGJFRGE.mjs";
import {
  AssistantMessageEventStream,
  EventStream,
  appendAssistantMessageDiagnostic,
  createAssistantMessageDiagnostic,
  createAssistantMessageEventStream,
  extractDiagnosticError,
  formatThrownValue
} from "./chunk-VLRLU4IK.mjs";
import {
  headersToRecord,
  providerHeadersToRecord
} from "./chunk-6WY5DPXF.mjs";
import {
  __export
} from "./chunk-XGB3TDIC.mjs";

// target/vendor-pi/pi-ai/dist/api/bedrock-converse-stream.lazy.js
var __rewriteRelativeImportExtension = function(path, preserveJsx) {
  if (typeof path === "string" && /^\.\.?\//.test(path)) {
    return path.replace(/\.(tsx)$|((?:\.d)?)((?:\.[^./]+?)?)\.([cm]?)ts$/i, function(m, tsx, d, ext, cm) {
      return tsx ? preserveJsx ? ".jsx" : ".js" : d && (!ext || !cm) ? m : d + ext + "." + cm.toLowerCase() + "js";
    });
  }
  return path;
};
var importNodeOnlyApi = (specifier) => {
  const runtimeSpecifier = import.meta.url.endsWith(".js") ? specifier.replace(/\.ts$/, ".js") : specifier;
  return import(__rewriteRelativeImportExtension(runtimeSpecifier));
};
var bedrockModuleOverride;
function setBedrockProviderModule(module) {
  bedrockModuleOverride = module;
}
var bedrockConverseStreamApi = () => lazyApi(async () => bedrockModuleOverride ?? await importNodeOnlyApi("./bedrock-converse-stream.ts"));

// target/vendor-pi/pi-ai/dist/api/bedrock-converse-stream.js
var bedrock_converse_stream_exports = {};
__export(bedrock_converse_stream_exports, {
  stream: () => stream2,
  streamSimple: () => streamSimple2
});
var host = () => globalThis.__yapi.hostApi("bedrock-converse-stream");
var stream2 = (model, context, options) => host().stream(model, context, options);
var streamSimple2 = (model, context, options) => host().streamSimple(model, context, options);

// target/vendor-pi/pi-ai/dist/auth/oauth/load.js
var __rewriteRelativeImportExtension2 = function(path, preserveJsx) {
  if (typeof path === "string" && /^\.\.?\//.test(path)) {
    return path.replace(/\.(tsx)$|((?:\.d)?)((?:\.[^./]+?)?)\.([cm]?)ts$/i, function(m, tsx, d, ext, cm) {
      return tsx ? preserveJsx ? ".jsx" : ".js" : d && (!ext || !cm) ? m : d + ext + "." + cm.toLowerCase() + "js";
    });
  }
  return path;
};
var importOAuthModule = (specifier) => {
  const runtimeSpecifier = import.meta.url.endsWith(".js") ? specifier.replace(/\.ts$/, ".js") : specifier;
  return import(__rewriteRelativeImportExtension2(runtimeSpecifier));
};
var bundledLoaders;
function registerBundledOAuthFlowLoaders(loaders) {
  bundledLoaders = loaders;
}
var loadAnthropicOAuth = async () => {
  if (bundledLoaders)
    return bundledLoaders.anthropic();
  return (await importOAuthModule("./anthropic.ts")).anthropicOAuth;
};
var loadOpenAICodexOAuth = async () => {
  if (bundledLoaders)
    return bundledLoaders.openaiCodex();
  return (await importOAuthModule("./openai-codex.ts")).openaiCodexOAuth;
};
var loadOpenAIChatGPTOAuth = async () => {
  if (bundledLoaders)
    return bundledLoaders.openaiChatGPT();
  return (await importOAuthModule("./openai-chatgpt.ts")).openaiChatGPTOAuth;
};
var loadGitHubCopilotOAuth = async () => {
  if (bundledLoaders)
    return bundledLoaders.githubCopilot();
  return (await importOAuthModule("./github-copilot.ts")).githubCopilotOAuth;
};
var loadOpenRouterOAuth = async () => {
  if (bundledLoaders)
    return bundledLoaders.openrouter();
  return (await importOAuthModule("./openrouter.ts")).openRouterOAuth;
};
var loadKimiCodingOAuth = async () => {
  if (bundledLoaders)
    return bundledLoaders.kimiCoding();
  return (await importOAuthModule("./kimi-coding.ts")).kimiCodingOAuth;
};
var loadMetaOAuth = async () => {
  if (bundledLoaders)
    return bundledLoaders.meta();
  return (await importOAuthModule("./meta.ts")).metaOAuth;
};
var loadXaiOAuth = async () => {
  if (bundledLoaders)
    return bundledLoaders.xai();
  return (await importOAuthModule("./xai.ts")).xaiOAuth;
};
var loadRadiusOAuth = async (options) => {
  if (bundledLoaders)
    return bundledLoaders.radius(options);
  return (await importOAuthModule("./radius.ts")).createRadiusOAuth(options);
};

// target/vendor-pi/pi-ai/dist/auth/helpers.js
function envApiKeyAuth(name, envVars) {
  return {
    name,
    login: async (interaction) => {
      interaction.signal.throwIfAborted();
      const key = await interaction.prompt({ type: "secret", message: `Enter ${name}` });
      interaction.signal.throwIfAborted();
      return { type: "api_key", key };
    },
    resolve: async ({ ctx, credential, signal }) => {
      signal.throwIfAborted();
      if (credential?.key) {
        return { auth: { apiKey: credential.key }, env: credential.env, source: "stored credential" };
      }
      for (const envVar of envVars) {
        const value = await ctx.env(envVar);
        signal.throwIfAborted();
        if (value)
          return { auth: { apiKey: value }, source: envVar };
      }
      return void 0;
    }
  };
}
function lazyOAuth(input) {
  let promise;
  const loaded = () => {
    promise ??= input.load();
    return promise;
  };
  return {
    name: input.name,
    isSubscription: input.isSubscription,
    loginLabel: input.loginLabel,
    login: async (interaction, options) => (await loaded()).login(interaction, options),
    refresh: async (credential, signal) => (await loaded()).refresh(credential, signal),
    toAuth: async (credential) => (await loaded()).toAuth(credential)
  };
}

// target/vendor-pi/pi-ai/dist/env-api-keys.js
var __rewriteRelativeImportExtension3 = function(path, preserveJsx) {
  if (typeof path === "string" && /^\.\.?\//.test(path)) {
    return path.replace(/\.(tsx)$|((?:\.d)?)((?:\.[^./]+?)?)\.([cm]?)ts$/i, function(m, tsx, d, ext, cm) {
      return tsx ? preserveJsx ? ".jsx" : ".js" : d && (!ext || !cm) ? m : d + ext + "." + cm.toLowerCase() + "js";
    });
  }
  return path;
};
var _existsSync = null;
var _homedir = null;
var _join = null;
var dynamicImport = (specifier) => import(__rewriteRelativeImportExtension3(specifier));
var NODE_FS_SPECIFIER = "node:fs";
var NODE_OS_SPECIFIER = "node:os";
var NODE_PATH_SPECIFIER = "node:path";
if (typeof process !== "undefined" && (process.versions?.node || process.versions?.bun)) {
  dynamicImport(NODE_FS_SPECIFIER).then((m) => {
    _existsSync = m.existsSync;
  });
  dynamicImport(NODE_OS_SPECIFIER).then((m) => {
    _homedir = m.homedir;
  });
  dynamicImport(NODE_PATH_SPECIFIER).then((m) => {
    _join = m.join;
  });
}
var ANTHROPIC_AUTH_TOKEN_ENV = "ANTHROPIC_AUTH_TOKEN";
var ANTHROPIC_OAUTH_TOKEN_ENV = "ANTHROPIC_OAUTH_TOKEN";
var ANTHROPIC_API_KEY_ENV = "ANTHROPIC_API_KEY";
var ANTHROPIC_FEDERATION_RULE_ID_ENV = "ANTHROPIC_FEDERATION_RULE_ID";
var ANTHROPIC_ORGANIZATION_ID_ENV = "ANTHROPIC_ORGANIZATION_ID";
var ANTHROPIC_SERVICE_ACCOUNT_ID_ENV = "ANTHROPIC_SERVICE_ACCOUNT_ID";
var ANTHROPIC_IDENTITY_TOKEN_FILE_ENV = "ANTHROPIC_IDENTITY_TOKEN_FILE";
var ANTHROPIC_WORKSPACE_ID_ENV = "ANTHROPIC_WORKSPACE_ID";
var cachedVertexAdcCredentialsExists = null;
function hasVertexAdcCredentials(env) {
  const explicitCredentialsPath = env?.GOOGLE_APPLICATION_CREDENTIALS;
  if (explicitCredentialsPath) {
    return _existsSync ? _existsSync(explicitCredentialsPath) : false;
  }
  if (cachedVertexAdcCredentialsExists === null) {
    if (!_existsSync || !_homedir || !_join) {
      const isNode = typeof process !== "undefined" && (process.versions?.node || process.versions?.bun);
      if (!isNode) {
        cachedVertexAdcCredentialsExists = false;
      }
      return false;
    }
    const gacPath = getProviderEnvValue("GOOGLE_APPLICATION_CREDENTIALS", env);
    if (gacPath) {
      cachedVertexAdcCredentialsExists = _existsSync(gacPath);
    } else {
      cachedVertexAdcCredentialsExists = _existsSync(_join(_homedir(), ".config", "gcloud", "application_default_credentials.json"));
    }
  }
  return cachedVertexAdcCredentialsExists;
}
function getApiKeyEnvVars(provider) {
  if (provider === "github-copilot") {
    return ["COPILOT_GITHUB_TOKEN"];
  }
  if (provider === "anthropic") {
    return [ANTHROPIC_AUTH_TOKEN_ENV, ANTHROPIC_OAUTH_TOKEN_ENV, ANTHROPIC_API_KEY_ENV];
  }
  const envMap = {
    "ant-ling": "ANT_LING_API_KEY",
    "qwen-token-plan": "QWEN_TOKEN_PLAN_API_KEY",
    "qwen-token-plan-cn": "QWEN_TOKEN_PLAN_CN_API_KEY",
    "qwen-token-plan-individual": "QWEN_TOKEN_PLAN_API_KEY",
    openai: "OPENAI_API_KEY",
    "azure-openai-responses": "AZURE_OPENAI_API_KEY",
    nvidia: "NVIDIA_API_KEY",
    deepseek: "DEEPSEEK_API_KEY",
    google: "GEMINI_API_KEY",
    "google-vertex": "GOOGLE_CLOUD_API_KEY",
    groq: "GROQ_API_KEY",
    cerebras: "CEREBRAS_API_KEY",
    xai: "XAI_API_KEY",
    typesafe: "TYPESAFE_API_KEY",
    radius: "RADIUS_API_KEY",
    openrouter: "OPENROUTER_API_KEY",
    "vercel-ai-gateway": "AI_GATEWAY_API_KEY",
    zai: "ZAI_API_KEY",
    "zai-coding-cn": "ZAI_CODING_CN_API_KEY",
    mistral: "MISTRAL_API_KEY",
    minimax: "MINIMAX_API_KEY",
    "minimax-cn": "MINIMAX_CN_API_KEY",
    moonshotai: "MOONSHOT_API_KEY",
    "moonshotai-cn": "MOONSHOT_API_KEY",
    huggingface: "HF_TOKEN",
    fireworks: "FIREWORKS_API_KEY",
    together: "TOGETHER_API_KEY",
    baseten: "BASETEN_API_KEY",
    opencode: "OPENCODE_API_KEY",
    "opencode-go": "OPENCODE_API_KEY",
    "kimi-coding": "KIMI_API_KEY",
    meta: "META_API_KEY",
    "cloudflare-workers-ai": "CLOUDFLARE_API_KEY",
    "cloudflare-ai-gateway": "CLOUDFLARE_API_KEY",
    xiaomi: "XIAOMI_API_KEY",
    "xiaomi-token-plan-cn": "XIAOMI_TOKEN_PLAN_CN_API_KEY",
    "xiaomi-token-plan-ams": "XIAOMI_TOKEN_PLAN_AMS_API_KEY",
    "xiaomi-token-plan-sgp": "XIAOMI_TOKEN_PLAN_SGP_API_KEY"
  };
  const envVar = envMap[provider];
  return envVar ? [envVar] : void 0;
}
function findEnvKeys(provider, env) {
  const envVars = getApiKeyEnvVars(provider);
  if (!envVars)
    return void 0;
  const found = envVars.filter((envVar) => !!getProviderEnvValue(envVar, env));
  return found.length > 0 ? found : void 0;
}
function getEnvApiKey(provider, env) {
  const envKeys = findEnvKeys(provider, env);
  if (envKeys?.[0]) {
    const apiKeyEnv = provider === "anthropic" ? envKeys.find((key) => key !== ANTHROPIC_AUTH_TOKEN_ENV) : envKeys[0];
    if (apiKeyEnv)
      return getProviderEnvValue(apiKeyEnv, env);
  }
  if (provider === "google-vertex") {
    const hasCredentials = hasVertexAdcCredentials(env);
    const hasProject = !!(getProviderEnvValue("GOOGLE_CLOUD_PROJECT", env) || getProviderEnvValue("GCLOUD_PROJECT", env));
    const hasLocation = !!getProviderEnvValue("GOOGLE_CLOUD_LOCATION", env);
    if (hasCredentials && hasProject && hasLocation) {
      return "<authenticated>";
    }
  }
  if (provider === "amazon-bedrock") {
    if (getProviderEnvValue("AWS_PROFILE", env) || getProviderEnvValue("AWS_ACCESS_KEY_ID", env) && getProviderEnvValue("AWS_SECRET_ACCESS_KEY", env) || getProviderEnvValue("AWS_BEARER_TOKEN_BEDROCK", env) || getProviderEnvValue("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI", env) || getProviderEnvValue("AWS_CONTAINER_CREDENTIALS_FULL_URI", env) || getProviderEnvValue("AWS_WEB_IDENTITY_TOKEN_FILE", env)) {
      return "<authenticated>";
    }
  }
  return void 0;
}

// target/vendor-pi/pi-ai/dist/api/anthropic-messages.lazy.js
var anthropicMessagesApi = () => lazyApi(() => import("./chunk-PIESELT4.mjs"));

// target/vendor-pi/pi-ai/dist/api/azure-openai-responses.lazy.js
var azureOpenAIResponsesApi = () => lazyApi(() => import("./chunk-DMTT4USJ.mjs"));

// target/vendor-pi/pi-ai/dist/api/google-generative-ai.lazy.js
var googleGenerativeAIApi = () => lazyApi(() => import("./chunk-OYT6PZUY.mjs"));

// target/vendor-pi/pi-ai/dist/api/google-vertex.lazy.js
var googleVertexApi = () => lazyApi(() => import("./chunk-HF7FT455.mjs"));

// target/vendor-pi/pi-ai/dist/api/mistral-conversations.lazy.js
var mistralConversationsApi = () => lazyApi(() => import("./chunk-MFDKPXJE.mjs"));

// target/vendor-pi/pi-ai/dist/api/openai-codex-responses.lazy.js
var openAICodexResponsesApi = () => lazyApi(() => import("./chunk-WKK7AB6I.mjs"));

// target/vendor-pi/pi-ai/dist/api/openai-completions.lazy.js
var openAICompletionsApi = () => lazyApi(() => import("./chunk-EA2IXWKG.mjs"));

// target/vendor-pi/pi-ai/dist/api/openai-responses.lazy.js
var openAIResponsesApi = () => lazyApi(() => import("./chunk-XYV5UC7B.mjs"));

// target/vendor-pi/pi-ai/dist/legacy-api-aliases.js
var anthropicMessagesStreams = anthropicMessagesApi();
var azureOpenAIResponsesStreams = azureOpenAIResponsesApi();
var googleGenerativeAIStreams = googleGenerativeAIApi();
var googleVertexStreams = googleVertexApi();
var mistralConversationsStreams = mistralConversationsApi();
var openAICodexResponsesStreams = openAICodexResponsesApi();
var openAICompletionsStreams = openAICompletionsApi();
var openAIResponsesStreams = openAIResponsesApi();
var streamAnthropic = anthropicMessagesStreams.stream;
var streamSimpleAnthropic = anthropicMessagesStreams.streamSimple;
var streamAzureOpenAIResponses = azureOpenAIResponsesStreams.stream;
var streamSimpleAzureOpenAIResponses = azureOpenAIResponsesStreams.streamSimple;
var streamGoogle = googleGenerativeAIStreams.stream;
var streamSimpleGoogle = googleGenerativeAIStreams.streamSimple;
var streamGoogleVertex = googleVertexStreams.stream;
var streamSimpleGoogleVertex = googleVertexStreams.streamSimple;
var streamMistral = mistralConversationsStreams.stream;
var streamSimpleMistral = mistralConversationsStreams.streamSimple;
var streamOpenAICodexResponses = openAICodexResponsesStreams.stream;
var streamSimpleOpenAICodexResponses = openAICodexResponsesStreams.streamSimple;
var streamOpenAICompletions = openAICompletionsStreams.stream;
var streamSimpleOpenAICompletions = openAICompletionsStreams.streamSimple;
var streamOpenAIResponses = openAIResponsesStreams.stream;
var streamSimpleOpenAIResponses = openAIResponsesStreams.streamSimple;

// target/vendor-pi/pi-ai/dist/providers/data/amazon-bedrock.json
var amazon_bedrock_default = {};

// target/vendor-pi/pi-ai/dist/providers/amazon-bedrock.models.js
var AMAZON_BEDROCK_MODELS = flattenChatModelCatalog("amazon-bedrock", amazon_bedrock_default);
var AMAZON_BEDROCK_IMAGE_MODELS = flattenImageModelCatalog("amazon-bedrock", amazon_bedrock_default);
var AMAZON_BEDROCK_CLASSIFIER_MODELS = flattenClassifierModelCatalog("amazon-bedrock", amazon_bedrock_default);

// target/vendor-pi/pi-ai/dist/providers/data/ant-ling.json
var ant_ling_default = {};

// target/vendor-pi/pi-ai/dist/providers/ant-ling.models.js
var ANT_LING_MODELS = flattenChatModelCatalog("ant-ling", ant_ling_default);
var ANT_LING_IMAGE_MODELS = flattenImageModelCatalog("ant-ling", ant_ling_default);
var ANT_LING_CLASSIFIER_MODELS = flattenClassifierModelCatalog("ant-ling", ant_ling_default);

// target/vendor-pi/pi-ai/dist/providers/data/anthropic.json
var anthropic_default = {};

// target/vendor-pi/pi-ai/dist/providers/anthropic.models.js
var ANTHROPIC_MODELS = flattenChatModelCatalog("anthropic", anthropic_default);
var ANTHROPIC_IMAGE_MODELS = flattenImageModelCatalog("anthropic", anthropic_default);
var ANTHROPIC_CLASSIFIER_MODELS = flattenClassifierModelCatalog("anthropic", anthropic_default);

// target/vendor-pi/pi-ai/dist/providers/data/azure-openai-responses.json
var azure_openai_responses_default = {};

// target/vendor-pi/pi-ai/dist/providers/azure-openai-responses.models.js
var AZURE_OPENAI_RESPONSES_MODELS = flattenChatModelCatalog("azure-openai-responses", azure_openai_responses_default);
var AZURE_OPENAI_RESPONSES_IMAGE_MODELS = flattenImageModelCatalog("azure-openai-responses", azure_openai_responses_default);
var AZURE_OPENAI_RESPONSES_CLASSIFIER_MODELS = flattenClassifierModelCatalog("azure-openai-responses", azure_openai_responses_default);

// target/vendor-pi/pi-ai/dist/providers/data/baseten.json
var baseten_default = {};

// target/vendor-pi/pi-ai/dist/providers/baseten.models.js
var BASETEN_MODELS = flattenChatModelCatalog("baseten", baseten_default);
var BASETEN_IMAGE_MODELS = flattenImageModelCatalog("baseten", baseten_default);
var BASETEN_CLASSIFIER_MODELS = flattenClassifierModelCatalog("baseten", baseten_default);

// target/vendor-pi/pi-ai/dist/providers/data/cerebras.json
var cerebras_default = {};

// target/vendor-pi/pi-ai/dist/providers/cerebras.models.js
var CEREBRAS_MODELS = flattenChatModelCatalog("cerebras", cerebras_default);
var CEREBRAS_IMAGE_MODELS = flattenImageModelCatalog("cerebras", cerebras_default);
var CEREBRAS_CLASSIFIER_MODELS = flattenClassifierModelCatalog("cerebras", cerebras_default);

// target/vendor-pi/pi-ai/dist/providers/data/cloudflare-ai-gateway.json
var cloudflare_ai_gateway_default = {};

// target/vendor-pi/pi-ai/dist/providers/cloudflare-ai-gateway.models.js
var CLOUDFLARE_AI_GATEWAY_MODELS = flattenChatModelCatalog("cloudflare-ai-gateway", cloudflare_ai_gateway_default);
var CLOUDFLARE_AI_GATEWAY_IMAGE_MODELS = flattenImageModelCatalog("cloudflare-ai-gateway", cloudflare_ai_gateway_default);
var CLOUDFLARE_AI_GATEWAY_CLASSIFIER_MODELS = flattenClassifierModelCatalog("cloudflare-ai-gateway", cloudflare_ai_gateway_default);

// target/vendor-pi/pi-ai/dist/providers/data/cloudflare-workers-ai.json
var cloudflare_workers_ai_default = {};

// target/vendor-pi/pi-ai/dist/providers/cloudflare-workers-ai.models.js
var CLOUDFLARE_WORKERS_AI_MODELS = flattenChatModelCatalog("cloudflare-workers-ai", cloudflare_workers_ai_default);
var CLOUDFLARE_WORKERS_AI_IMAGE_MODELS = flattenImageModelCatalog("cloudflare-workers-ai", cloudflare_workers_ai_default);
var CLOUDFLARE_WORKERS_AI_CLASSIFIER_MODELS = flattenClassifierModelCatalog("cloudflare-workers-ai", cloudflare_workers_ai_default);

// target/vendor-pi/pi-ai/dist/providers/data/deepseek.json
var deepseek_default = {};

// target/vendor-pi/pi-ai/dist/providers/deepseek.models.js
var DEEPSEEK_MODELS = flattenChatModelCatalog("deepseek", deepseek_default);
var DEEPSEEK_IMAGE_MODELS = flattenImageModelCatalog("deepseek", deepseek_default);
var DEEPSEEK_CLASSIFIER_MODELS = flattenClassifierModelCatalog("deepseek", deepseek_default);

// target/vendor-pi/pi-ai/dist/providers/data/fireworks.json
var fireworks_default = {};

// target/vendor-pi/pi-ai/dist/providers/fireworks.models.js
var FIREWORKS_MODELS = flattenChatModelCatalog("fireworks", fireworks_default);
var FIREWORKS_IMAGE_MODELS = flattenImageModelCatalog("fireworks", fireworks_default);
var FIREWORKS_CLASSIFIER_MODELS = flattenClassifierModelCatalog("fireworks", fireworks_default);

// target/vendor-pi/pi-ai/dist/providers/data/google.json
var google_default = {};

// target/vendor-pi/pi-ai/dist/providers/google.models.js
var GOOGLE_MODELS = flattenChatModelCatalog("google", google_default);
var GOOGLE_IMAGE_MODELS = flattenImageModelCatalog("google", google_default);
var GOOGLE_CLASSIFIER_MODELS = flattenClassifierModelCatalog("google", google_default);

// target/vendor-pi/pi-ai/dist/providers/data/google-vertex.json
var google_vertex_default = {};

// target/vendor-pi/pi-ai/dist/providers/google-vertex.models.js
var GOOGLE_VERTEX_MODELS = flattenChatModelCatalog("google-vertex", google_vertex_default);
var GOOGLE_VERTEX_IMAGE_MODELS = flattenImageModelCatalog("google-vertex", google_vertex_default);
var GOOGLE_VERTEX_CLASSIFIER_MODELS = flattenClassifierModelCatalog("google-vertex", google_vertex_default);

// target/vendor-pi/pi-ai/dist/providers/data/groq.json
var groq_default = {};

// target/vendor-pi/pi-ai/dist/providers/groq.models.js
var GROQ_MODELS = flattenChatModelCatalog("groq", groq_default);
var GROQ_IMAGE_MODELS = flattenImageModelCatalog("groq", groq_default);
var GROQ_CLASSIFIER_MODELS = flattenClassifierModelCatalog("groq", groq_default);

// target/vendor-pi/pi-ai/dist/providers/data/huggingface.json
var huggingface_default = {};

// target/vendor-pi/pi-ai/dist/providers/huggingface.models.js
var HUGGINGFACE_MODELS = flattenChatModelCatalog("huggingface", huggingface_default);
var HUGGINGFACE_IMAGE_MODELS = flattenImageModelCatalog("huggingface", huggingface_default);
var HUGGINGFACE_CLASSIFIER_MODELS = flattenClassifierModelCatalog("huggingface", huggingface_default);

// target/vendor-pi/pi-ai/dist/providers/data/kimi-coding.json
var kimi_coding_default = {};

// target/vendor-pi/pi-ai/dist/providers/kimi-coding.models.js
var KIMI_CODING_MODELS = flattenChatModelCatalog("kimi-coding", kimi_coding_default);
var KIMI_CODING_IMAGE_MODELS = flattenImageModelCatalog("kimi-coding", kimi_coding_default);
var KIMI_CODING_CLASSIFIER_MODELS = flattenClassifierModelCatalog("kimi-coding", kimi_coding_default);

// target/vendor-pi/pi-ai/dist/providers/data/meta.json
var meta_default = {};

// target/vendor-pi/pi-ai/dist/providers/meta.models.js
var META_MODELS = flattenChatModelCatalog("meta", meta_default);
var META_IMAGE_MODELS = flattenImageModelCatalog("meta", meta_default);
var META_CLASSIFIER_MODELS = flattenClassifierModelCatalog("meta", meta_default);

// target/vendor-pi/pi-ai/dist/providers/data/minimax.json
var minimax_default = {};

// target/vendor-pi/pi-ai/dist/providers/minimax.models.js
var MINIMAX_MODELS = flattenChatModelCatalog("minimax", minimax_default);
var MINIMAX_IMAGE_MODELS = flattenImageModelCatalog("minimax", minimax_default);
var MINIMAX_CLASSIFIER_MODELS = flattenClassifierModelCatalog("minimax", minimax_default);

// target/vendor-pi/pi-ai/dist/providers/data/minimax-cn.json
var minimax_cn_default = {};

// target/vendor-pi/pi-ai/dist/providers/minimax-cn.models.js
var MINIMAX_CN_MODELS = flattenChatModelCatalog("minimax-cn", minimax_cn_default);
var MINIMAX_CN_IMAGE_MODELS = flattenImageModelCatalog("minimax-cn", minimax_cn_default);
var MINIMAX_CN_CLASSIFIER_MODELS = flattenClassifierModelCatalog("minimax-cn", minimax_cn_default);

// target/vendor-pi/pi-ai/dist/providers/data/mistral.json
var mistral_default = {};

// target/vendor-pi/pi-ai/dist/providers/mistral.models.js
var MISTRAL_MODELS = flattenChatModelCatalog("mistral", mistral_default);
var MISTRAL_IMAGE_MODELS = flattenImageModelCatalog("mistral", mistral_default);
var MISTRAL_CLASSIFIER_MODELS = flattenClassifierModelCatalog("mistral", mistral_default);

// target/vendor-pi/pi-ai/dist/providers/data/moonshotai.json
var moonshotai_default = {};

// target/vendor-pi/pi-ai/dist/providers/moonshotai.models.js
var MOONSHOTAI_MODELS = flattenChatModelCatalog("moonshotai", moonshotai_default);
var MOONSHOTAI_IMAGE_MODELS = flattenImageModelCatalog("moonshotai", moonshotai_default);
var MOONSHOTAI_CLASSIFIER_MODELS = flattenClassifierModelCatalog("moonshotai", moonshotai_default);

// target/vendor-pi/pi-ai/dist/providers/data/moonshotai-cn.json
var moonshotai_cn_default = {};

// target/vendor-pi/pi-ai/dist/providers/moonshotai-cn.models.js
var MOONSHOTAI_CN_MODELS = flattenChatModelCatalog("moonshotai-cn", moonshotai_cn_default);
var MOONSHOTAI_CN_IMAGE_MODELS = flattenImageModelCatalog("moonshotai-cn", moonshotai_cn_default);
var MOONSHOTAI_CN_CLASSIFIER_MODELS = flattenClassifierModelCatalog("moonshotai-cn", moonshotai_cn_default);

// target/vendor-pi/pi-ai/dist/providers/data/nvidia.json
var nvidia_default = {};

// target/vendor-pi/pi-ai/dist/providers/nvidia.models.js
var NVIDIA_MODELS = flattenChatModelCatalog("nvidia", nvidia_default);
var NVIDIA_IMAGE_MODELS = flattenImageModelCatalog("nvidia", nvidia_default);
var NVIDIA_CLASSIFIER_MODELS = flattenClassifierModelCatalog("nvidia", nvidia_default);

// target/vendor-pi/pi-ai/dist/providers/data/openai.json
var openai_default = {};

// target/vendor-pi/pi-ai/dist/providers/openai.models.js
var OPENAI_MODELS = flattenChatModelCatalog("openai", openai_default);
var OPENAI_IMAGE_MODELS = flattenImageModelCatalog("openai", openai_default);
var OPENAI_CLASSIFIER_MODELS = flattenClassifierModelCatalog("openai", openai_default);

// target/vendor-pi/pi-ai/dist/providers/data/openai-codex.json
var openai_codex_default = {};

// target/vendor-pi/pi-ai/dist/providers/openai-codex.models.js
var OPENAI_CODEX_MODELS = flattenChatModelCatalog("openai-codex", openai_codex_default);
var OPENAI_CODEX_IMAGE_MODELS = flattenImageModelCatalog("openai-codex", openai_codex_default);
var OPENAI_CODEX_CLASSIFIER_MODELS = flattenClassifierModelCatalog("openai-codex", openai_codex_default);

// target/vendor-pi/pi-ai/dist/providers/data/opencode.json
var opencode_default = {};

// target/vendor-pi/pi-ai/dist/providers/opencode.models.js
var OPENCODE_MODELS = flattenChatModelCatalog("opencode", opencode_default);
var OPENCODE_IMAGE_MODELS = flattenImageModelCatalog("opencode", opencode_default);
var OPENCODE_CLASSIFIER_MODELS = flattenClassifierModelCatalog("opencode", opencode_default);

// target/vendor-pi/pi-ai/dist/providers/data/opencode-go.json
var opencode_go_default = {};

// target/vendor-pi/pi-ai/dist/providers/opencode-go.models.js
var OPENCODE_GO_MODELS = flattenChatModelCatalog("opencode-go", opencode_go_default);
var OPENCODE_GO_IMAGE_MODELS = flattenImageModelCatalog("opencode-go", opencode_go_default);
var OPENCODE_GO_CLASSIFIER_MODELS = flattenClassifierModelCatalog("opencode-go", opencode_go_default);

// target/vendor-pi/pi-ai/dist/providers/data/openrouter.json
var openrouter_default = {};

// target/vendor-pi/pi-ai/dist/providers/openrouter.models.js
var OPENROUTER_MODELS = flattenChatModelCatalog("openrouter", openrouter_default);
var OPENROUTER_IMAGE_MODELS = flattenImageModelCatalog("openrouter", openrouter_default);
var OPENROUTER_CLASSIFIER_MODELS = flattenClassifierModelCatalog("openrouter", openrouter_default);

// target/vendor-pi/pi-ai/dist/providers/data/qwen-token-plan.json
var qwen_token_plan_default = {};

// target/vendor-pi/pi-ai/dist/providers/qwen-token-plan.models.js
var QWEN_TOKEN_PLAN_MODELS = flattenChatModelCatalog("qwen-token-plan", qwen_token_plan_default);
var QWEN_TOKEN_PLAN_IMAGE_MODELS = flattenImageModelCatalog("qwen-token-plan", qwen_token_plan_default);
var QWEN_TOKEN_PLAN_CLASSIFIER_MODELS = flattenClassifierModelCatalog("qwen-token-plan", qwen_token_plan_default);

// target/vendor-pi/pi-ai/dist/providers/data/qwen-token-plan-cn.json
var qwen_token_plan_cn_default = {};

// target/vendor-pi/pi-ai/dist/providers/qwen-token-plan-cn.models.js
var QWEN_TOKEN_PLAN_CN_MODELS = flattenChatModelCatalog("qwen-token-plan-cn", qwen_token_plan_cn_default);
var QWEN_TOKEN_PLAN_CN_IMAGE_MODELS = flattenImageModelCatalog("qwen-token-plan-cn", qwen_token_plan_cn_default);
var QWEN_TOKEN_PLAN_CN_CLASSIFIER_MODELS = flattenClassifierModelCatalog("qwen-token-plan-cn", qwen_token_plan_cn_default);

// target/vendor-pi/pi-ai/dist/providers/data/qwen-token-plan-individual.json
var qwen_token_plan_individual_default = {};

// target/vendor-pi/pi-ai/dist/providers/qwen-token-plan-individual.models.js
var QWEN_TOKEN_PLAN_INDIVIDUAL_MODELS = flattenChatModelCatalog("qwen-token-plan-individual", qwen_token_plan_individual_default);
var QWEN_TOKEN_PLAN_INDIVIDUAL_IMAGE_MODELS = flattenImageModelCatalog("qwen-token-plan-individual", qwen_token_plan_individual_default);
var QWEN_TOKEN_PLAN_INDIVIDUAL_CLASSIFIER_MODELS = flattenClassifierModelCatalog("qwen-token-plan-individual", qwen_token_plan_individual_default);

// target/vendor-pi/pi-ai/dist/providers/data/radius.json
var radius_default = {};

// target/vendor-pi/pi-ai/dist/providers/radius.models.js
var RADIUS_MODELS = flattenChatModelCatalog("radius", radius_default);
var RADIUS_IMAGE_MODELS = flattenImageModelCatalog("radius", radius_default);
var RADIUS_CLASSIFIER_MODELS = flattenClassifierModelCatalog("radius", radius_default);

// target/vendor-pi/pi-ai/dist/providers/data/together.json
var together_default = {};

// target/vendor-pi/pi-ai/dist/providers/together.models.js
var TOGETHER_MODELS = flattenChatModelCatalog("together", together_default);
var TOGETHER_IMAGE_MODELS = flattenImageModelCatalog("together", together_default);
var TOGETHER_CLASSIFIER_MODELS = flattenClassifierModelCatalog("together", together_default);

// target/vendor-pi/pi-ai/dist/providers/data/typesafe.json
var typesafe_default = {};

// target/vendor-pi/pi-ai/dist/providers/typesafe.models.js
var TYPESAFE_MODELS = flattenChatModelCatalog("typesafe", typesafe_default);
var TYPESAFE_IMAGE_MODELS = flattenImageModelCatalog("typesafe", typesafe_default);
var TYPESAFE_CLASSIFIER_MODELS = flattenClassifierModelCatalog("typesafe", typesafe_default);

// target/vendor-pi/pi-ai/dist/providers/data/vercel-ai-gateway.json
var vercel_ai_gateway_default = {};

// target/vendor-pi/pi-ai/dist/providers/vercel-ai-gateway.models.js
var VERCEL_AI_GATEWAY_MODELS = flattenChatModelCatalog("vercel-ai-gateway", vercel_ai_gateway_default);
var VERCEL_AI_GATEWAY_IMAGE_MODELS = flattenImageModelCatalog("vercel-ai-gateway", vercel_ai_gateway_default);
var VERCEL_AI_GATEWAY_CLASSIFIER_MODELS = flattenClassifierModelCatalog("vercel-ai-gateway", vercel_ai_gateway_default);

// target/vendor-pi/pi-ai/dist/providers/data/xai.json
var xai_default = {};

// target/vendor-pi/pi-ai/dist/providers/xai.models.js
var XAI_MODELS = flattenChatModelCatalog("xai", xai_default);
var XAI_IMAGE_MODELS = flattenImageModelCatalog("xai", xai_default);
var XAI_CLASSIFIER_MODELS = flattenClassifierModelCatalog("xai", xai_default);

// target/vendor-pi/pi-ai/dist/providers/data/xiaomi.json
var xiaomi_default = {};

// target/vendor-pi/pi-ai/dist/providers/xiaomi.models.js
var XIAOMI_MODELS = flattenChatModelCatalog("xiaomi", xiaomi_default);
var XIAOMI_IMAGE_MODELS = flattenImageModelCatalog("xiaomi", xiaomi_default);
var XIAOMI_CLASSIFIER_MODELS = flattenClassifierModelCatalog("xiaomi", xiaomi_default);

// target/vendor-pi/pi-ai/dist/providers/data/xiaomi-token-plan-ams.json
var xiaomi_token_plan_ams_default = {};

// target/vendor-pi/pi-ai/dist/providers/xiaomi-token-plan-ams.models.js
var XIAOMI_TOKEN_PLAN_AMS_MODELS = flattenChatModelCatalog("xiaomi-token-plan-ams", xiaomi_token_plan_ams_default);
var XIAOMI_TOKEN_PLAN_AMS_IMAGE_MODELS = flattenImageModelCatalog("xiaomi-token-plan-ams", xiaomi_token_plan_ams_default);
var XIAOMI_TOKEN_PLAN_AMS_CLASSIFIER_MODELS = flattenClassifierModelCatalog("xiaomi-token-plan-ams", xiaomi_token_plan_ams_default);

// target/vendor-pi/pi-ai/dist/providers/data/xiaomi-token-plan-cn.json
var xiaomi_token_plan_cn_default = {};

// target/vendor-pi/pi-ai/dist/providers/xiaomi-token-plan-cn.models.js
var XIAOMI_TOKEN_PLAN_CN_MODELS = flattenChatModelCatalog("xiaomi-token-plan-cn", xiaomi_token_plan_cn_default);
var XIAOMI_TOKEN_PLAN_CN_IMAGE_MODELS = flattenImageModelCatalog("xiaomi-token-plan-cn", xiaomi_token_plan_cn_default);
var XIAOMI_TOKEN_PLAN_CN_CLASSIFIER_MODELS = flattenClassifierModelCatalog("xiaomi-token-plan-cn", xiaomi_token_plan_cn_default);

// target/vendor-pi/pi-ai/dist/providers/data/xiaomi-token-plan-sgp.json
var xiaomi_token_plan_sgp_default = {};

// target/vendor-pi/pi-ai/dist/providers/xiaomi-token-plan-sgp.models.js
var XIAOMI_TOKEN_PLAN_SGP_MODELS = flattenChatModelCatalog("xiaomi-token-plan-sgp", xiaomi_token_plan_sgp_default);
var XIAOMI_TOKEN_PLAN_SGP_IMAGE_MODELS = flattenImageModelCatalog("xiaomi-token-plan-sgp", xiaomi_token_plan_sgp_default);
var XIAOMI_TOKEN_PLAN_SGP_CLASSIFIER_MODELS = flattenClassifierModelCatalog("xiaomi-token-plan-sgp", xiaomi_token_plan_sgp_default);

// target/vendor-pi/pi-ai/dist/providers/data/zai.json
var zai_default = {};

// target/vendor-pi/pi-ai/dist/providers/zai.models.js
var ZAI_MODELS = flattenChatModelCatalog("zai", zai_default);
var ZAI_IMAGE_MODELS = flattenImageModelCatalog("zai", zai_default);
var ZAI_CLASSIFIER_MODELS = flattenClassifierModelCatalog("zai", zai_default);

// target/vendor-pi/pi-ai/dist/providers/data/zai-coding-cn.json
var zai_coding_cn_default = {};

// target/vendor-pi/pi-ai/dist/providers/zai-coding-cn.models.js
var ZAI_CODING_CN_MODELS = flattenChatModelCatalog("zai-coding-cn", zai_coding_cn_default);
var ZAI_CODING_CN_IMAGE_MODELS = flattenImageModelCatalog("zai-coding-cn", zai_coding_cn_default);
var ZAI_CODING_CN_CLASSIFIER_MODELS = flattenClassifierModelCatalog("zai-coding-cn", zai_coding_cn_default);

// target/vendor-pi/pi-ai/dist/models.generated.js
var MODELS = {
  "amazon-bedrock": AMAZON_BEDROCK_MODELS,
  "ant-ling": ANT_LING_MODELS,
  "anthropic": ANTHROPIC_MODELS,
  "azure-openai-responses": AZURE_OPENAI_RESPONSES_MODELS,
  "baseten": BASETEN_MODELS,
  "cerebras": CEREBRAS_MODELS,
  "cloudflare-ai-gateway": CLOUDFLARE_AI_GATEWAY_MODELS,
  "cloudflare-workers-ai": CLOUDFLARE_WORKERS_AI_MODELS,
  "deepseek": DEEPSEEK_MODELS,
  "fireworks": FIREWORKS_MODELS,
  "github-copilot": GITHUB_COPILOT_MODELS,
  "google": GOOGLE_MODELS,
  "google-vertex": GOOGLE_VERTEX_MODELS,
  "groq": GROQ_MODELS,
  "huggingface": HUGGINGFACE_MODELS,
  "kimi-coding": KIMI_CODING_MODELS,
  "meta": META_MODELS,
  "minimax": MINIMAX_MODELS,
  "minimax-cn": MINIMAX_CN_MODELS,
  "mistral": MISTRAL_MODELS,
  "moonshotai": MOONSHOTAI_MODELS,
  "moonshotai-cn": MOONSHOTAI_CN_MODELS,
  "nvidia": NVIDIA_MODELS,
  "openai": OPENAI_MODELS,
  "openai-codex": OPENAI_CODEX_MODELS,
  "opencode": OPENCODE_MODELS,
  "opencode-go": OPENCODE_GO_MODELS,
  "openrouter": OPENROUTER_MODELS,
  "qwen-token-plan": QWEN_TOKEN_PLAN_MODELS,
  "qwen-token-plan-cn": QWEN_TOKEN_PLAN_CN_MODELS,
  "qwen-token-plan-individual": QWEN_TOKEN_PLAN_INDIVIDUAL_MODELS,
  "radius": RADIUS_MODELS,
  "together": TOGETHER_MODELS,
  "typesafe": TYPESAFE_MODELS,
  "vercel-ai-gateway": VERCEL_AI_GATEWAY_MODELS,
  "xai": XAI_MODELS,
  "xiaomi": XIAOMI_MODELS,
  "xiaomi-token-plan-ams": XIAOMI_TOKEN_PLAN_AMS_MODELS,
  "xiaomi-token-plan-cn": XIAOMI_TOKEN_PLAN_CN_MODELS,
  "xiaomi-token-plan-sgp": XIAOMI_TOKEN_PLAN_SGP_MODELS,
  "zai": ZAI_MODELS,
  "zai-coding-cn": ZAI_CODING_CN_MODELS
};
var IMAGE_MODELS = {
  "amazon-bedrock": AMAZON_BEDROCK_IMAGE_MODELS,
  "ant-ling": ANT_LING_IMAGE_MODELS,
  "anthropic": ANTHROPIC_IMAGE_MODELS,
  "azure-openai-responses": AZURE_OPENAI_RESPONSES_IMAGE_MODELS,
  "baseten": BASETEN_IMAGE_MODELS,
  "cerebras": CEREBRAS_IMAGE_MODELS,
  "cloudflare-ai-gateway": CLOUDFLARE_AI_GATEWAY_IMAGE_MODELS,
  "cloudflare-workers-ai": CLOUDFLARE_WORKERS_AI_IMAGE_MODELS,
  "deepseek": DEEPSEEK_IMAGE_MODELS,
  "fireworks": FIREWORKS_IMAGE_MODELS,
  "github-copilot": GITHUB_COPILOT_IMAGE_MODELS,
  "google": GOOGLE_IMAGE_MODELS,
  "google-vertex": GOOGLE_VERTEX_IMAGE_MODELS,
  "groq": GROQ_IMAGE_MODELS,
  "huggingface": HUGGINGFACE_IMAGE_MODELS,
  "kimi-coding": KIMI_CODING_IMAGE_MODELS,
  "meta": META_IMAGE_MODELS,
  "minimax": MINIMAX_IMAGE_MODELS,
  "minimax-cn": MINIMAX_CN_IMAGE_MODELS,
  "mistral": MISTRAL_IMAGE_MODELS,
  "moonshotai": MOONSHOTAI_IMAGE_MODELS,
  "moonshotai-cn": MOONSHOTAI_CN_IMAGE_MODELS,
  "nvidia": NVIDIA_IMAGE_MODELS,
  "openai": OPENAI_IMAGE_MODELS,
  "openai-codex": OPENAI_CODEX_IMAGE_MODELS,
  "opencode": OPENCODE_IMAGE_MODELS,
  "opencode-go": OPENCODE_GO_IMAGE_MODELS,
  "openrouter": OPENROUTER_IMAGE_MODELS,
  "qwen-token-plan": QWEN_TOKEN_PLAN_IMAGE_MODELS,
  "qwen-token-plan-cn": QWEN_TOKEN_PLAN_CN_IMAGE_MODELS,
  "qwen-token-plan-individual": QWEN_TOKEN_PLAN_INDIVIDUAL_IMAGE_MODELS,
  "radius": RADIUS_IMAGE_MODELS,
  "together": TOGETHER_IMAGE_MODELS,
  "typesafe": TYPESAFE_IMAGE_MODELS,
  "vercel-ai-gateway": VERCEL_AI_GATEWAY_IMAGE_MODELS,
  "xai": XAI_IMAGE_MODELS,
  "xiaomi": XIAOMI_IMAGE_MODELS,
  "xiaomi-token-plan-ams": XIAOMI_TOKEN_PLAN_AMS_IMAGE_MODELS,
  "xiaomi-token-plan-cn": XIAOMI_TOKEN_PLAN_CN_IMAGE_MODELS,
  "xiaomi-token-plan-sgp": XIAOMI_TOKEN_PLAN_SGP_IMAGE_MODELS,
  "zai": ZAI_IMAGE_MODELS,
  "zai-coding-cn": ZAI_CODING_CN_IMAGE_MODELS
};
var CLASSIFIER_MODELS = {
  "amazon-bedrock": AMAZON_BEDROCK_CLASSIFIER_MODELS,
  "ant-ling": ANT_LING_CLASSIFIER_MODELS,
  "anthropic": ANTHROPIC_CLASSIFIER_MODELS,
  "azure-openai-responses": AZURE_OPENAI_RESPONSES_CLASSIFIER_MODELS,
  "baseten": BASETEN_CLASSIFIER_MODELS,
  "cerebras": CEREBRAS_CLASSIFIER_MODELS,
  "cloudflare-ai-gateway": CLOUDFLARE_AI_GATEWAY_CLASSIFIER_MODELS,
  "cloudflare-workers-ai": CLOUDFLARE_WORKERS_AI_CLASSIFIER_MODELS,
  "deepseek": DEEPSEEK_CLASSIFIER_MODELS,
  "fireworks": FIREWORKS_CLASSIFIER_MODELS,
  "github-copilot": GITHUB_COPILOT_CLASSIFIER_MODELS,
  "google": GOOGLE_CLASSIFIER_MODELS,
  "google-vertex": GOOGLE_VERTEX_CLASSIFIER_MODELS,
  "groq": GROQ_CLASSIFIER_MODELS,
  "huggingface": HUGGINGFACE_CLASSIFIER_MODELS,
  "kimi-coding": KIMI_CODING_CLASSIFIER_MODELS,
  "meta": META_CLASSIFIER_MODELS,
  "minimax": MINIMAX_CLASSIFIER_MODELS,
  "minimax-cn": MINIMAX_CN_CLASSIFIER_MODELS,
  "mistral": MISTRAL_CLASSIFIER_MODELS,
  "moonshotai": MOONSHOTAI_CLASSIFIER_MODELS,
  "moonshotai-cn": MOONSHOTAI_CN_CLASSIFIER_MODELS,
  "nvidia": NVIDIA_CLASSIFIER_MODELS,
  "openai": OPENAI_CLASSIFIER_MODELS,
  "openai-codex": OPENAI_CODEX_CLASSIFIER_MODELS,
  "opencode": OPENCODE_CLASSIFIER_MODELS,
  "opencode-go": OPENCODE_GO_CLASSIFIER_MODELS,
  "openrouter": OPENROUTER_CLASSIFIER_MODELS,
  "qwen-token-plan": QWEN_TOKEN_PLAN_CLASSIFIER_MODELS,
  "qwen-token-plan-cn": QWEN_TOKEN_PLAN_CN_CLASSIFIER_MODELS,
  "qwen-token-plan-individual": QWEN_TOKEN_PLAN_INDIVIDUAL_CLASSIFIER_MODELS,
  "radius": RADIUS_CLASSIFIER_MODELS,
  "together": TOGETHER_CLASSIFIER_MODELS,
  "typesafe": TYPESAFE_CLASSIFIER_MODELS,
  "vercel-ai-gateway": VERCEL_AI_GATEWAY_CLASSIFIER_MODELS,
  "xai": XAI_CLASSIFIER_MODELS,
  "xiaomi": XIAOMI_CLASSIFIER_MODELS,
  "xiaomi-token-plan-ams": XIAOMI_TOKEN_PLAN_AMS_CLASSIFIER_MODELS,
  "xiaomi-token-plan-cn": XIAOMI_TOKEN_PLAN_CN_CLASSIFIER_MODELS,
  "xiaomi-token-plan-sgp": XIAOMI_TOKEN_PLAN_SGP_CLASSIFIER_MODELS,
  "zai": ZAI_CLASSIFIER_MODELS,
  "zai-coding-cn": ZAI_CODING_CN_CLASSIFIER_MODELS
};

// target/vendor-pi/pi-ai/dist/image-models.js
var imageModelsByProvider = /* @__PURE__ */ new Map();
for (const [provider, models] of Object.entries(IMAGE_MODELS)) {
  const imageModels = /* @__PURE__ */ new Map();
  for (const model of Object.values(models)) {
    imageModels.set(model.id, model);
  }
  if (imageModels.size > 0)
    imageModelsByProvider.set(provider, imageModels);
}
function getImageModel(provider, modelId) {
  return imageModelsByProvider.get(provider)?.get(modelId);
}
function getImageProviders() {
  return Array.from(imageModelsByProvider.keys());
}
function getImageModels(provider) {
  const models = imageModelsByProvider.get(provider);
  return models ? Array.from(models.values()) : [];
}

// target/vendor-pi/pi-ai/dist/images-api-registry.js
var imagesApiProviderRegistry = /* @__PURE__ */ new Map();
function wrapGenerateImages(api, generateImages2) {
  return (model, context, options) => {
    if (model.api !== api) {
      throw new Error(`Mismatched api: ${model.api} expected ${api}`);
    }
    return generateImages2(model, context, options);
  };
}
function registerImagesApiProvider(provider, sourceId) {
  imagesApiProviderRegistry.set(provider.api, {
    provider: {
      api: provider.api,
      generateImages: wrapGenerateImages(provider.api, provider.generateImages)
    },
    sourceId
  });
}
function getImagesApiProvider(api) {
  return imagesApiProviderRegistry.get(api)?.provider;
}

// target/vendor-pi/pi-ai/dist/providers/images/register-builtins.js
var openRouterImagesProviderModulePromise;
function createLazyLoadErrorImages(model, error) {
  return {
    api: model.api,
    provider: model.provider,
    model: model.id,
    output: [],
    stopReason: "error",
    errorMessage: error instanceof Error ? error.message : String(error),
    timestamp: Date.now()
  };
}
function loadOpenRouterImagesProviderModule() {
  openRouterImagesProviderModulePromise ||= import("./chunk-JZ4LANIM.mjs").then((module) => module);
  return openRouterImagesProviderModulePromise;
}
var generateImagesOpenRouter = async (model, context, options) => {
  try {
    const module = await loadOpenRouterImagesProviderModule();
    return await module.generateImages(model, context, options);
  } catch (error) {
    return createLazyLoadErrorImages(model, error);
  }
};
function registerBuiltInImagesApiProviders() {
  registerImagesApiProvider({
    api: "openrouter-images",
    generateImages: generateImagesOpenRouter
  });
}
registerBuiltInImagesApiProviders();

// target/vendor-pi/pi-ai/dist/images.js
function resolveImagesApiProvider(api) {
  const provider = getImagesApiProvider(api);
  if (!provider) {
    throw new Error(`No API provider registered for api: ${api}`);
  }
  return provider;
}
async function generateImages(model, context, options) {
  const provider = resolveImagesApiProvider(model.api);
  return provider.generateImages(model, context, options);
}

// target/vendor-pi/pi-ai/dist/session-resources.js
var sessionResourceCleanups = /* @__PURE__ */ new Set();
function registerSessionResourceCleanup(cleanup) {
  sessionResourceCleanups.add(cleanup);
  return () => {
    sessionResourceCleanups.delete(cleanup);
  };
}
function cleanupSessionResources(sessionId) {
  const errors = [];
  for (const cleanup of sessionResourceCleanups) {
    try {
      cleanup(sessionId);
    } catch (error) {
      errors.push(error);
    }
  }
  if (errors.length > 0) {
    throw new AggregateError(errors, "Failed to cleanup session resources");
  }
}

// target/vendor-pi/pi-ai/dist/utils/abort-signals.js
function combineAbortSignals(signals) {
  const activeSignals = signals.filter((signal) => signal !== void 0);
  if (activeSignals.length === 0) {
    return { cleanup: () => {
    } };
  }
  if (activeSignals.length === 1) {
    return { signal: activeSignals[0], cleanup: () => {
    } };
  }
  const controller = new AbortController();
  const listeners = [];
  const abort = (signal) => {
    if (!controller.signal.aborted) {
      controller.abort(signal.reason);
    }
  };
  for (const signal of activeSignals) {
    if (signal.aborted) {
      abort(signal);
      break;
    }
    const listener = () => abort(signal);
    signal.addEventListener("abort", listener, { once: true });
    listeners.push({ signal, listener });
  }
  return {
    signal: controller.signal,
    cleanup: () => {
      for (const { signal, listener } of listeners) {
        signal.removeEventListener("abort", listener);
      }
    }
  };
}

// target/vendor-pi/pi-ai/dist/utils/assistant-message-frame.js
function cloneTextContent(content) {
  return {
    type: "text",
    text: content.text,
    ...content.textSignature === void 0 ? {} : { textSignature: content.textSignature }
  };
}
function cloneThinkingContent(content) {
  return {
    type: "thinking",
    thinking: content.thinking,
    ...content.thinkingSignature === void 0 ? {} : { thinkingSignature: content.thinkingSignature },
    ...content.redacted === void 0 ? {} : { redacted: content.redacted }
  };
}
function cloneToolCall(toolCall) {
  return {
    type: "toolCall",
    id: toolCall.id,
    name: toolCall.name,
    arguments: structuredClone(toolCall.arguments),
    ...toolCall.thoughtSignature === void 0 ? {} : { thoughtSignature: toolCall.thoughtSignature },
    ...toolCall.namespace === void 0 ? {} : { namespace: toolCall.namespace }
  };
}
function cloneStartMessage(message) {
  return {
    role: "assistant",
    content: [],
    api: message.api,
    provider: message.provider,
    model: message.model,
    ...message.responseModel === void 0 ? {} : { responseModel: message.responseModel },
    ...message.responseId === void 0 ? {} : { responseId: message.responseId },
    ...message.providerThinkingLevel === void 0 ? {} : { providerThinkingLevel: message.providerThinkingLevel },
    ...message.diagnostics === void 0 ? {} : { diagnostics: structuredClone(message.diagnostics) },
    usage: structuredClone(message.usage),
    stopReason: "pending",
    timestamp: message.timestamp
  };
}
function assertContentIndex(contentIndex) {
  if (!Number.isSafeInteger(contentIndex) || contentIndex < 0) {
    throw new Error(`Invalid assistant message frame contentIndex: ${contentIndex}`);
  }
}
function eventBlock(event) {
  assertContentIndex(event.contentIndex);
  const block = event.partial.content[event.contentIndex];
  if (!block) {
    throw new Error(`${event.type} event has no content block at index ${event.contentIndex}`);
  }
  return block;
}
function serializedArguments(argumentsValue) {
  const serialized = JSON.stringify(argumentsValue);
  if (serialized === void 0)
    throw new Error("Tool-call arguments are not JSON-serializable");
  return serialized;
}
var EMPTY_PARSED_TOOL_ARGUMENTS = serializedArguments(parseStreamingJson(""));
function isJsonPrefix(snapshot, current) {
  if (typeof snapshot === "string")
    return typeof current === "string" && current.startsWith(snapshot);
  if (Array.isArray(snapshot)) {
    return Array.isArray(current) && snapshot.length <= current.length && snapshot.every((value, index) => isJsonPrefix(value, current[index]));
  }
  if (typeof snapshot !== "object" || snapshot === null)
    return Object.is(snapshot, current);
  if (typeof current !== "object" || current === null || Array.isArray(current))
    return false;
  const currentRecord = current;
  return Object.entries(snapshot).every(([key, value]) => Object.hasOwn(currentRecord, key) && isJsonPrefix(value, currentRecord[key]));
}
var AssistantMessageFrameEncoder = class {
  started = false;
  terminal = false;
  blocks = /* @__PURE__ */ new Map();
  encode(event) {
    if (this.terminal)
      throw new Error(`Assistant message event ${event.type} follows a terminal event`);
    switch (event.type) {
      case "start":
        if (this.started)
          throw new Error("Assistant message stream contains more than one start event");
        this.started = true;
        return { type: "start", partial: cloneStartMessage(event.partial) };
      case "done":
        if (!this.started)
          throw new Error("Assistant message done event appears before start");
        this.terminal = true;
        return void 0;
      case "error":
        this.terminal = true;
        return void 0;
    }
    if (!this.started)
      throw new Error(`Assistant message ${event.type} event appears before start`);
    switch (event.type) {
      case "text_start": {
        const content = eventBlock(event);
        if (content.type !== "text") {
          throw new Error(`text_start event points to ${content.type} block at index ${event.contentIndex}`);
        }
        this.startBlock(event.contentIndex, {
          kind: "text",
          coveredChars: content.text.length,
          deltaChars: 0
        });
        return { type: "text_start", contentIndex: event.contentIndex, content: cloneTextContent(content) };
      }
      case "text_delta":
        return this.encodeTextDelta(event.contentIndex, event.delta, "text");
      case "text_end": {
        const content = eventBlock(event);
        if (content.type !== "text") {
          throw new Error(`text_end event points to ${content.type} block at index ${event.contentIndex}`);
        }
        this.endBlock(event.contentIndex, "text");
        return {
          type: "text_end",
          contentIndex: event.contentIndex,
          content: event.content,
          ...content.textSignature === void 0 ? {} : { textSignature: content.textSignature }
        };
      }
      case "thinking_start": {
        const content = eventBlock(event);
        if (content.type !== "thinking") {
          throw new Error(`thinking_start event points to ${content.type} block at index ${event.contentIndex}`);
        }
        this.startBlock(event.contentIndex, {
          kind: "thinking",
          coveredChars: content.thinking.length,
          deltaChars: 0
        });
        return {
          type: "thinking_start",
          contentIndex: event.contentIndex,
          content: cloneThinkingContent(content)
        };
      }
      case "thinking_delta":
        return this.encodeTextDelta(event.contentIndex, event.delta, "thinking");
      case "thinking_end": {
        const content = eventBlock(event);
        if (content.type !== "thinking") {
          throw new Error(`thinking_end event points to ${content.type} block at index ${event.contentIndex}`);
        }
        this.endBlock(event.contentIndex, "thinking");
        return {
          type: "thinking_end",
          contentIndex: event.contentIndex,
          content: event.content,
          ...content.thinkingSignature === void 0 ? {} : { thinkingSignature: content.thinkingSignature },
          ...content.redacted === void 0 ? {} : { redacted: content.redacted }
        };
      }
      case "toolcall_start": {
        const content = eventBlock(event);
        if (content.type !== "toolCall") {
          throw new Error(`toolcall_start event points to ${content.type} block at index ${event.contentIndex}`);
        }
        const snapshotArguments = serializedArguments(content.arguments);
        const caughtUp = snapshotArguments === EMPTY_PARSED_TOOL_ARGUMENTS;
        this.startBlock(event.contentIndex, {
          kind: "toolCall",
          caughtUp,
          catchupJson: "",
          snapshotArguments: caughtUp ? "" : snapshotArguments
        });
        return { type: "toolcall_start", contentIndex: event.contentIndex, toolCall: cloneToolCall(content) };
      }
      case "toolcall_delta": {
        const state = this.block(event.contentIndex, "toolCall");
        if (state.kind !== "toolCall")
          throw new Error("Unreachable tool-call encoder state");
        if (state.caughtUp) {
          return event.delta.length === 0 ? void 0 : { type: "toolcall_delta", contentIndex: event.contentIndex, delta: event.delta };
        }
        state.catchupJson += event.delta;
        const argumentsValue = parseStreamingJson(state.catchupJson);
        if (serializedArguments(argumentsValue) !== state.snapshotArguments) {
          const snapshotArguments = parseStreamingJson(state.snapshotArguments);
          if (!isJsonPrefix(snapshotArguments, argumentsValue))
            return void 0;
        }
        state.caughtUp = true;
        state.snapshotArguments = "";
        const json = state.catchupJson;
        state.catchupJson = "";
        return json.length === 0 ? void 0 : { type: "toolcall_checkpoint", contentIndex: event.contentIndex, json };
      }
      case "toolcall_end": {
        const content = eventBlock(event);
        if (content.type !== "toolCall") {
          throw new Error(`toolcall_end event points to ${content.type} block at index ${event.contentIndex}`);
        }
        if (event.toolCall.type !== "toolCall") {
          throw new Error(`toolcall_end event has invalid tool call at index ${event.contentIndex}`);
        }
        this.endBlock(event.contentIndex, "toolCall");
        return {
          type: "toolcall_end",
          contentIndex: event.contentIndex,
          id: event.toolCall.id,
          name: event.toolCall.name,
          arguments: structuredClone(event.toolCall.arguments),
          ...event.toolCall.thoughtSignature === void 0 ? {} : { thoughtSignature: event.toolCall.thoughtSignature },
          ...event.toolCall.namespace === void 0 ? {} : { namespace: event.toolCall.namespace }
        };
      }
    }
  }
  startBlock(contentIndex, state) {
    assertContentIndex(contentIndex);
    if (this.blocks.has(contentIndex)) {
      throw new Error(`Assistant message block ${contentIndex} starts more than once`);
    }
    this.blocks.set(contentIndex, state);
  }
  block(contentIndex, kind) {
    assertContentIndex(contentIndex);
    const state = this.blocks.get(contentIndex);
    if (state === void 0)
      throw new Error(`Assistant message ${kind} block ${contentIndex} has not started`);
    if (state.kind !== kind) {
      throw new Error(`Assistant message block ${contentIndex} is ${state.kind}, not ${kind}`);
    }
    return state;
  }
  endBlock(contentIndex, kind) {
    this.block(contentIndex, kind);
    this.blocks.delete(contentIndex);
  }
  encodeTextDelta(contentIndex, delta, kind) {
    const state = this.block(contentIndex, kind);
    if (state.kind === "toolCall")
      throw new Error("Unreachable text encoder state");
    const deltaStart = state.deltaChars;
    state.deltaChars += delta.length;
    const covered = Math.max(0, state.coveredChars - deltaStart);
    if (covered >= delta.length)
      return void 0;
    const uncovered = covered === 0 ? delta : delta.slice(covered);
    return kind === "text" ? { type: "text_delta", contentIndex, delta: uncovered } : { type: "thinking_delta", contentIndex, delta: uncovered };
  }
};
function appendBlock(message, states, contentIndex, block, state) {
  assertContentIndex(contentIndex);
  if (contentIndex !== message.content.length) {
    const reason = contentIndex < message.content.length ? "already exists" : "would leave a gap";
    throw new Error(`Cannot start assistant message block at index ${contentIndex}: ${reason}`);
  }
  message.content.push(structuredClone(block));
  states.set(contentIndex, state);
}
function activeBlock(message, states, contentIndex, expectedKind, frameType) {
  assertContentIndex(contentIndex);
  const state = states.get(contentIndex);
  const block = message.content[contentIndex];
  if (!state || !block) {
    throw new Error(`${frameType} frame has no started block at index ${contentIndex}`);
  }
  if (state.kind !== expectedKind || block.type !== expectedKind) {
    throw new Error(`${frameType} frame expected ${expectedKind} block at index ${contentIndex}, found ${block.type}`);
  }
  if (state.ended) {
    throw new Error(`${frameType} frame follows the end of block at index ${contentIndex}`);
  }
  return { block, state };
}
function reduceAssistantMessageFrames(frames) {
  let message;
  let frameBeforeStart;
  const states = /* @__PURE__ */ new Map();
  for (const frame of frames) {
    if (frame.type === "start") {
      if (message)
        throw new Error("Assistant message frame sequence contains more than one start frame");
      if (frameBeforeStart !== void 0)
        throw new Error(`${frameBeforeStart} frame appears before the start frame`);
      message = structuredClone(frame.partial);
      continue;
    }
    if (!message) {
      frameBeforeStart ??= frame.type;
      continue;
    }
    switch (frame.type) {
      case "text_start":
        if (frame.content.type !== "text") {
          throw new Error(`text_start frame contains ${frame.content.type} content`);
        }
        appendBlock(message, states, frame.contentIndex, frame.content, { kind: "text", ended: false });
        break;
      case "text_delta": {
        const { block } = activeBlock(message, states, frame.contentIndex, "text", frame.type);
        if (block.type !== "text")
          throw new Error("Unreachable text frame state");
        block.text += frame.delta;
        break;
      }
      case "text_end": {
        const { block, state } = activeBlock(message, states, frame.contentIndex, "text", frame.type);
        if (block.type !== "text")
          throw new Error("Unreachable text frame state");
        block.text = frame.content;
        delete block.textSignature;
        if (frame.textSignature !== void 0)
          block.textSignature = frame.textSignature;
        state.ended = true;
        break;
      }
      case "thinking_start":
        if (frame.content.type !== "thinking") {
          throw new Error(`thinking_start frame contains ${frame.content.type} content`);
        }
        appendBlock(message, states, frame.contentIndex, frame.content, {
          kind: "thinking",
          ended: false
        });
        break;
      case "thinking_delta": {
        const { block } = activeBlock(message, states, frame.contentIndex, "thinking", frame.type);
        if (block.type !== "thinking")
          throw new Error("Unreachable thinking frame state");
        block.thinking += frame.delta;
        break;
      }
      case "thinking_end": {
        const { block, state } = activeBlock(message, states, frame.contentIndex, "thinking", frame.type);
        if (block.type !== "thinking")
          throw new Error("Unreachable thinking frame state");
        block.thinking = frame.content;
        delete block.thinkingSignature;
        delete block.redacted;
        if (frame.thinkingSignature !== void 0)
          block.thinkingSignature = frame.thinkingSignature;
        if (frame.redacted !== void 0)
          block.redacted = frame.redacted;
        state.ended = true;
        break;
      }
      case "toolcall_start":
        if (frame.toolCall.type !== "toolCall") {
          throw new Error(`toolcall_start frame contains ${frame.toolCall.type} content`);
        }
        appendBlock(message, states, frame.contentIndex, frame.toolCall, {
          kind: "toolCall",
          ended: false,
          json: ""
        });
        break;
      case "toolcall_checkpoint": {
        const { block, state } = activeBlock(message, states, frame.contentIndex, "toolCall", frame.type);
        if (block.type !== "toolCall" || state.kind !== "toolCall") {
          throw new Error("Unreachable tool-call checkpoint state");
        }
        state.json = frame.json;
        block.arguments = parseStreamingJson(frame.json);
        break;
      }
      case "toolcall_delta": {
        const { block, state } = activeBlock(message, states, frame.contentIndex, "toolCall", frame.type);
        if (block.type !== "toolCall" || state.kind !== "toolCall") {
          throw new Error("Unreachable tool-call frame state");
        }
        state.json += frame.delta;
        break;
      }
      case "toolcall_end": {
        const { block, state } = activeBlock(message, states, frame.contentIndex, "toolCall", frame.type);
        if (block.type !== "toolCall")
          throw new Error("Unreachable tool-call frame state");
        block.id = frame.id;
        block.name = frame.name;
        block.arguments = structuredClone(frame.arguments);
        delete block.thoughtSignature;
        delete block.namespace;
        if (frame.thoughtSignature !== void 0)
          block.thoughtSignature = frame.thoughtSignature;
        if (frame.namespace !== void 0)
          block.namespace = frame.namespace;
        state.ended = true;
        break;
      }
    }
  }
  if (!message)
    return void 0;
  for (const [contentIndex, state] of states) {
    if (state.kind !== "toolCall" || state.ended || state.json.length === 0)
      continue;
    const block = message.content[contentIndex];
    if (block?.type !== "toolCall")
      throw new Error("Unreachable tool-call frame state");
    block.arguments = parseStreamingJson(state.json);
  }
  return message;
}

// target/vendor-pi/pi-ai/dist/utils/node-http-proxy.js
var DEFAULT_PROXY_PORTS = {
  ftp: 21,
  gopher: 70,
  http: 80,
  https: 443,
  ws: 80,
  wss: 443
};
function getProxyEnv(key, env) {
  const lowercaseKey = key.toLowerCase();
  const uppercaseKey = key.toUpperCase();
  return env?.[lowercaseKey] || env?.[uppercaseKey] || getProviderEnvValue(lowercaseKey) || getProviderEnvValue(uppercaseKey) || "";
}
function parseProxyTargetUrl(targetUrl) {
  if (targetUrl instanceof URL) {
    return targetUrl;
  }
  try {
    return new URL(targetUrl);
  } catch {
    return void 0;
  }
}
function stripBrackets(host2) {
  return host2.startsWith("[") && host2.endsWith("]") ? host2.slice(1, -1) : host2;
}
function parseNoProxyEntry(entry) {
  const trimmed = entry.trim().toLowerCase();
  if (!trimmed)
    return void 0;
  if (trimmed.startsWith("[")) {
    const closingBracket = trimmed.indexOf("]");
    if (closingBracket !== -1) {
      const host2 = trimmed.slice(1, closingBracket);
      const rest = trimmed.slice(closingBracket + 1);
      if (rest.startsWith(":")) {
        const port = Number.parseInt(rest.slice(1), 10);
        return { host: host2, port: Number.isNaN(port) ? 0 : port };
      }
      return { host: host2, port: 0 };
    }
  }
  if (trimmed.includes(":") && trimmed.split(":").length > 2) {
    return { host: trimmed, port: 0 };
  }
  const colonIndex = trimmed.lastIndexOf(":");
  if (colonIndex !== -1 && colonIndex === trimmed.indexOf(":")) {
    const host2 = trimmed.slice(0, colonIndex);
    const port = Number.parseInt(trimmed.slice(colonIndex + 1), 10);
    if (!Number.isNaN(port)) {
      return { host: host2, port };
    }
  }
  return { host: trimmed, port: 0 };
}
function shouldProxyHostname(hostname, port, env) {
  const noProxy = getProxyEnv("no_proxy", env).toLowerCase();
  if (!noProxy) {
    return true;
  }
  if (noProxy === "*") {
    return false;
  }
  const normalizedTargetHost = stripBrackets(hostname.toLowerCase());
  return noProxy.split(/[,\s]/).every((entry) => {
    const parsed = parseNoProxyEntry(entry);
    if (!parsed) {
      return true;
    }
    if (parsed.port && parsed.port !== port) {
      return true;
    }
    let domain = stripBrackets(parsed.host);
    if (domain.startsWith("*.")) {
      domain = domain.slice(2);
    } else if (domain.startsWith(".") || domain.startsWith("*")) {
      domain = domain.slice(1);
    }
    if (!domain) {
      return true;
    }
    if (normalizedTargetHost === domain) {
      return false;
    }
    if (normalizedTargetHost.endsWith(`.${domain}`)) {
      return false;
    }
    return true;
  });
}
function getProxyForUrl(targetUrl, env) {
  const parsedUrl = parseProxyTargetUrl(targetUrl);
  if (!parsedUrl?.protocol || !parsedUrl.host) {
    return "";
  }
  const protocol = parsedUrl.protocol.split(":", 1)[0];
  const hostname = stripBrackets(parsedUrl.hostname || parsedUrl.host.replace(/:\d*$/, ""));
  const port = Number.parseInt(parsedUrl.port, 10) || DEFAULT_PROXY_PORTS[protocol] || 0;
  if (!shouldProxyHostname(hostname, port, env)) {
    return "";
  }
  let proxy = getProxyEnv(`${protocol}_proxy`, env) || getProxyEnv("all_proxy", env);
  if (proxy && !proxy.includes("://")) {
    proxy = `${protocol}://${proxy}`;
  }
  return proxy;
}
var UNSUPPORTED_PROXY_PROTOCOL_MESSAGE = "Unsupported proxy protocol. SOCKS and PAC proxy URLs are not supported; use an HTTP or HTTPS proxy URL.";
function resolveHttpProxyUrlForTarget(targetUrl, env) {
  const proxy = getProxyForUrl(targetUrl, env);
  if (!proxy) {
    return void 0;
  }
  let proxyUrl;
  try {
    proxyUrl = new URL(proxy);
  } catch (error) {
    throw new Error(`Invalid proxy URL ${JSON.stringify(proxy)}: ${error instanceof Error ? error.message : String(error)}`);
  }
  if (proxyUrl.protocol !== "http:" && proxyUrl.protocol !== "https:") {
    throw new Error(`${UNSUPPORTED_PROXY_PROTOCOL_MESSAGE} Got ${proxyUrl.protocol}`);
  }
  return proxyUrl;
}

// target/vendor-pi/pi-ai/dist/utils/overflow.js
var OVERFLOW_PATTERNS = [
  /prompt (?:is )?too long/i,
  // Anthropic and z.ai token overflow
  /prompt exceeds max length/i,
  // z.ai CN endpoint token overflow
  /request_too_large/i,
  // Anthropic request byte-size overflow (HTTP 413)
  /input is too long for requested model/i,
  // Amazon Bedrock
  /exceeds the context window/i,
  // OpenAI (Completions & Responses API)
  /exceeds (?:the )?(?:model'?s )?maximum context length(?: of [\d,]+ tokens?|\s*\([\d,]+\))/i,
  // OpenAI-compatible proxies (LiteLLM)
  /input token count.*exceeds the maximum/i,
  // Google (Gemini)
  /maximum prompt length is \d+/i,
  // xAI (Grok)
  /reduce the length of the messages/i,
  // Groq
  /maximum context length is \d+ tokens/i,
  // OpenRouter (most backends)
  /exceeds (?:the )?maximum allowed input length of [\d,]+ tokens?/i,
  // OpenRouter/Poolside
  /input \(\d+ tokens\) is longer than the model'?s context length \(\d+ tokens\)/i,
  // Together AI
  /exceeds the limit of \d+/i,
  // GitHub Copilot
  /exceeds the available context size/i,
  // llama.cpp server
  /greater than the context length/i,
  // LM Studio
  /context window exceeds limit/i,
  // MiniMax
  /exceeded model token limit/i,
  // Kimi For Coding
  /too large for model with \d+ maximum context length/i,
  // Mistral
  /prompt has [\d,]+ tokens?, but the configured context size is [\d,]+ tokens?/i,
  // DS4 server
  /model_context_window_exceeded/i,
  // z.ai non-standard finish_reason surfaced as error text
  /prompt too long; exceeded (?:max )?context length/i,
  // Ollama explicit overflow error
  /range of input length should be/i,
  // DashScope / Qwen Token Plan
  /context[_ ]length[_ ]exceeded/i,
  // Generic fallback
  /too many tokens/i,
  // Generic fallback
  /token limit exceeded/i
  // Generic fallback
];
var CEREBRAS_BODYLESS_OVERFLOW_PATTERN = /^4(?:00|13)\s*(?:status code)?\s*\(no body\)/i;
var NON_OVERFLOW_PATTERNS = [
  /^(Throttling error|Service unavailable):/i,
  // AWS Bedrock non-overflow errors (human-readable prefixes from formatBedrockError)
  /rate limit/i,
  // Generic rate limiting
  /too many requests/i
  // Generic HTTP 429 style
];
function isContextOverflow(message, contextWindow) {
  if (message.stopReason === "error" && message.errorMessage) {
    const isNonOverflow = NON_OVERFLOW_PATTERNS.some((p) => p.test(message.errorMessage));
    if (!isNonOverflow) {
      if (OVERFLOW_PATTERNS.some((p) => p.test(message.errorMessage))) {
        return true;
      }
      if (message.provider === "cerebras" && CEREBRAS_BODYLESS_OVERFLOW_PATTERN.test(message.errorMessage)) {
        return true;
      }
    }
  }
  if (contextWindow && message.stopReason === "stop") {
    const inputTokens = message.usage.input + message.usage.cacheRead;
    if (inputTokens > contextWindow) {
      return true;
    }
  }
  if (contextWindow && message.stopReason === "length" && message.usage.output === 0) {
    const inputTokens = message.usage.input + message.usage.cacheRead;
    if (inputTokens >= contextWindow * 0.99) {
      return true;
    }
  }
  return false;
}
function isRecoverableLength(message, desiredMaxOutput) {
  return message.stopReason === "length" && desiredMaxOutput > 0 && message.usage.output < desiredMaxOutput;
}
function getOverflowPatterns() {
  return [...OVERFLOW_PATTERNS];
}

// target/vendor-pi/pi-ai/dist/utils/retry.js
function buildProviderErrorPattern(patterns) {
  return new RegExp(patterns.join("|"), "i");
}
var NON_RETRYABLE_PROVIDER_LIMIT_ERROR_PATTERN = buildProviderErrorPattern([
  // OpenCode Go/free-tier limits returned as 429 JSON error types by OpenCode's
  // Zen API. These are subscription/account limits, not transient throttles.
  "GoUsageLimitError",
  "FreeUsageLimitError",
  // OpenCode Go subscription-limit text asks users to enable available-balance
  // usage after rolling/weekly/monthly limits are reached.
  "Monthly usage limit reached",
  "available balance",
  // Generic quota/budget/billing exhaustion. `insufficient_quota` is OpenAI's
  // quota/billing error code; the other strings cover common gateway wording.
  "insufficient_quota",
  "out of budget",
  "quota exceeded",
  "billing",
  // Sign in with ChatGPT: the subscription's shared usage limit, which resets
  // after hours rather than seconds.
  "subscription_sharing_usage_limit_exceeded"
]);
var RETRYABLE_PROVIDER_ERROR_PATTERN = buildProviderErrorPattern([
  // Generic provider load, HTTP status, and server-side transient failures.
  "overloaded",
  "currently experiencing high demand",
  "rate.?limit",
  "too many requests",
  "429",
  "500",
  "502",
  "503",
  "504",
  "520",
  "524",
  "service.?unavailable",
  "server.?error",
  "internal.?error",
  // Wrapper/provider text for transient upstream failures, including OpenRouter
  // "Provider returned error" responses (#2264).
  "provider.?returned.?error",
  "exceeded request buffer limit while retrying upstream",
  // Network, proxy, and fetch transport failures. This includes OpenAI Codex
  // raw-fetch failures such as "upstream connect", "connection refused", and
  // "reset before headers" (#733), plus OpenRouter connection drops (#3317).
  "network.?error",
  "connection.?error",
  "connection.?refused",
  "connection.?lost",
  "other side closed",
  "fetch failed",
  "getaddrinfo",
  "ENOTFOUND",
  "EAI_AGAIN",
  "upstream.?connect",
  "reset before headers",
  "socket hang up",
  "socket connection was closed",
  "timed? out",
  "timeout",
  "terminated",
  // WebSocket transports can report close/error text instead of HTTP/fetch text.
  "websocket.?closed",
  "websocket.?error",
  // Premature stream endings from SDKs and transports. Anthropic can throw
  // "stream ended without ..." and "Anthropic stream ended before message_stop"
  // (#4433); Bedrock/Smithy can throw an HTTP/2 no-response error (#3594).
  "ended without",
  "stream ended before message_stop",
  "stream ended before a terminal response event",
  "http2 request did not get a response",
  // Provider-requested retry delay cap failures should flow through the outer
  // retry policy so callers can surface/abort the backoff (#1123).
  "retry delay",
  // Explicit retry guidance emitted mid-stream by OpenAI Responses and Bedrock
  // stream exceptions (#6019).
  "you can retry your request",
  "try your request again",
  "please retry your request",
  // gRPC based providers (e.g. NVIDIA NIM)
  "ResourceExhausted",
  // Sign in with ChatGPT: usage or user data temporarily unavailable. Usage
  // failures can arrive mid-stream without an HTTP 503 in the message.
  "subscription_sharing_usage_unavailable",
  "subscription_sharing_user_unavailable"
]);
var DEFAULT_MAX_AGENT_RETRY_DELAY_MS = 6e4;
function retryDelayMs(policy, attempt) {
  const delay = policy.baseDelayMs * 2 ** Math.max(0, attempt - 1);
  const safeDelay = Number.isSafeInteger(delay) ? delay : Number.MAX_SAFE_INTEGER;
  return Math.min(safeDelay, policy.maxAgentDelayMs ?? DEFAULT_MAX_AGENT_RETRY_DELAY_MS);
}
var RetrySleepAbortError = class extends Error {
  constructor() {
    super("Aborted");
  }
};
function sleep2(ms, signal) {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(new RetrySleepAbortError());
      return;
    }
    const timeout = setTimeout(resolve, ms);
    signal?.addEventListener("abort", () => {
      clearTimeout(timeout);
      reject(new RetrySleepAbortError());
    }, { once: true });
  });
}
async function retryAssistantCall(produce, policy, signal, callbacks) {
  const maxAttempts = policy?.enabled ? policy.maxRetries : 0;
  let attempt = 0;
  let lastRetry;
  for (; ; ) {
    const response = await produce();
    if (response.stopReason === "aborted") {
      if (lastRetry)
        await callbacks?.onRetryFinished?.(false, lastRetry.attempt);
      return response;
    }
    if (response.stopReason !== "error") {
      if (lastRetry)
        await callbacks?.onRetryFinished?.(true, lastRetry.attempt);
      return response;
    }
    if (attempt >= maxAttempts || !isRetryableAssistantError(response)) {
      if (lastRetry)
        await callbacks?.onRetryFinished?.(false, lastRetry.attempt, response.errorMessage);
      return response;
    }
    attempt++;
    lastRetry = { attempt, errorMessage: response.errorMessage || "Unknown error" };
    const delayMs = retryDelayMs(policy, attempt);
    await callbacks?.onRetryScheduled?.(attempt, maxAttempts, delayMs, lastRetry.errorMessage);
    try {
      await sleep2(delayMs, signal);
    } catch (error) {
      await callbacks?.onRetryFinished?.(false, attempt, lastRetry.errorMessage);
      if (error instanceof RetrySleepAbortError) {
        const { errorMessage: _errorMessage, ...rest } = response;
        return { ...rest, stopReason: "aborted" };
      }
      throw error;
    }
    await callbacks?.onRetryAttemptStart?.();
  }
}
function isRetryableAssistantError(message) {
  if (message.stopReason !== "error" || !message.errorMessage)
    return false;
  const errorMessage = message.errorMessage;
  if (NON_RETRYABLE_PROVIDER_LIMIT_ERROR_PATTERN.test(errorMessage))
    return false;
  return RETRYABLE_PROVIDER_ERROR_PATTERN.test(errorMessage);
}

// target/vendor-pi/pi-ai/dist/utils/typebox-helpers.js
function StringEnum(values, options) {
  return typebox_exports.Unsafe({
    type: "string",
    enum: values,
    ...options?.description && { description: options.description },
    ...options?.default && { default: options.default }
  });
}

// target/vendor-pi/pi-ai/dist/utils/uuid.js
var MAX_UUID_V7_TIMESTAMP = 281474976710655;
var MAX_SEQUENCE = (1n << 41n) - 1n;
var lastOrdinaryTimestamp = -1;
var sequence;
function uuidv7(timestampMs) {
  const requestedTimestamp = timestampMs ?? Date.now();
  if (!Number.isInteger(requestedTimestamp) || requestedTimestamp < 0 || requestedTimestamp > MAX_UUID_V7_TIMESTAMP) {
    throw new RangeError(`UUIDv7 timestamp must be an integer between 0 and ${MAX_UUID_V7_TIMESTAMP}`);
  }
  const effectiveTimestamp = timestampMs === void 0 ? Math.max(requestedTimestamp, lastOrdinaryTimestamp) : timestampMs;
  if (timestampMs === void 0)
    lastOrdinaryTimestamp = effectiveTimestamp;
  const bytes = new Uint8Array(16);
  globalThis.crypto.getRandomValues(bytes);
  if (sequence === void 0) {
    sequence = BigInt(bytes[1]) << 32n | BigInt(bytes[2]) << 24n | BigInt(bytes[3]) << 16n | BigInt(bytes[4]) << 8n | BigInt(bytes[5]);
  } else {
    if (sequence === MAX_SEQUENCE)
      throw new RangeError("UUIDv7 generator sequence exhausted");
    sequence++;
  }
  const timestamp = BigInt(effectiveTimestamp);
  for (let index = 5; index >= 0; index--) {
    bytes[index] = Number(timestamp >> BigInt((5 - index) * 8)) & 255;
  }
  bytes[6] = 112 | Number(sequence >> 37n & 0x0fn);
  bytes[7] = Number(sequence >> 29n & 0xffn);
  bytes[8] = 128 | Number(sequence >> 23n & 0x3fn);
  bytes[9] = Number(sequence >> 15n & 0xffn);
  bytes[10] = Number(sequence >> 7n & 0xffn);
  bytes[11] = Number((sequence & 0x7fn) << 1n) | bytes[11] & 1;
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0"));
  return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex.slice(6, 8).join("")}-${hex.slice(8, 10).join("")}-${hex.slice(10).join("")}`;
}

// target/vendor-pi/pi-ai/dist/api/cloudflare.js
var CLOUDFLARE_WORKERS_AI_BASE_URL = "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1";
var CLOUDFLARE_WORKERS_AI_REST_BASE_URL = "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai";
var CLOUDFLARE_AI_GATEWAY_COMPAT_BASE_URL = "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/compat";
var CLOUDFLARE_AI_GATEWAY_OPENAI_BASE_URL = "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai";
var CLOUDFLARE_AI_GATEWAY_ANTHROPIC_BASE_URL = "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic";

// target/vendor-pi/pi-ai/dist/api/cloudflare-ai-binding.js
var CLOUDFLARE_GATEWAY_BINDING_AUTH_SENTINEL = "cloudflare-gateway-binding";
function createAiBindingFetch(binding) {
  if (typeof binding.fetch !== "function") {
    throw new TypeError("createAiBindingFetch: the AI binding does not expose fetch()");
  }
  const bindingFetch = binding.fetch.bind(binding);
  return (input, init) => bindingFetch(input, init);
}

// target/vendor-pi/pi-ai/dist/api/cloudflare-workers-ai-system-one.lazy.js
var cloudflareWorkersAISystemOneApi = () => ({
  classify: async (model, context, options) => (await import("./chunk-3ACNL4L5.mjs")).classify(model, context, options)
});

// xtask/vendor-pi/pi-ai/sdk/google-genai.mjs
var FinishReason = {
  FINISH_REASON_UNSPECIFIED: "FINISH_REASON_UNSPECIFIED",
  STOP: "STOP",
  MAX_TOKENS: "MAX_TOKENS",
  SAFETY: "SAFETY",
  RECITATION: "RECITATION",
  LANGUAGE: "LANGUAGE",
  OTHER: "OTHER",
  BLOCKLIST: "BLOCKLIST",
  PROHIBITED_CONTENT: "PROHIBITED_CONTENT",
  SPII: "SPII",
  MALFORMED_FUNCTION_CALL: "MALFORMED_FUNCTION_CALL",
  IMAGE_SAFETY: "IMAGE_SAFETY",
  UNEXPECTED_TOOL_CALL: "UNEXPECTED_TOOL_CALL",
  TOO_MANY_TOOL_CALLS: "TOO_MANY_TOOL_CALLS",
  IMAGE_PROHIBITED_CONTENT: "IMAGE_PROHIBITED_CONTENT",
  NO_IMAGE: "NO_IMAGE",
  IMAGE_RECITATION: "IMAGE_RECITATION",
  IMAGE_OTHER: "IMAGE_OTHER"
};
var FunctionCallingConfigMode = {
  MODE_UNSPECIFIED: "MODE_UNSPECIFIED",
  AUTO: "AUTO",
  ANY: "ANY",
  NONE: "NONE",
  VALIDATED: "VALIDATED"
};
var ThinkingLevel = {
  THINKING_LEVEL_UNSPECIFIED: "THINKING_LEVEL_UNSPECIFIED",
  MINIMAL: "MINIMAL",
  LOW: "LOW",
  MEDIUM: "MEDIUM",
  HIGH: "HIGH"
};

// target/vendor-pi/pi-ai/dist/api/google-shared.js
var GOOGLE_SDK_THINKING_LEVEL_MAP = {
  THINKING_LEVEL_UNSPECIFIED: ThinkingLevel.THINKING_LEVEL_UNSPECIFIED,
  MINIMAL: ThinkingLevel.MINIMAL,
  LOW: ThinkingLevel.LOW,
  MEDIUM: ThinkingLevel.MEDIUM,
  HIGH: ThinkingLevel.HIGH
};
function resolveGoogleThinkingLevel(model, level) {
  const mapped = model.thinkingLevelMap?.[level];
  const resolvedLevel = typeof mapped === "string" ? mapped.toLowerCase() : level;
  switch (resolvedLevel) {
    case "minimal":
    case "low":
    case "medium":
    case "high":
      return resolvedLevel;
    default:
      throw new Error(`Unsupported Google thinking level mapping for ${model.provider}/${model.id}: ${level} -> ${String(mapped)}`);
  }
}
function usesGoogleThinkingLevel(model) {
  const id = model.id.toLowerCase();
  return (
    // Match Gemini 3 Pro/Flash IDs with or without a minor version, such as
    // gemini-3-flash-preview, gemini-3.1-pro-preview, and gemini-3.8-flash.
    /gemini-3(?:\.\d+)?-(?:pro|flash)/.test(id) || id === "gemini-flash-latest" || id === "gemini-flash-lite-latest" || // Match both hosted Gemma 4 naming forms: gemma-4-* and gemma4-*.
    /gemma-?4/.test(id)
  );
}
function toGoogleThinkingLevel(level) {
  switch (level) {
    case "minimal":
      return "MINIMAL";
    case "low":
      return "LOW";
    case "medium":
      return "MEDIUM";
    case "high":
      return "HIGH";
  }
}
function toGoogleSdkThinkingLevel(level) {
  return GOOGLE_SDK_THINKING_LEVEL_MAP[level];
}
function getDisabledGoogleThinkingConfig(model) {
  if (!usesGoogleThinkingLevel(model))
    return { thinkingBudget: 0 };
  const fallback = clampThinkingLevel(model, "off");
  if (fallback === "off")
    return { thinkingBudget: 0 };
  const resolvedLevel = resolveGoogleThinkingLevel(model, fallback);
  const apiLevel = toGoogleThinkingLevel(resolvedLevel);
  return { thinkingLevel: toGoogleSdkThinkingLevel(apiLevel) };
}
function isThinkingPart(part) {
  return part.thought === true;
}
function retainThoughtSignature(existing, incoming) {
  if (typeof incoming === "string" && incoming.length > 0)
    return incoming;
  return existing;
}
var base64SignaturePattern = /^[A-Za-z0-9+/]+={0,2}$/;
function isValidThoughtSignature(signature) {
  if (!signature)
    return false;
  if (signature.length % 4 !== 0)
    return false;
  return base64SignaturePattern.test(signature);
}
function resolveThoughtSignature(isSameProviderAndModel, signature) {
  return isSameProviderAndModel && isValidThoughtSignature(signature) ? signature : void 0;
}
function requiresToolCallId(modelId) {
  const geminiMajorVersion = getGeminiMajorVersion(modelId);
  return modelId.startsWith("claude-") || modelId.startsWith("gpt-oss-") || geminiMajorVersion !== void 0 && geminiMajorVersion >= 3;
}
function getGeminiMajorVersion(modelId) {
  const match = modelId.toLowerCase().match(/^gemini(?:-live)?-(\d+)/);
  if (!match)
    return void 0;
  return Number.parseInt(match[1], 10);
}
function supportsMultimodalFunctionResponse(modelId) {
  const geminiMajorVersion = getGeminiMajorVersion(modelId);
  if (geminiMajorVersion !== void 0) {
    return geminiMajorVersion >= 3;
  }
  return true;
}
function convertMessages(model, context) {
  const conversation = withoutInitialSystemMessage(collapseSystemMessages(context).messages);
  const contents = [];
  const normalizeToolCallId = (id) => {
    if (!requiresToolCallId(model.id))
      return id;
    return id.replace(/[^a-zA-Z0-9_-]/g, "_").slice(0, 64);
  };
  const transformedMessages = transformMessages(conversation, model, normalizeToolCallId);
  for (const msg of transformedMessages) {
    if (msg.role === "user") {
      if (typeof msg.content === "string") {
        contents.push({
          role: "user",
          parts: [{ text: sanitizeSurrogates(msg.content) }]
        });
      } else {
        const parts = msg.content.map((item) => {
          if (item.type === "text") {
            return { text: sanitizeSurrogates(item.text) };
          } else {
            return {
              inlineData: {
                mimeType: item.mimeType,
                data: item.data
              }
            };
          }
        });
        if (parts.length === 0)
          continue;
        contents.push({
          role: "user",
          parts
        });
      }
    } else if (msg.role === "assistant") {
      const parts = [];
      const isSameProviderAndModel = msg.provider === model.provider && msg.model === model.id;
      for (const block of msg.content) {
        if (block.type === "text") {
          const thoughtSignature = resolveThoughtSignature(isSameProviderAndModel, block.textSignature);
          if ((!block.text || block.text.trim() === "") && !thoughtSignature)
            continue;
          parts.push({
            text: sanitizeSurrogates(block.text),
            ...thoughtSignature && { thoughtSignature }
          });
        } else if (block.type === "thinking") {
          if (isSameProviderAndModel) {
            const thoughtSignature = resolveThoughtSignature(isSameProviderAndModel, block.thinkingSignature);
            if ((!block.thinking || block.thinking.trim() === "") && !thoughtSignature)
              continue;
            parts.push({
              thought: true,
              text: sanitizeSurrogates(block.thinking),
              ...thoughtSignature && { thoughtSignature }
            });
          } else {
            if (!block.thinking || block.thinking.trim() === "")
              continue;
            parts.push({
              text: sanitizeSurrogates(block.thinking)
            });
          }
        } else if (block.type === "toolCall") {
          const thoughtSignature = resolveThoughtSignature(isSameProviderAndModel, block.thoughtSignature);
          const part = {
            functionCall: {
              name: block.name,
              args: block.arguments ?? {},
              ...requiresToolCallId(model.id) ? { id: block.id } : {}
            },
            ...thoughtSignature && { thoughtSignature }
          };
          parts.push(part);
        }
      }
      if (parts.length === 0)
        continue;
      contents.push({
        role: "model",
        parts
      });
    } else if (msg.role === "toolResult") {
      const textContent = msg.content.filter((c) => c.type === "text");
      const textResult = textContent.map((c) => c.text).join("\n");
      const imageContent = model.input.includes("image") ? msg.content.filter((c) => c.type === "image") : [];
      const hasText = textResult.length > 0;
      const hasImages = imageContent.length > 0;
      const modelSupportsMultimodalFunctionResponse = supportsMultimodalFunctionResponse(model.id);
      const responseValue = hasText ? sanitizeSurrogates(textResult) : hasImages ? "(see attached image)" : "";
      const imageParts = imageContent.map((imageBlock) => ({
        inlineData: {
          mimeType: imageBlock.mimeType,
          data: imageBlock.data
        }
      }));
      const includeId = requiresToolCallId(model.id);
      const functionResponsePart = {
        functionResponse: {
          name: msg.toolName,
          response: msg.isError ? { error: responseValue } : { output: responseValue },
          ...hasImages && modelSupportsMultimodalFunctionResponse && { parts: imageParts },
          ...includeId ? { id: msg.toolCallId } : {}
        }
      };
      const lastContent = contents[contents.length - 1];
      if (lastContent?.role === "user" && lastContent.parts?.some((p) => p.functionResponse)) {
        lastContent.parts.push(functionResponsePart);
      } else {
        contents.push({
          role: "user",
          parts: [functionResponsePart]
        });
      }
      if (hasImages && !modelSupportsMultimodalFunctionResponse) {
        contents.push({
          role: "user",
          parts: [{ text: "Tool result image:" }, ...imageParts]
        });
      }
    }
  }
  return contents;
}
var JSON_SCHEMA_META_DECLARATIONS = /* @__PURE__ */ new Set([
  "$schema",
  "$id",
  "$anchor",
  "$dynamicAnchor",
  "$vocabulary",
  "$comment",
  "$defs",
  "definitions"
  // pre-draft-2019-09 equivalent of $defs
]);
function sanitizeForOpenApi(schema) {
  if (typeof schema !== "object" || schema === null || Array.isArray(schema)) {
    return schema;
  }
  const result = {};
  for (const [key, value] of Object.entries(schema)) {
    if (JSON_SCHEMA_META_DECLARATIONS.has(key))
      continue;
    result[key] = sanitizeForOpenApi(value);
  }
  return result;
}
function convertTools(tools, useParameters = false, supportsStrictMode = true) {
  if (tools.length === 0)
    return void 0;
  return [
    {
      functionDeclarations: tools.map((tool) => {
        const strict = resolveJsonSchemaStrictSampling(tool, supportsStrictMode);
        const parameters = getJsonSchemaToolParameters(tool, strict);
        return {
          name: tool.name,
          description: tool.description,
          ...useParameters ? { parameters: sanitizeForOpenApi(parameters) } : { parametersJsonSchema: parameters }
        };
      })
    }
  ];
}
function supportsGoogleStrictToolSampling(modelId) {
  const majorVersion = getGeminiMajorVersion(modelId);
  return majorVersion !== void 0 && majorVersion >= 3;
}
function mapToolChoice(choice) {
  switch (choice) {
    case "auto":
      return FunctionCallingConfigMode.AUTO;
    case "none":
      return FunctionCallingConfigMode.NONE;
    case "any":
      return FunctionCallingConfigMode.ANY;
    default:
      return FunctionCallingConfigMode.AUTO;
  }
}
function resolveGoogleFunctionCallingMode(tools, toolChoice, supportsStrictMode) {
  const useStrictMode = tools.some((tool) => resolveJsonSchemaStrictSampling(tool, supportsStrictMode) === true);
  if (toolChoice === "none" || toolChoice === "any") {
    return mapToolChoice(toolChoice);
  }
  if (useStrictMode) {
    return FunctionCallingConfigMode.VALIDATED;
  }
  return toolChoice ? mapToolChoice(toolChoice) : void 0;
}
function mapStopReason(reason) {
  switch (reason) {
    case FinishReason.STOP:
      return "stop";
    case FinishReason.MAX_TOKENS:
      return "length";
    case FinishReason.BLOCKLIST:
    case FinishReason.PROHIBITED_CONTENT:
    case FinishReason.SPII:
    case FinishReason.SAFETY:
    case FinishReason.IMAGE_SAFETY:
    case FinishReason.IMAGE_PROHIBITED_CONTENT:
    case FinishReason.IMAGE_RECITATION:
    case FinishReason.IMAGE_OTHER:
    case FinishReason.RECITATION:
    case FinishReason.FINISH_REASON_UNSPECIFIED:
    case FinishReason.OTHER:
    case FinishReason.LANGUAGE:
    case FinishReason.MALFORMED_FUNCTION_CALL:
    case FinishReason.UNEXPECTED_TOOL_CALL:
    case FinishReason.TOO_MANY_TOOL_CALLS:
    case FinishReason.NO_IMAGE:
      return "error";
    default: {
      const _exhaustive = reason;
      throw new Error(`Unhandled stop reason: ${_exhaustive}`);
    }
  }
}
function mapStopReasonString(reason) {
  switch (reason) {
    case "STOP":
      return "stop";
    case "MAX_TOKENS":
      return "length";
    default:
      return "error";
  }
}
function retryGoogleRequest(request, options) {
  return retryProviderRequest(async () => {
    try {
      return await request();
    } catch (error) {
      if (error instanceof Error && "status" in error && !("headers" in error)) {
        error.headers = void 0;
      }
      throw error;
    }
  }, {
    maxRetries: options?.maxRetries,
    maxRetryDelayMs: options?.maxRetryDelayMs,
    signal: options?.signal
  });
}

// target/vendor-pi/pi-ai/dist/api/llama-cpp-classify.lazy.js
var llamaCppClassifyApi = () => ({
  classify: async (model, context, options) => (await import("./chunk-5ZVF3775.mjs")).classify(model, context, options)
});

// target/vendor-pi/pi-ai/dist/api/openai-responses-shared.js
function encodeTextSignatureV1(id, phase) {
  const payload = { v: 1, id };
  if (phase)
    payload.phase = phase;
  return JSON.stringify(payload);
}
function parseTextSignature(signature) {
  if (!signature)
    return void 0;
  if (signature.startsWith("{")) {
    try {
      const parsed = JSON.parse(signature);
      if (parsed.v === 1 && typeof parsed.id === "string") {
        if (parsed.phase === "commentary" || parsed.phase === "final_answer") {
          return { id: parsed.id, phase: parsed.phase };
        }
        return { id: parsed.id };
      }
    } catch {
    }
  }
  return { id: signature };
}
function convertToolResultOutput(model, content) {
  const textResult = content.filter((c) => c.type === "text").map((c) => c.text).join("\n");
  const images = content.filter((c) => c.type === "image");
  const hasText = textResult.length > 0;
  if (images.length === 0 || !model.input.includes("image")) {
    return sanitizeSurrogates(hasText ? textResult : images.length > 0 ? "(see attached image)" : "(no tool output)");
  }
  const output = [];
  if (hasText) {
    output.push({ type: "input_text", text: sanitizeSurrogates(textResult) });
  }
  for (const image of images) {
    output.push({
      type: "input_image",
      detail: "auto",
      image_url: `data:${image.mimeType};base64,${image.data}`
    });
  }
  return output;
}
function convertResponsesMessages(model, context, allowedToolCallProviders, options) {
  const normalizedContext = resolveTranscript(context, options?.supportsMidConvoSystemMessages);
  const messages = [];
  const normalizeIdPart = (part) => {
    const sanitized = part.replace(/[^a-zA-Z0-9_-]/g, "_");
    const normalized = sanitized.length > 64 ? sanitized.slice(0, 64) : sanitized;
    return normalized.replace(/_+$/, "");
  };
  const buildForeignResponsesItemId = (itemId) => {
    const normalized = `fc_${shortHash(itemId)}`;
    return normalized.length > 64 ? normalized.slice(0, 64) : normalized;
  };
  const normalizeToolCallId = (id, _targetModel, source) => {
    if (!allowedToolCallProviders.has(model.provider))
      return normalizeIdPart(id);
    if (!id.includes("|"))
      return normalizeIdPart(id);
    const [callId, itemId] = id.split("|");
    const normalizedCallId = normalizeIdPart(callId);
    const isForeignToolCall = source.provider !== model.provider || source.api !== model.api;
    let normalizedItemId = isForeignToolCall ? buildForeignResponsesItemId(itemId) : normalizeIdPart(itemId);
    if (!normalizedItemId.startsWith("fc_")) {
      normalizedItemId = normalizeIdPart(`fc_${normalizedItemId}`);
    }
    return `${normalizedCallId}|${normalizedItemId}`;
  };
  const transformedMessages = transformMessages(normalizedContext.messages, model, normalizeToolCallId);
  const transcriptTools = resolveTranscriptTools(normalizedContext.messages, (options?.supportsAdditionalTools ?? false) || (options?.supportsToolSearch ?? false));
  const appendSystemToolAdditions = (message, seed) => {
    const tools = transcriptTools.anchorsAdditions ? message.toolsAdded ?? [] : [];
    if (tools.length === 0)
      return;
    if (options?.supportsAdditionalTools) {
      messages.push({
        type: "additional_tools",
        role: "developer",
        tools: convertResponsesTools(tools, options.toolOptions)
      });
      return;
    }
    if (!options?.supportsToolSearch)
      return;
    const names = tools.map((tool) => tool.name);
    const callId = `pi_tool_load_${shortHash(`${seed}:${names.join(",")}`)}`;
    messages.push({
      type: "tool_search_call",
      call_id: callId,
      execution: "client",
      status: "completed",
      arguments: { query: names.join(" "), limit: names.length }
    });
    messages.push({
      type: "tool_search_output",
      call_id: callId,
      execution: "client",
      status: "completed",
      tools: convertResponsesTools(tools, { ...options.toolOptions, toolSearchResult: true })
    });
  };
  const includeInitialSystemMessage = options?.includeSystemPrompt ?? true;
  const compat = model.compat;
  const instructionRole = model.reasoning && compat?.supportsDeveloperRole !== false ? "developer" : "system";
  let msgIndex = 0;
  let sourceIndex = 0;
  for (const msg of transformedMessages) {
    const isLeadingSystemMessage = sourceIndex++ === 0 && msg.role === "system";
    if (msg.role === "system") {
      if (!isLeadingSystemMessage)
        appendSystemToolAdditions(msg, `system:${msgIndex}`);
      if (!isLeadingSystemMessage || includeInitialSystemMessage) {
        const text = isLeadingSystemMessage ? getSystemMessageText(msg) : renderSystemMessageUpdate(msg);
        if (text.length > 0) {
          messages.push({ role: instructionRole, content: sanitizeSurrogates(text) });
        }
      }
    } else if (msg.role === "user") {
      if (typeof msg.content === "string") {
        messages.push({
          role: "user",
          content: [{ type: "input_text", text: sanitizeSurrogates(msg.content) }]
        });
      } else {
        const content = msg.content.map((item) => {
          if (item.type === "text") {
            return {
              type: "input_text",
              text: sanitizeSurrogates(item.text)
            };
          }
          return {
            type: "input_image",
            detail: "auto",
            image_url: `data:${item.mimeType};base64,${item.data}`
          };
        });
        if (content.length === 0)
          continue;
        messages.push({
          role: "user",
          content
        });
      }
    } else if (msg.role === "assistant") {
      const output = [];
      const assistantMsg = msg;
      const isSameProviderAndApi = assistantMsg.provider === model.provider && assistantMsg.api === model.api;
      const isSameModel = isSameProviderAndApi && assistantMsg.model === model.id;
      const isDifferentModel = isSameProviderAndApi && assistantMsg.model !== model.id;
      let textBlockIndex = 0;
      for (const block of msg.content) {
        if (block.type === "thinking") {
          if (block.thinkingSignature) {
            const reasoningItem = JSON.parse(block.thinkingSignature);
            output.push(reasoningItem);
          }
        } else if (block.type === "text") {
          const textBlock = block;
          const parsedSignature = parseTextSignature(textBlock.textSignature);
          const fallbackMessageId = textBlockIndex === 0 ? `msg_pi_${msgIndex}` : `msg_pi_${msgIndex}_${textBlockIndex}`;
          textBlockIndex++;
          let msgId = parsedSignature?.id;
          if (!msgId) {
            msgId = fallbackMessageId;
          } else if (msgId.length > 64) {
            msgId = `msg_${shortHash(msgId)}`;
          }
          output.push({
            type: "message",
            role: "assistant",
            content: [{ type: "output_text", text: sanitizeSurrogates(textBlock.text), annotations: [] }],
            status: "completed",
            id: msgId,
            phase: parsedSignature?.phase
          });
        } else if (block.type === "toolCall") {
          const toolCall = block;
          const [callId, itemIdRaw] = toolCall.id.split("|");
          const customInputProperty = options?.grammarToolInputProperties?.get(toolCall.name);
          let itemId = itemIdRaw;
          const itemIdPrefix = customInputProperty === void 0 ? "fc_" : "ctc_";
          if (isDifferentModel || !itemId?.startsWith(itemIdPrefix)) {
            itemId = void 0;
          }
          if (customInputProperty !== void 0) {
            output.push({
              type: "custom_tool_call",
              id: itemId,
              call_id: callId,
              name: toolCall.name,
              input: sanitizeSurrogates(getGrammarToolInput(toolCall.name, toolCall.arguments, customInputProperty)),
              ...isSameModel && toolCall.namespace !== void 0 ? { namespace: toolCall.namespace } : {}
            });
          } else {
            output.push({
              type: "function_call",
              id: itemId,
              call_id: callId,
              name: toolCall.name,
              arguments: JSON.stringify(toolCall.arguments),
              ...isSameModel && toolCall.namespace !== void 0 ? { namespace: toolCall.namespace } : {}
            });
          }
        }
      }
      if (output.length === 0)
        continue;
      messages.push(...output);
    } else if (msg.role === "toolResult") {
      const [callId] = msg.toolCallId.split("|");
      const output = convertToolResultOutput(model, msg.content);
      if (options?.grammarToolInputProperties?.has(msg.toolName)) {
        messages.push({
          type: "custom_tool_call_output",
          call_id: callId,
          output
        });
      } else {
        messages.push({
          type: "function_call_output",
          call_id: callId,
          output
        });
      }
    }
    if (!isLeadingSystemMessage)
      msgIndex++;
  }
  return messages;
}
function convertResponsesTools(tools, options) {
  const defaultStrict = options?.strict === void 0 ? false : options.strict;
  const supportsStrictMode = options?.supportsStrictMode ?? true;
  const supportsOpenAIGrammarTools = options?.supportsOpenAIGrammarTools ?? false;
  return tools.map((tool) => {
    const grammar = resolveGrammarConstrainedSampling(tool, supportsOpenAIGrammarTools);
    if (grammar) {
      return {
        type: "custom",
        name: tool.name,
        description: tool.description,
        format: {
          type: "grammar",
          syntax: grammar.format,
          definition: grammar.definition
        },
        ...options?.toolSearchResult ? { defer_loading: true } : {}
      };
    }
    const constrainedStrict = resolveJsonSchemaStrictSampling(tool, supportsStrictMode);
    const strict = constrainedStrict ?? defaultStrict;
    const functionTool = {
      type: "function",
      name: tool.name,
      description: tool.description,
      parameters: getJsonSchemaToolParameters(tool, strict === true),
      ...options?.toolSearchResult ? { defer_loading: true } : {}
    };
    if (supportsStrictMode) {
      functionTool.strict = strict;
    }
    return functionTool;
  });
}
function getCustomToolCallInput(block) {
  const property = block.customInput?.property;
  if (property === void 0)
    return "";
  const value = block.arguments[property];
  return typeof value === "string" ? value : "";
}
function appendCustomToolCallInput(block, nextInput, close) {
  const customInput = block.customInput;
  if (!customInput)
    return void 0;
  const delta = appendGrammarToolInputJsonDelta(customInput.jsonBuffer, customInput.property, nextInput, close);
  block.arguments = { [customInput.property]: nextInput };
  return delta;
}
async function processResponsesStream(openaiStream, output, stream3, model, options) {
  let sawTerminalResponseEvent = false;
  const outputSlots = /* @__PURE__ */ new Map();
  const reasoningBlocksById = /* @__PURE__ */ new Map();
  const applyMessagePhaseStopReason = (item) => {
    if (item.type === "message" && item.phase === "final_answer") {
      output.stopReason = "stop";
    }
  };
  const getSlot = (outputIndex, type) => {
    const slot = outputSlots.get(outputIndex);
    return slot?.type === type ? slot : void 0;
  };
  const pushToolCallDelta = (slot, delta) => {
    if (delta === void 0)
      return;
    stream3.push({
      type: "toolcall_delta",
      contentIndex: slot.contentIndex,
      delta,
      partial: output
    });
  };
  const createSlot = (outputIndex, item) => {
    if (item.type === "reasoning") {
      const block = { type: "thinking", thinking: "" };
      output.content.push(block);
      const slot = {
        type: "thinking",
        block,
        contentIndex: output.content.length - 1
      };
      outputSlots.set(outputIndex, slot);
      stream3.push({ type: "thinking_start", contentIndex: slot.contentIndex, partial: output });
      return slot;
    }
    if (item.type === "message") {
      applyMessagePhaseStopReason(item);
      const block = { type: "text", text: "" };
      output.content.push(block);
      const slot = { type: "text", block, contentIndex: output.content.length - 1 };
      outputSlots.set(outputIndex, slot);
      stream3.push({ type: "text_start", contentIndex: slot.contentIndex, partial: output });
      return slot;
    }
    if (item.type === "function_call") {
      const block = {
        type: "toolCall",
        id: `${item.call_id}|${item.id}`,
        name: item.name,
        arguments: {},
        ...item.namespace !== void 0 ? { namespace: item.namespace } : {},
        partialJson: item.arguments || ""
      };
      output.content.push(block);
      const slot = {
        type: "toolCall",
        block,
        contentIndex: output.content.length - 1
      };
      outputSlots.set(outputIndex, slot);
      stream3.push({ type: "toolcall_start", contentIndex: slot.contentIndex, partial: output });
      return slot;
    }
    if (item.type === "custom_tool_call") {
      const inputProperty = options?.grammarToolInputProperties?.get(item.name) ?? "input";
      const input = item.input || "";
      const block = {
        type: "toolCall",
        id: `${item.call_id}|${item.id}`,
        name: item.name,
        arguments: { [inputProperty]: input },
        ...item.namespace !== void 0 ? { namespace: item.namespace } : {},
        customInput: {
          property: inputProperty,
          jsonBuffer: { input: "", started: false, closed: false }
        }
      };
      output.content.push(block);
      const slot = {
        type: "toolCall",
        block,
        contentIndex: output.content.length - 1
      };
      outputSlots.set(outputIndex, slot);
      stream3.push({ type: "toolcall_start", contentIndex: slot.contentIndex, partial: output });
      return slot;
    }
    return void 0;
  };
  const getOrCreateSlot = (outputIndex, item) => {
    return outputSlots.get(outputIndex) ?? createSlot(outputIndex, item);
  };
  const backfillReasoningSignatures = (responseOutput) => {
    for (const item of responseOutput) {
      if (item.type !== "reasoning" || !item.encrypted_content)
        continue;
      const block = reasoningBlocksById.get(item.id);
      if (!block?.thinkingSignature)
        continue;
      const storedItem = JSON.parse(block.thinkingSignature);
      if (storedItem.encrypted_content)
        continue;
      block.thinkingSignature = JSON.stringify({
        ...storedItem,
        encrypted_content: item.encrypted_content
      });
    }
  };
  const finalizeResponse = (response) => {
    sawTerminalResponseEvent = true;
    backfillReasoningSignatures(response.output ?? []);
    if (response?.id) {
      output.responseId = response.id;
    }
    if (response?.usage) {
      const inputDetails = response.usage.input_tokens_details;
      const cachedTokens = inputDetails?.cached_tokens || 0;
      const cacheWriteTokens = inputDetails?.cache_write_tokens || 0;
      output.usage = {
        // OpenAI includes cached and cache-write tokens in input_tokens, so subtract both.
        input: Math.max(0, (response.usage.input_tokens || 0) - cachedTokens - cacheWriteTokens),
        output: response.usage.output_tokens || 0,
        cacheRead: cachedTokens,
        cacheWrite: cacheWriteTokens,
        reasoning: response.usage.output_tokens_details?.reasoning_tokens || 0,
        totalTokens: response.usage.total_tokens || 0,
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 }
      };
    }
    calculateCost(model, output.usage);
    if (options?.applyServiceTierPricing) {
      const serviceTier = options.resolveServiceTier ? options.resolveServiceTier(response?.service_tier, options.serviceTier) : response?.service_tier ?? options.serviceTier;
      options.applyServiceTierPricing(output.usage, serviceTier);
    }
    const status = response?.status;
    const incompleteDetails = response?.incomplete_details;
    const incompleteReason = typeof incompleteDetails?.reason === "string" ? incompleteDetails.reason : void 0;
    output.rawStopReason = incompleteReason ? `${status}.${incompleteReason}` : status;
    const mappedStop = mapStopReason2(status, incompleteReason);
    output.stopReason = mappedStop.stopReason;
    if (mappedStop.errorMessage === void 0)
      delete output.errorMessage;
    else
      output.errorMessage = mappedStop.errorMessage;
    if (output.content.some((b) => b.type === "toolCall") && output.stopReason === "stop") {
      output.stopReason = "toolUse";
    }
  };
  for await (const event of openaiStream) {
    await options?.onProviderStreamEvent?.(event, model);
    if (event.type === "response.created") {
      output.responseId = event.response.id;
    } else if (event.type === "response.output_item.added") {
      createSlot(event.output_index, event.item);
    } else if (event.type === "response.reasoning_summary_text.delta") {
      const slot = getSlot(event.output_index, "thinking");
      if (!slot)
        continue;
      slot.block.thinking += event.delta;
      stream3.push({
        type: "thinking_delta",
        contentIndex: slot.contentIndex,
        delta: event.delta,
        partial: output
      });
    } else if (event.type === "response.reasoning_summary_part.done") {
      const slot = getSlot(event.output_index, "thinking");
      if (!slot)
        continue;
      slot.block.thinking += "\n\n";
      stream3.push({
        type: "thinking_delta",
        contentIndex: slot.contentIndex,
        delta: "\n\n",
        partial: output
      });
    } else if (event.type === "response.reasoning_text.delta") {
      const slot = getSlot(event.output_index, "thinking");
      if (!slot)
        continue;
      slot.block.thinking += event.delta;
      stream3.push({
        type: "thinking_delta",
        contentIndex: slot.contentIndex,
        delta: event.delta,
        partial: output
      });
    } else if (event.type === "response.output_text.delta") {
      const slot = getSlot(event.output_index, "text");
      if (!slot)
        continue;
      slot.block.text += event.delta;
      stream3.push({
        type: "text_delta",
        contentIndex: slot.contentIndex,
        delta: event.delta,
        partial: output
      });
    } else if (event.type === "response.refusal.delta") {
      const slot = getSlot(event.output_index, "text");
      if (!slot)
        continue;
      slot.block.text += event.delta;
      stream3.push({
        type: "text_delta",
        contentIndex: slot.contentIndex,
        delta: event.delta,
        partial: output
      });
    } else if (event.type === "response.function_call_arguments.delta") {
      const slot = getSlot(event.output_index, "toolCall");
      if (!slot || slot.block.partialJson === void 0)
        continue;
      slot.block.partialJson += event.delta;
      slot.block.arguments = parseStreamingJson(slot.block.partialJson);
      pushToolCallDelta(slot, event.delta);
    } else if (event.type === "response.function_call_arguments.done") {
      const slot = getSlot(event.output_index, "toolCall");
      if (!slot || slot.block.partialJson === void 0)
        continue;
      const previousPartialJson = slot.block.partialJson;
      slot.block.partialJson = event.arguments;
      slot.block.arguments = parseStreamingJson(slot.block.partialJson);
      if (event.arguments.startsWith(previousPartialJson)) {
        const delta = event.arguments.slice(previousPartialJson.length);
        if (delta.length > 0)
          pushToolCallDelta(slot, delta);
      }
    } else if (event.type === "response.custom_tool_call_input.delta") {
      const slot = getSlot(event.output_index, "toolCall");
      if (!slot || !slot.block.customInput)
        continue;
      pushToolCallDelta(slot, appendCustomToolCallInput(slot.block, getCustomToolCallInput(slot.block) + event.delta, false));
    } else if (event.type === "response.custom_tool_call_input.done") {
      const slot = getSlot(event.output_index, "toolCall");
      if (!slot || !slot.block.customInput)
        continue;
      pushToolCallDelta(slot, appendCustomToolCallInput(slot.block, event.input, true));
    } else if (event.type === "response.output_item.done") {
      const item = event.item;
      applyMessagePhaseStopReason(item);
      const slot = getOrCreateSlot(event.output_index, item);
      if (item.type === "reasoning" && slot?.type === "thinking") {
        const summaryText = item.summary?.map((s) => s.text).join("\n\n") || "";
        const contentText2 = item.content?.map((c) => c.text).join("\n\n") || "";
        slot.block.thinking = summaryText || contentText2 || slot.block.thinking;
        slot.block.thinkingSignature = JSON.stringify(item);
        reasoningBlocksById.set(item.id, slot.block);
        stream3.push({
          type: "thinking_end",
          contentIndex: slot.contentIndex,
          content: slot.block.thinking,
          partial: output
        });
        outputSlots.delete(event.output_index);
      } else if (item.type === "message" && slot?.type === "text") {
        slot.block.text = item.content?.map((c) => c.type === "output_text" ? c.text : c.refusal).join("") || "";
        slot.block.textSignature = encodeTextSignatureV1(item.id, item.phase ?? void 0);
        stream3.push({
          type: "text_end",
          contentIndex: slot.contentIndex,
          content: slot.block.text,
          partial: output
        });
        outputSlots.delete(event.output_index);
      } else if (item.type === "function_call" && slot?.type === "toolCall" && slot.block.partialJson !== void 0) {
        slot.block.arguments = parseStreamingJson(item.arguments || slot.block.partialJson || "{}");
        if (item.namespace !== void 0)
          slot.block.namespace = item.namespace;
        delete slot.block.partialJson;
        stream3.push({
          type: "toolcall_end",
          contentIndex: slot.contentIndex,
          toolCall: slot.block,
          partial: output
        });
        outputSlots.delete(event.output_index);
      } else if (item.type === "custom_tool_call" && slot?.type === "toolCall" && slot.block.customInput) {
        pushToolCallDelta(slot, appendCustomToolCallInput(slot.block, item.input ?? getCustomToolCallInput(slot.block), true));
        if (item.namespace !== void 0)
          slot.block.namespace = item.namespace;
        delete slot.block.customInput;
        stream3.push({
          type: "toolcall_end",
          contentIndex: slot.contentIndex,
          toolCall: slot.block,
          partial: output
        });
        outputSlots.delete(event.output_index);
      }
    } else if (event.type === "response.completed" || event.type === "response.incomplete") {
      finalizeResponse(event.response);
    } else if (event.type === "error") {
      throw new Error(`Error Code ${event.code}: ${event.message}` || "Unknown error");
    } else if (event.type === "response.failed") {
      sawTerminalResponseEvent = true;
      output.rawStopReason = event.response?.status;
      const error = event.response?.error;
      const details = event.response?.incomplete_details;
      const msg = error ? `${error.code || "unknown"}: ${error.message || "no message"}` : details?.reason ? `incomplete: ${details.reason}` : "Unknown error (no error details in response)";
      throw new Error(msg);
    }
  }
  if (!sawTerminalResponseEvent) {
    throw new Error("OpenAI Responses stream ended before a terminal response event");
  }
  if (output.stopReason === "toolUse") {
    for (const block of output.content) {
      if (block.type !== "toolCall")
        continue;
      const toolCall = block;
      if (toolCall.partialJson !== void 0 || toolCall.customInput !== void 0) {
        throw new Error(`OpenAI Responses stream completed with an unfinished tool call: ${toolCall.name} (${toolCall.id})`);
      }
    }
  }
}
function mapStopReason2(status, incompleteReason) {
  if (!status)
    return { stopReason: "stop" };
  switch (status) {
    case "completed":
      return { stopReason: "stop" };
    case "incomplete":
      if (incompleteReason === "max_output_tokens") {
        return { stopReason: "length" };
      }
      return {
        stopReason: "error",
        errorMessage: incompleteReason ? `Response incomplete: ${incompleteReason}` : "Response incomplete without a provider reason"
      };
    case "failed":
    case "cancelled":
      return { stopReason: "error" };
    // These two are wonky ...
    case "in_progress":
    case "queued":
      return { stopReason: "stop" };
    default: {
      const _exhaustive = status;
      throw new Error(`Unhandled stop reason: ${_exhaustive}`);
    }
  }
}

// target/vendor-pi/pi-ai/dist/api/openrouter-images.lazy.js
var openrouterImagesApi = () => ({
  generateImages: async (model, context, options) => (await import("./chunk-JZ4LANIM.mjs")).generateImages(model, context, options)
});

// target/vendor-pi/pi-ai/dist/api/pi-messages.lazy.js
var piMessagesApi = () => lazyApi(() => import("./chunk-BAZ4HGG2.mjs"));

// target/vendor-pi/pi-ai/dist/api/typesafe-system-one.lazy.js
var typesafeSystemOneApi = () => ({
  classify: async (model, context, options) => (await import("./chunk-VJEXLSBC.mjs")).classify(model, context, options)
});

// target/vendor-pi/pi-ai/dist/providers/amazon-bedrock.js
var bedrockAuth = {
  name: "AWS credentials or bearer token",
  login: async (interaction) => {
    interaction.signal.throwIfAborted();
    const method = await interaction.prompt({
      type: "select",
      message: "Select Amazon Bedrock authentication method:",
      options: [
        { id: "bearer-token", label: "Bearer token" },
        { id: "aws-profile", label: "AWS profile" },
        { id: "credential-chain", label: "Existing AWS credential chain" }
      ]
    });
    interaction.signal.throwIfAborted();
    if (method === "bearer-token") {
      return {
        type: "api_key",
        key: await interaction.prompt({ type: "secret", message: "Enter Amazon Bedrock bearer token" })
      };
    }
    interaction.notify({
      type: "info",
      message: "Amazon Bedrock supports AWS profiles, IAM credentials, and role-based credentials.",
      links: [
        {
          label: "AWS credential provider chain",
          url: "https://docs.aws.amazon.com/sdkref/latest/guide/standardized-credentials.html"
        }
      ]
    });
    if (method === "aws-profile") {
      return {
        type: "api_key",
        env: { AWS_PROFILE: await interaction.prompt({ type: "text", message: "Enter AWS profile name" }) }
      };
    }
    if (method !== "credential-chain")
      throw new Error(`Unknown Amazon Bedrock auth method: ${method}`);
    await interaction.prompt({
      type: "text",
      message: "Configure AWS credentials, then press Enter to continue"
    });
    return { type: "api_key" };
  },
  resolve: async ({ ctx, credential, signal }) => {
    const env = async (name) => {
      signal.throwIfAborted();
      const value = await ctx.env(name);
      signal.throwIfAborted();
      return value;
    };
    if (credential?.key) {
      return { auth: { apiKey: credential.key }, env: credential.env, source: "stored credential" };
    }
    if (await env("AWS_BEARER_TOKEN_BEDROCK"))
      return { auth: {}, source: "AWS_BEARER_TOKEN_BEDROCK" };
    if (credential?.env?.AWS_PROFILE ?? await env("AWS_PROFILE")) {
      return {
        auth: {},
        env: credential?.env,
        source: credential?.env?.AWS_PROFILE ? "stored credential" : "AWS_PROFILE"
      };
    }
    if (await env("AWS_ACCESS_KEY_ID") && await env("AWS_SECRET_ACCESS_KEY")) {
      return { auth: {}, source: "AWS access keys" };
    }
    if (await env("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI"))
      return { auth: {}, source: "ECS task role" };
    if (await env("AWS_CONTAINER_CREDENTIALS_FULL_URI"))
      return { auth: {}, source: "ECS task role" };
    if (await env("AWS_WEB_IDENTITY_TOKEN_FILE"))
      return { auth: {}, source: "web identity token" };
    return void 0;
  }
};
function amazonBedrockProvider() {
  return createProvider({
    id: "amazon-bedrock",
    name: "Amazon Bedrock",
    auth: { apiKey: bedrockAuth },
    models: Object.values(AMAZON_BEDROCK_MODELS),
    api: bedrockConverseStreamApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/ant-ling.js
function antLingProvider() {
  return createProvider({
    id: "ant-ling",
    name: "Ant Ling",
    baseUrl: "https://api.ant-ling.com/v1",
    auth: { apiKey: envApiKeyAuth("Ant Ling API key", ["ANT_LING_API_KEY"]) },
    models: Object.values(ANT_LING_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/anthropic.js
function anthropicApiKeyAuth() {
  return {
    name: "Anthropic API key",
    login: async (interaction) => {
      interaction.signal.throwIfAborted();
      const key = await interaction.prompt({ type: "secret", message: "Enter Anthropic API key" });
      interaction.signal.throwIfAborted();
      return { type: "api_key", key };
    },
    resolve: async ({ ctx, credential, signal }) => {
      signal.throwIfAborted();
      if (credential?.key) {
        return { auth: { apiKey: credential.key }, env: credential.env, source: "stored credential" };
      }
      const authToken = await ctx.env(ANTHROPIC_AUTH_TOKEN_ENV);
      signal.throwIfAborted();
      if (authToken) {
        return {
          auth: { headers: { Authorization: `Bearer ${authToken}` } },
          source: ANTHROPIC_AUTH_TOKEN_ENV
        };
      }
      for (const envVar of [ANTHROPIC_OAUTH_TOKEN_ENV, ANTHROPIC_API_KEY_ENV]) {
        const apiKey = await ctx.env(envVar);
        signal.throwIfAborted();
        if (apiKey)
          return { auth: { apiKey }, source: envVar };
      }
      const federation = {};
      for (const envVar of [
        ANTHROPIC_FEDERATION_RULE_ID_ENV,
        ANTHROPIC_ORGANIZATION_ID_ENV,
        ANTHROPIC_IDENTITY_TOKEN_FILE_ENV
      ]) {
        const value = await ctx.env(envVar);
        signal.throwIfAborted();
        if (!value)
          return void 0;
        federation[envVar] = value;
      }
      for (const envVar of [ANTHROPIC_SERVICE_ACCOUNT_ID_ENV, ANTHROPIC_WORKSPACE_ID_ENV]) {
        const value = await ctx.env(envVar);
        signal.throwIfAborted();
        if (value)
          federation[envVar] = value;
      }
      return { auth: {}, env: federation, source: "workload identity federation" };
    }
  };
}
function anthropicProvider() {
  return createProvider({
    id: "anthropic",
    name: "Anthropic",
    baseUrl: "https://api.anthropic.com",
    auth: {
      apiKey: anthropicApiKeyAuth(),
      oauth: lazyOAuth({
        name: "Anthropic (Claude Pro/Max)",
        isSubscription: true,
        load: loadAnthropicOAuth
      })
    },
    models: Object.values(ANTHROPIC_MODELS),
    api: anthropicMessagesApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/azure-openai-responses.js
function azureOpenAIResponsesProvider() {
  return createProvider({
    id: "azure-openai-responses",
    name: "Azure OpenAI",
    auth: { apiKey: envApiKeyAuth("Azure OpenAI API key", ["AZURE_OPENAI_API_KEY"]) },
    models: Object.values(AZURE_OPENAI_RESPONSES_MODELS),
    api: azureOpenAIResponsesApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/baseten.js
function basetenProvider() {
  return createProvider({
    id: "baseten",
    name: "Baseten",
    baseUrl: "https://inference.baseten.co/v1",
    auth: { apiKey: envApiKeyAuth("Baseten API key", ["BASETEN_API_KEY"]) },
    models: Object.values(BASETEN_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/cerebras.js
function cerebrasProvider() {
  return createProvider({
    id: "cerebras",
    name: "Cerebras",
    baseUrl: "https://api.cerebras.ai/v1",
    auth: { apiKey: envApiKeyAuth("Cerebras API key", ["CEREBRAS_API_KEY"]) },
    models: Object.values(CEREBRAS_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/cloudflare-auth.js
var CLOUDFLARE_API_KEY = "CLOUDFLARE_API_KEY";
var CLOUDFLARE_ACCOUNT_ID = "CLOUDFLARE_ACCOUNT_ID";
var CLOUDFLARE_GATEWAY_ID = "CLOUDFLARE_GATEWAY_ID";
async function resolveValue(name, ctx, credential, signal) {
  const fromCredential = credential ? name === CLOUDFLARE_API_KEY ? credential.key : credential.env?.[name] : void 0;
  if (fromCredential !== void 0)
    return fromCredential;
  signal.throwIfAborted();
  const value = await ctx.env(name);
  signal.throwIfAborted();
  return value;
}
async function resolveCloudflareEnv(kind, ctx, credential, signal) {
  const apiKey = await resolveValue(CLOUDFLARE_API_KEY, ctx, credential, signal);
  const accountId = await resolveValue(CLOUDFLARE_ACCOUNT_ID, ctx, credential, signal);
  const gatewayId = kind === "ai-gateway" ? await resolveValue(CLOUDFLARE_GATEWAY_ID, ctx, credential, signal) : void 0;
  if (!apiKey || !accountId || kind === "ai-gateway" && !gatewayId)
    return void 0;
  return {
    apiKey,
    env: {
      CLOUDFLARE_ACCOUNT_ID: accountId,
      ...gatewayId ? { CLOUDFLARE_GATEWAY_ID: gatewayId } : {}
    },
    source: credential ? "stored credential" : CLOUDFLARE_API_KEY
  };
}
function cloudflareWorkersAIAuth() {
  return {
    name: "Cloudflare API key",
    login: async (interaction) => {
      const key = await interaction.prompt({ type: "secret", message: "Enter Cloudflare API key" });
      const accountId = await interaction.prompt({ type: "text", message: "Enter Cloudflare account ID" });
      return { type: "api_key", key, env: { CLOUDFLARE_ACCOUNT_ID: accountId } };
    },
    resolve: async ({ ctx, credential, signal }) => {
      const resolved = await resolveCloudflareEnv("workers-ai", ctx, credential, signal);
      if (!resolved)
        return void 0;
      return {
        auth: { apiKey: resolved.apiKey },
        env: resolved.env,
        source: resolved.source
      };
    }
  };
}
function cloudflareAIGatewayAuth() {
  return {
    name: "Cloudflare API key",
    login: async (interaction) => {
      const key = await interaction.prompt({ type: "secret", message: "Enter Cloudflare API key" });
      const accountId = await interaction.prompt({ type: "text", message: "Enter Cloudflare account ID" });
      const gatewayId = await interaction.prompt({ type: "text", message: "Enter Cloudflare AI Gateway ID" });
      return {
        type: "api_key",
        key,
        env: { CLOUDFLARE_ACCOUNT_ID: accountId, CLOUDFLARE_GATEWAY_ID: gatewayId }
      };
    },
    resolve: async ({ ctx, credential, signal }) => {
      const resolved = await resolveCloudflareEnv("ai-gateway", ctx, credential, signal);
      if (!resolved)
        return void 0;
      return {
        auth: {
          headers: {
            "cf-aig-authorization": `Bearer ${resolved.apiKey}`,
            Authorization: null,
            "x-api-key": null
          }
        },
        env: resolved.env,
        source: resolved.source
      };
    }
  };
}

// target/vendor-pi/pi-ai/dist/providers/cloudflare-stream.js
var CLOUDFLARE_ACCOUNT_ID2 = "CLOUDFLARE_ACCOUNT_ID";
var CLOUDFLARE_GATEWAY_ID2 = "CLOUDFLARE_GATEWAY_ID";
function resolveCloudflareModel(model, env) {
  if (!env)
    return model;
  const baseUrl = model.baseUrl.replaceAll(`{${CLOUDFLARE_ACCOUNT_ID2}}`, env[CLOUDFLARE_ACCOUNT_ID2] ?? `{${CLOUDFLARE_ACCOUNT_ID2}}`).replaceAll(`{${CLOUDFLARE_GATEWAY_ID2}}`, env[CLOUDFLARE_GATEWAY_ID2] ?? `{${CLOUDFLARE_GATEWAY_ID2}}`);
  return baseUrl === model.baseUrl ? model : { ...model, baseUrl };
}
function cloudflareStreams(streams) {
  return {
    stream: (model, context, options) => streams.stream(resolveCloudflareModel(model, options?.env), context, options),
    streamSimple: (model, context, options) => streams.streamSimple(resolveCloudflareModel(model, options?.env), context, options)
  };
}
function cloudflareClassifier(classifier) {
  return {
    classify: (model, context, options) => classifier.classify(resolveCloudflareModel(model, options?.env), context, options)
  };
}

// target/vendor-pi/pi-ai/dist/providers/cloudflare-ai-gateway.js
function cloudflareAIGatewayProvider() {
  return createProvider({
    id: "cloudflare-ai-gateway",
    name: "Cloudflare AI Gateway",
    auth: { apiKey: cloudflareAIGatewayAuth() },
    models: Object.values(CLOUDFLARE_AI_GATEWAY_MODELS),
    api: {
      "anthropic-messages": cloudflareStreams(anthropicMessagesApi()),
      "openai-completions": cloudflareStreams(openAICompletionsApi()),
      "openai-responses": cloudflareStreams(openAIResponsesApi())
    }
  });
}

// target/vendor-pi/pi-ai/dist/providers/cloudflare-workers-ai.js
function cloudflareWorkersAIProvider() {
  return createProvider({
    id: "cloudflare-workers-ai",
    name: "Cloudflare Workers AI",
    auth: { apiKey: cloudflareWorkersAIAuth() },
    models: [
      ...Object.values(CLOUDFLARE_WORKERS_AI_MODELS),
      ...Object.values(CLOUDFLARE_WORKERS_AI_CLASSIFIER_MODELS)
    ],
    api: cloudflareStreams(openAICompletionsApi()),
    classifiers: {
      "cloudflare-workers-ai-system-one": cloudflareClassifier(cloudflareWorkersAISystemOneApi())
    }
  });
}

// target/vendor-pi/pi-ai/dist/providers/data/.manifest.json
var manifest_default = { schemaVersion: 6, generatedAt: "2026-10-01T18:57:11.882Z", structureHash: "235f2f320916ab6b0d7193e0bf66ec7983e9bc05abeddd7264923fb1e7eaf76e", files: { "amazon-bedrock.json": "b962444f8daad1e3022d3e4bd59af8e8802d6012615397015c825996219e4af5", "ant-ling.json": "628feab3ef6c7d80ce96f75999804e1d9f05fec3851e5302e30fff3b69db90f6", "anthropic.json": "3ff00b68990382e42626ebd1b65dcc28ae3cad9e6faa8920331ba92f067f1b3a", "azure-openai-responses.json": "adeb2cf9d944d84f81b0d633c2e4e6bfa548a806c431902bd037c5f36500be21", "baseten.json": "b04d1423d6958bec857ffccc36726f091ec1a8f3a09cf0836b64cc94b0ee697d", "cerebras.json": "3eccb58c5de3d0dc7ba49dc6e477ba29f00cb3150ce0a35cc90d59e6b5ab368e", "cloudflare-ai-gateway.json": "52decade622d1d25f1af90385b1aec7331dad00d56e010a5e4d15118efae67ce", "cloudflare-workers-ai.json": "b9f76cc83756a05920a8a7046c515919a1de918331b860a211f309bb716c4bb0", "deepseek.json": "10a296fb3e898f7715c80890c8af0af4f5fd58f22dbcd6612fdc5d762157ccdc", "fireworks.json": "d6144ba451bd7a0617669527bec1898f97a633f7d7ea5035d8e03234550340c0", "github-copilot.json": "573f05c2dfc0c8158e252dd286d6b174cd323f965284d7ae157439d8e3dea92c", "google-vertex.json": "40090c680538bc867f2624d986a449f361be4dfa6ba96d7a27402050f036e009", "google.json": "c0f5a633e5e5a5044432db3e537f4bc674d4de18df09325c6349547fcc13cabe", "groq.json": "910952ad9dad07856983459d63fce0148dba1203edd7b10d801a77c5824f3405", "huggingface.json": "1b8604bd31a5e05fe43a40ec1ec6dfc1a51a28b0b020151ef057fab43f57c923", "kimi-coding.json": "582250b920454d98940da3e7e99d30b807edbea28f2eb8ac15f658d2ef41e8fc", "meta.json": "0c7e9a370a7005c5a89dc94f0f93bc67b8935c3f802de324f11af6ff56fff429", "minimax-cn.json": "f962c20ee6a6a342820f778e9dc363a18a524bf11998f6e2c57f2989915d62e5", "minimax.json": "17a432f80d0ea6dd26eade80a4314250d3a855678601728a5f70dd8fa0d21dd2", "mistral.json": "10f33bff9adf1248f7e6848e5890c265399f1f94e5b42cfdc28c109d547af98d", "moonshotai-cn.json": "79c9585dc84ee525ebaebbc78b3c55f3d369ec884d4f943488f1e0b14e3587f5", "moonshotai.json": "1da2c4e34f22aef17a07c974d85bdee8e1b68b6dc712e3769c935b3f1e7dadca", "nvidia.json": "3b4e7b799bf3eaacd1af0491eef77fe33147ea7f72a3a9fa54d77af0e7eacf4c", "openai-codex.json": "3cc35a7e122e5c77ee8ec30a9a3bd8bd59629997e85ba2cb3f9e0ed62e9a09c0", "openai.json": "87faafcdd168c9b6fdaef521137834a207437daf0d71641fd0a2f3dddf4d1ecb", "opencode-go.json": "d05b3eb87b81c6e5a94801484e668e660fe60c68f06a972b716d581c3bbd7c25", "opencode.json": "1c504731d37468cc992a88b231cde29d46f0273e2710d8af227247c58afaa1b7", "openrouter.json": "767e2ea83edbbdfcc9e9d66a2d4e9eae9b14589fb97e137daebef7958b1b0283", "qwen-token-plan-cn.json": "2469a4091c474ec3db0a49a9962d8d2726c074ad9065b96bc4e0c87861c4c6c5", "qwen-token-plan-individual.json": "0fb7cf64faf39d1a3b0a884210a86c87084b805eda15644f5f39293287b959dd", "qwen-token-plan.json": "eff10620cb577142de4ec02698d5961a71fda2fe4f6ae7ea9432bad9d6844176", "radius.json": "c8e24a65119360946749c6d5e45782d84ab2794df661b369bb27c2bff3b72e67", "together.json": "94ec3839972061405885d64f5fdbde48eb1f29a159be5e607e9770befd54e28e", "typesafe.json": "a4429a2f696d5cc5273969af61a7310bda07d6bdcd27bf9ac9c76c534cc43716", "vercel-ai-gateway.json": "b3574cebd04f1db537c2d06517caf3e184360f181b9cbc5ce1c287886dbfce77", "xai.json": "879c49b78b663a52a220ac4ee013e726b10c4f50aaa03bc58ee53a4b54c76928", "xiaomi-token-plan-ams.json": "ca65178f4525d9705bc8d085ed136cecbe149c4080e857ea6a64ae643d2ac00a", "xiaomi-token-plan-cn.json": "b065326926681c2262084e59c1c19ebdaf146c07107b5242c76b456becf74db0", "xiaomi-token-plan-sgp.json": "92c3d6d707f81fd9cd2b6d1fb57dc431d90ba51e10fe49d97bc8806035c23492", "xiaomi.json": "b81d01a374f7b558728c78d81f4168efa9c8996e6e2b5618409a8094b083c759", "zai-coding-cn.json": "ee5ee705fb6e413a6088db5e3616c27fce802a8ece6c355f1db35490d787f610", "zai.json": "58e6642fdfb736f0b9a4a277c760c536a3cea66a030abbfb2615f7b2851d5c6f" } };

// target/vendor-pi/pi-ai/dist/providers/deepseek.js
function deepseekProvider() {
  return createProvider({
    id: "deepseek",
    name: "DeepSeek",
    baseUrl: "https://api.deepseek.com",
    auth: { apiKey: envApiKeyAuth("DeepSeek API key", ["DEEPSEEK_API_KEY"]) },
    models: Object.values(DEEPSEEK_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/fireworks.js
function fireworksProvider() {
  return createProvider({
    id: "fireworks",
    name: "Fireworks",
    baseUrl: "https://api.fireworks.ai/inference",
    auth: { apiKey: envApiKeyAuth("Fireworks API key", ["FIREWORKS_API_KEY"]) },
    models: Object.values(FIREWORKS_MODELS),
    api: {
      "anthropic-messages": anthropicMessagesApi(),
      "openai-completions": openAICompletionsApi()
    }
  });
}

// target/vendor-pi/pi-ai/dist/providers/github-copilot.js
function githubCopilotProvider() {
  return createProvider({
    id: "github-copilot",
    name: "GitHub Copilot",
    baseUrl: "https://api.individual.githubcopilot.com",
    auth: {
      apiKey: envApiKeyAuth("GitHub Copilot token", ["COPILOT_GITHUB_TOKEN"]),
      oauth: lazyOAuth({ name: "GitHub Copilot", isSubscription: true, load: loadGitHubCopilotOAuth })
    },
    models: Object.values(GITHUB_COPILOT_MODELS),
    filterModels: (models, credential) => {
      if (credential?.type !== "oauth")
        return models;
      const availableModelIds = credential.availableModelIds;
      if (!Array.isArray(availableModelIds) || !availableModelIds.every((id) => typeof id === "string")) {
        return models;
      }
      const available = new Set(availableModelIds);
      return models.filter((model) => available.has(model.id));
    },
    api: {
      "anthropic-messages": anthropicMessagesApi(),
      "openai-completions": openAICompletionsApi(),
      "openai-responses": openAIResponsesApi()
    }
  });
}

// target/vendor-pi/pi-ai/dist/providers/google.js
function googleProvider() {
  return createProvider({
    id: "google",
    name: "Google",
    baseUrl: "https://generativelanguage.googleapis.com/v1beta",
    auth: { apiKey: envApiKeyAuth("Gemini API key", ["GEMINI_API_KEY"]) },
    models: Object.values(GOOGLE_MODELS),
    api: googleGenerativeAIApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/google-vertex.js
var VERTEX_ADC_PATH = "~/.config/gcloud/application_default_credentials.json";
var vertexAuth = {
  name: "Google Cloud credentials",
  login: async (interaction) => {
    interaction.signal.throwIfAborted();
    const method = await interaction.prompt({
      type: "select",
      message: "Select Google Vertex AI authentication method:",
      options: [
        { id: "api-key", label: "Google Cloud API key" },
        { id: "adc", label: "Application Default Credentials" },
        { id: "service-account", label: "Service account credentials file" }
      ]
    });
    interaction.signal.throwIfAborted();
    if (method === "api-key") {
      return {
        type: "api_key",
        key: await interaction.prompt({ type: "secret", message: "Enter Google Cloud API key" })
      };
    }
    if (method !== "adc" && method !== "service-account") {
      throw new Error(`Unknown Google Vertex AI auth method: ${method}`);
    }
    interaction.notify({
      type: "info",
      message: method === "adc" ? "Run `gcloud auth application-default login`, then provide the project and location." : "Provide a service account credentials file, project, and location.",
      links: [
        {
          label: "Application Default Credentials",
          url: "https://cloud.google.com/docs/authentication/provide-credentials-adc"
        }
      ]
    });
    const project = await interaction.prompt({ type: "text", message: "Enter Google Cloud project ID" });
    const location = await interaction.prompt({ type: "text", message: "Enter Google Cloud location" });
    const credentialsPath = method === "service-account" ? await interaction.prompt({ type: "text", message: "Enter service account credentials file path" }) : void 0;
    return {
      type: "api_key",
      env: {
        GOOGLE_CLOUD_PROJECT: project,
        GOOGLE_CLOUD_LOCATION: location,
        ...credentialsPath ? { GOOGLE_APPLICATION_CREDENTIALS: credentialsPath } : {}
      }
    };
  },
  resolve: async ({ ctx, credential, signal }) => {
    const env = async (name) => {
      signal.throwIfAborted();
      const value = await ctx.env(name);
      signal.throwIfAborted();
      return value;
    };
    const key = credential?.key ?? await env("GOOGLE_CLOUD_API_KEY");
    if (key)
      return { auth: { apiKey: key }, source: credential?.key ? "stored credential" : "GOOGLE_CLOUD_API_KEY" };
    const adcPath = credential?.env?.GOOGLE_APPLICATION_CREDENTIALS ?? await env("GOOGLE_APPLICATION_CREDENTIALS");
    signal.throwIfAborted();
    const hasCredentials = await ctx.fileExists(adcPath ?? VERTEX_ADC_PATH);
    signal.throwIfAborted();
    const project = credential?.env?.GOOGLE_CLOUD_PROJECT ?? await env("GOOGLE_CLOUD_PROJECT") ?? await env("GCLOUD_PROJECT");
    const location = credential?.env?.GOOGLE_CLOUD_LOCATION ?? await env("GOOGLE_CLOUD_LOCATION");
    if (hasCredentials && project && location) {
      return {
        auth: {},
        env: credential?.env,
        source: credential ? "stored credential" : "gcloud application default credentials"
      };
    }
    return void 0;
  }
};
function googleVertexProvider() {
  return createProvider({
    id: "google-vertex",
    name: "Google Vertex AI",
    auth: { apiKey: vertexAuth },
    models: Object.values(GOOGLE_VERTEX_MODELS),
    api: googleVertexApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/groq.js
function groqProvider() {
  return createProvider({
    id: "groq",
    name: "Groq",
    baseUrl: "https://api.groq.com/openai/v1",
    auth: { apiKey: envApiKeyAuth("Groq API key", ["GROQ_API_KEY"]) },
    models: Object.values(GROQ_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/huggingface.js
function huggingfaceProvider() {
  return createProvider({
    id: "huggingface",
    name: "Hugging Face",
    baseUrl: "https://router.huggingface.co/v1",
    auth: { apiKey: envApiKeyAuth("Hugging Face token", ["HF_TOKEN"]) },
    models: Object.values(HUGGINGFACE_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/kimi-coding.js
function kimiCodingProvider() {
  return createProvider({
    id: "kimi-coding",
    name: "Kimi For Coding",
    baseUrl: "https://api.kimi.com/coding",
    auth: {
      apiKey: envApiKeyAuth("Kimi API key", ["KIMI_API_KEY"]),
      oauth: lazyOAuth({
        name: "Kimi Code (subscription)",
        isSubscription: true,
        loginLabel: "Sign in with Kimi Code",
        load: loadKimiCodingOAuth
      })
    },
    models: Object.values(KIMI_CODING_MODELS),
    api: anthropicMessagesApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/meta.js
function metaProvider() {
  return createProvider({
    id: "meta",
    name: "Meta",
    baseUrl: "https://api.meta.ai/v1",
    auth: {
      apiKey: envApiKeyAuth("Meta Model API key", ["META_API_KEY"]),
      oauth: lazyOAuth({
        name: "Meta (Muse subscription)",
        isSubscription: true,
        loginLabel: "Sign in with Meta",
        load: loadMetaOAuth
      })
    },
    models: Object.values(META_MODELS),
    api: openAIResponsesApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/minimax.js
function minimaxProvider() {
  return createProvider({
    id: "minimax",
    name: "MiniMax",
    baseUrl: "https://api.minimax.io/anthropic",
    auth: { apiKey: envApiKeyAuth("MiniMax API key", ["MINIMAX_API_KEY"]) },
    models: Object.values(MINIMAX_MODELS),
    api: anthropicMessagesApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/minimax-cn.js
function minimaxCnProvider() {
  return createProvider({
    id: "minimax-cn",
    name: "MiniMax CN",
    baseUrl: "https://api.minimaxi.com/anthropic",
    auth: { apiKey: envApiKeyAuth("MiniMax CN API key", ["MINIMAX_CN_API_KEY"]) },
    models: Object.values(MINIMAX_CN_MODELS),
    api: anthropicMessagesApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/mistral.js
function mistralProvider() {
  return createProvider({
    id: "mistral",
    name: "Mistral",
    baseUrl: "https://api.mistral.ai",
    auth: { apiKey: envApiKeyAuth("Mistral API key", ["MISTRAL_API_KEY"]) },
    models: Object.values(MISTRAL_MODELS),
    api: mistralConversationsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/moonshotai.js
function moonshotaiProvider() {
  return createProvider({
    id: "moonshotai",
    name: "Moonshot AI",
    baseUrl: "https://api.moonshot.ai/v1",
    auth: { apiKey: envApiKeyAuth("Moonshot AI API key", ["MOONSHOT_API_KEY"]) },
    models: Object.values(MOONSHOTAI_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/moonshotai-cn.js
function moonshotaiCnProvider() {
  return createProvider({
    id: "moonshotai-cn",
    name: "Moonshot AI CN",
    baseUrl: "https://api.moonshot.cn/v1",
    auth: { apiKey: envApiKeyAuth("Moonshot AI API key", ["MOONSHOT_API_KEY"]) },
    models: Object.values(MOONSHOTAI_CN_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/nvidia.js
function nvidiaProvider() {
  return createProvider({
    id: "nvidia",
    name: "NVIDIA",
    baseUrl: "https://integrate.api.nvidia.com/v1",
    auth: { apiKey: envApiKeyAuth("NVIDIA API key", ["NVIDIA_API_KEY"]) },
    models: Object.values(NVIDIA_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/openai.js
function openaiProvider() {
  return createProvider({
    id: "openai",
    name: "OpenAI",
    baseUrl: "https://api.openai.com/v1",
    auth: {
      apiKey: envApiKeyAuth("OpenAI API key", ["OPENAI_API_KEY"]),
      oauth: lazyOAuth({
        name: "OpenAI (ChatGPT subscription)",
        isSubscription: true,
        loginLabel: "Sign in with ChatGPT",
        load: loadOpenAIChatGPTOAuth
      })
    },
    models: Object.values(OPENAI_MODELS),
    api: openAIResponsesApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/openai-codex.js
function openaiCodexProvider() {
  return createProvider({
    id: "openai-codex",
    name: "OpenAI Codex (legacy)",
    baseUrl: "https://chatgpt.com/backend-api",
    auth: {
      oauth: lazyOAuth({
        name: "OpenAI (ChatGPT Plus/Pro)",
        isSubscription: true,
        load: loadOpenAICodexOAuth
      })
    },
    models: Object.values(OPENAI_CODEX_MODELS),
    api: openAICodexResponsesApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/opencode-headers.js
var OPENCODE_SESSION_HEADER = "x-opencode-session";
function hasHeader(headers, name) {
  const expected = name.toLowerCase();
  return Object.keys(headers ?? {}).some((key) => key.toLowerCase() === expected);
}
function withSessionHeader(options) {
  if (!options?.sessionId || hasHeader(options.headers, OPENCODE_SESSION_HEADER))
    return options;
  return {
    ...options,
    headers: { ...options.headers, [OPENCODE_SESSION_HEADER]: options.sessionId }
  };
}
function withOpenCodeSessionHeader(streams) {
  return {
    ...streams,
    stream: (model, context, options) => streams.stream(model, context, withSessionHeader(options)),
    streamSimple: (model, context, options) => streams.streamSimple(model, context, withSessionHeader(options))
  };
}

// target/vendor-pi/pi-ai/dist/providers/opencode.js
function opencodeProvider() {
  return createProvider({
    id: "opencode",
    name: "OpenCode Zen",
    auth: { apiKey: envApiKeyAuth("OpenCode API key", ["OPENCODE_API_KEY"]) },
    models: [...Object.values(OPENCODE_MODELS), ...Object.values(OPENCODE_CLASSIFIER_MODELS)],
    api: {
      "anthropic-messages": withOpenCodeSessionHeader(anthropicMessagesApi()),
      "google-generative-ai": withOpenCodeSessionHeader(googleGenerativeAIApi()),
      "openai-completions": withOpenCodeSessionHeader(openAICompletionsApi()),
      "openai-responses": withOpenCodeSessionHeader(openAIResponsesApi())
    },
    // OpenCode Zen serves TypeSafe's System One protocol at /zen/v1/systemone.
    classifiers: { "typesafe-system-one": typesafeSystemOneApi() }
  });
}

// target/vendor-pi/pi-ai/dist/providers/opencode-go.js
function opencodeGoProvider() {
  return createProvider({
    id: "opencode-go",
    name: "OpenCode Go",
    auth: { apiKey: envApiKeyAuth("OpenCode API key", ["OPENCODE_API_KEY"]) },
    models: Object.values(OPENCODE_GO_MODELS),
    api: {
      "anthropic-messages": withOpenCodeSessionHeader(anthropicMessagesApi()),
      "openai-completions": withOpenCodeSessionHeader(openAICompletionsApi()),
      "openai-responses": withOpenCodeSessionHeader(openAIResponsesApi())
    }
  });
}

// target/vendor-pi/pi-ai/dist/providers/openrouter.js
function openrouterProvider() {
  return createProvider({
    id: "openrouter",
    name: "OpenRouter",
    baseUrl: "https://openrouter.ai/api/v1",
    auth: {
      apiKey: envApiKeyAuth("OpenRouter API key", ["OPENROUTER_API_KEY"]),
      oauth: lazyOAuth({
        name: "OpenRouter OAuth",
        loginLabel: "Sign in with OpenRouter",
        load: loadOpenRouterOAuth
      })
    },
    models: [
      ...Object.values(OPENROUTER_MODELS),
      ...Object.values(OPENROUTER_IMAGE_MODELS),
      ...Object.values(OPENROUTER_CLASSIFIER_MODELS)
    ],
    api: {
      "anthropic-messages": anthropicMessagesApi(),
      "openai-completions": openAICompletionsApi()
    },
    images: { "openrouter-images": openrouterImagesApi() },
    // OpenRouter serves TypeSafe's System One protocol at /api/v1/systemone.
    classifiers: { "typesafe-system-one": typesafeSystemOneApi() }
  });
}

// target/vendor-pi/pi-ai/dist/providers/qwen-token-plan.js
function qwenTokenPlanProvider() {
  return createProvider({
    id: "qwen-token-plan",
    name: "Qwen Token Plan",
    baseUrl: "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
    auth: { apiKey: envApiKeyAuth("Qwen Token Plan API key", ["QWEN_TOKEN_PLAN_API_KEY"]) },
    models: Object.values(QWEN_TOKEN_PLAN_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/qwen-token-plan-cn.js
function qwenTokenPlanCnProvider() {
  return createProvider({
    id: "qwen-token-plan-cn",
    name: "Qwen Token Plan CN",
    baseUrl: "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
    auth: { apiKey: envApiKeyAuth("Qwen Token Plan CN API key", ["QWEN_TOKEN_PLAN_CN_API_KEY"]) },
    models: Object.values(QWEN_TOKEN_PLAN_CN_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/qwen-token-plan-individual.js
function qwenTokenPlanIndividualProvider() {
  return createProvider({
    id: "qwen-token-plan-individual",
    name: "Qwen Token Plan Individual",
    baseUrl: "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
    auth: { apiKey: envApiKeyAuth("Qwen Token Plan Individual API key", ["QWEN_TOKEN_PLAN_API_KEY"]) },
    models: Object.values(QWEN_TOKEN_PLAN_INDIVIDUAL_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/radius.js
function radiusProvider(options = {}) {
  const id = options.id ?? "radius";
  const name = options.name ?? "Radius";
  const gateway = normalizeRadiusGatewayUrl(options.gateway ?? DEFAULT_RADIUS_GATEWAY);
  const baselineModels = gateway === normalizeRadiusGatewayUrl(DEFAULT_RADIUS_GATEWAY) ? Object.values(RADIUS_MODELS).map((model) => ({ ...model, provider: id })) : [];
  let dynamicModels = getRadiusModels(id, void 0);
  const streams = piMessagesApi();
  return {
    id,
    name,
    auth: {
      apiKey: envApiKeyAuth("Radius API key", ["RADIUS_API_KEY"]),
      oauth: lazyOAuth({ name, load: () => loadRadiusOAuth({ name, gateway }) })
    },
    getModels: () => {
      const merged = [...baselineModels];
      for (const model of dynamicModels) {
        const index = merged.findIndex((entry) => entry.id === model.id);
        if (index >= 0)
          merged[index] = model;
        else
          merged.push(model);
      }
      return merged;
    },
    refreshModels: async (context) => {
      const stored = context.stored;
      if (stored) {
        const restored = stored.models.filter((model) => model.provider === id);
        if (!await context.publish({
          update: () => {
            dynamicModels = restored;
          }
        })) {
          return;
        }
      }
      if (!stored && context.credential?.type === "oauth") {
        const legacy = getRadiusModels(id, context.credential);
        if (legacy.length > 0) {
          if (!await context.publish({
            persist: { models: legacy, checkedAt: Date.now() },
            update: () => {
              dynamicModels = legacy;
            }
          })) {
            return;
          }
        }
      }
      if (!context.allowNetwork || context.signal.aborted)
        return;
      const apiKey = context.credential?.type === "oauth" ? context.credential.access : context.credential?.key;
      const config = await loadRadiusGatewayConfig(gateway, apiKey, context.signal);
      if (context.signal.aborted)
        return;
      const refreshed = getRadiusModelsFromConfig(id, config);
      await context.publish({
        persist: { models: refreshed, checkedAt: Date.now() },
        update: () => {
          dynamicModels = refreshed;
        }
      });
    },
    stream: (model, context, streamOptions) => streams.stream(model, context, streamOptions),
    streamSimple: (model, context, streamOptions) => streams.streamSimple(model, context, streamOptions)
  };
}

// target/vendor-pi/pi-ai/dist/providers/together.js
function togetherProvider() {
  return createProvider({
    id: "together",
    name: "Together",
    baseUrl: "https://api.together.ai/v1",
    auth: { apiKey: envApiKeyAuth("Together API key", ["TOGETHER_API_KEY"]) },
    models: Object.values(TOGETHER_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/typesafe.js
function typesafeProvider() {
  return createProvider({
    id: "typesafe",
    name: "TypeSafe",
    auth: {
      apiKey: envApiKeyAuth("TypeSafe API key", ["TYPESAFE_API_KEY"])
    },
    models: Object.values(TYPESAFE_CLASSIFIER_MODELS),
    classifiers: {
      "typesafe-system-one": typesafeSystemOneApi()
    }
  });
}

// target/vendor-pi/pi-ai/dist/providers/vercel-ai-gateway.js
function vercelAIGatewayProvider() {
  return createProvider({
    id: "vercel-ai-gateway",
    name: "Vercel AI Gateway",
    baseUrl: "https://ai-gateway.vercel.sh",
    auth: { apiKey: envApiKeyAuth("Vercel AI Gateway API key", ["AI_GATEWAY_API_KEY"]) },
    models: [...Object.values(VERCEL_AI_GATEWAY_MODELS), ...Object.values(VERCEL_AI_GATEWAY_CLASSIFIER_MODELS)],
    api: anthropicMessagesApi(),
    // AI Gateway serves TypeSafe's System One protocol at /typesafe/v1/systemone.
    classifiers: { "typesafe-system-one": typesafeSystemOneApi() }
  });
}

// target/vendor-pi/pi-ai/dist/providers/xai.js
function xaiProvider() {
  return createProvider({
    id: "xai",
    name: "xAI",
    baseUrl: "https://api.x.ai/v1",
    auth: {
      apiKey: envApiKeyAuth("xAI API key", ["XAI_API_KEY"]),
      oauth: lazyOAuth({
        name: "xAI (Grok/X subscription)",
        isSubscription: true,
        loginLabel: "Sign in with SuperGrok or X Premium",
        load: loadXaiOAuth
      })
    },
    models: Object.values(XAI_MODELS),
    api: openAIResponsesApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/xiaomi.js
function xiaomiProvider() {
  return createProvider({
    id: "xiaomi",
    name: "Xiaomi",
    baseUrl: "https://api.xiaomimimo.com/v1",
    auth: { apiKey: envApiKeyAuth("Xiaomi API key", ["XIAOMI_API_KEY"]) },
    models: Object.values(XIAOMI_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/xiaomi-token-plan-ams.js
function xiaomiTokenPlanAmsProvider() {
  return createProvider({
    id: "xiaomi-token-plan-ams",
    name: "Xiaomi Token Plan AMS",
    baseUrl: "https://token-plan-ams.xiaomimimo.com/v1",
    auth: { apiKey: envApiKeyAuth("Xiaomi Token Plan AMS API key", ["XIAOMI_TOKEN_PLAN_AMS_API_KEY"]) },
    models: Object.values(XIAOMI_TOKEN_PLAN_AMS_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/xiaomi-token-plan-cn.js
function xiaomiTokenPlanCnProvider() {
  return createProvider({
    id: "xiaomi-token-plan-cn",
    name: "Xiaomi Token Plan CN",
    baseUrl: "https://token-plan-cn.xiaomimimo.com/v1",
    auth: { apiKey: envApiKeyAuth("Xiaomi Token Plan CN API key", ["XIAOMI_TOKEN_PLAN_CN_API_KEY"]) },
    models: Object.values(XIAOMI_TOKEN_PLAN_CN_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/xiaomi-token-plan-sgp.js
function xiaomiTokenPlanSgpProvider() {
  return createProvider({
    id: "xiaomi-token-plan-sgp",
    name: "Xiaomi Token Plan SGP",
    baseUrl: "https://token-plan-sgp.xiaomimimo.com/v1",
    auth: { apiKey: envApiKeyAuth("Xiaomi Token Plan SGP API key", ["XIAOMI_TOKEN_PLAN_SGP_API_KEY"]) },
    models: Object.values(XIAOMI_TOKEN_PLAN_SGP_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/zai.js
function zaiProvider() {
  return createProvider({
    id: "zai",
    name: "Z.AI",
    baseUrl: "https://api.z.ai/api/coding/paas/v4",
    auth: { apiKey: envApiKeyAuth("Z.AI API key", ["ZAI_API_KEY"]) },
    models: Object.values(ZAI_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/zai-coding-cn.js
function zaiCodingCnProvider() {
  return createProvider({
    id: "zai-coding-cn",
    name: "Z.AI Coding CN",
    baseUrl: "https://open.bigmodel.cn/api/coding/paas/v4",
    auth: { apiKey: envApiKeyAuth("Z.AI Coding CN API key", ["ZAI_CODING_CN_API_KEY"]) },
    models: Object.values(ZAI_CODING_CN_MODELS),
    api: openAICompletionsApi()
  });
}

// target/vendor-pi/pi-ai/dist/providers/all.js
function getBuiltinModel(provider, modelId) {
  return MODELS[provider]?.[modelId];
}
function getBuiltinImageModel(provider, modelId) {
  return IMAGE_MODELS[provider]?.[modelId];
}
function getBuiltinClassifierModel(provider, modelId) {
  return CLASSIFIER_MODELS[provider]?.[modelId];
}
function getBuiltinProviders() {
  return Object.keys(MODELS);
}
function getBuiltinModelDataGeneratedAt() {
  const generatedAt = Date.parse(manifest_default.generatedAt);
  return Number.isNaN(generatedAt) ? void 0 : generatedAt;
}
function getBuiltinModels(provider) {
  const models = MODELS[provider];
  return Object.values(models ?? {});
}
function getBuiltinImageModels(provider) {
  const models = IMAGE_MODELS[provider];
  return Object.values(models ?? {});
}
function getBuiltinClassifierModels(provider) {
  const models = CLASSIFIER_MODELS[provider];
  return Object.values(models ?? {});
}
function getAllBuiltinModels(provider) {
  return [...getBuiltinModels(provider), ...getBuiltinImageModels(provider), ...getBuiltinClassifierModels(provider)];
}
function builtinProviders() {
  return [
    amazonBedrockProvider(),
    antLingProvider(),
    anthropicProvider(),
    azureOpenAIResponsesProvider(),
    basetenProvider(),
    cerebrasProvider(),
    cloudflareAIGatewayProvider(),
    cloudflareWorkersAIProvider(),
    deepseekProvider(),
    fireworksProvider(),
    githubCopilotProvider(),
    googleProvider(),
    googleVertexProvider(),
    groqProvider(),
    huggingfaceProvider(),
    kimiCodingProvider(),
    metaProvider(),
    minimaxProvider(),
    minimaxCnProvider(),
    mistralProvider(),
    moonshotaiProvider(),
    moonshotaiCnProvider(),
    nvidiaProvider(),
    openaiProvider(),
    openaiCodexProvider(),
    opencodeProvider(),
    opencodeGoProvider(),
    openrouterProvider(),
    qwenTokenPlanProvider(),
    qwenTokenPlanCnProvider(),
    qwenTokenPlanIndividualProvider(),
    radiusProvider(),
    togetherProvider(),
    typesafeProvider(),
    vercelAIGatewayProvider(),
    xaiProvider(),
    xiaomiProvider(),
    xiaomiTokenPlanAmsProvider(),
    xiaomiTokenPlanCnProvider(),
    xiaomiTokenPlanSgpProvider(),
    zaiProvider(),
    zaiCodingCnProvider()
  ];
}
function builtinModels(options) {
  const models = createModels(options);
  for (const provider of builtinProviders()) {
    models.setProvider(provider);
  }
  return models;
}

// target/vendor-pi/pi-ai/dist/providers/faux.js
var DEFAULT_API = "faux";
var DEFAULT_PROVIDER = "faux";
var DEFAULT_MODEL_ID = "faux-1";
var DEFAULT_MODEL_NAME = "Faux Model";
var DEFAULT_BASE_URL = "http://localhost:0";
var DEFAULT_MIN_TOKEN_SIZE = 3;
var DEFAULT_MAX_TOKEN_SIZE = 5;
var DEFAULT_USAGE = {
  input: 0,
  output: 0,
  cacheRead: 0,
  cacheWrite: 0,
  totalTokens: 0,
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 }
};
function fauxText(text) {
  return { type: "text", text };
}
function fauxThinking(thinking) {
  return { type: "thinking", thinking };
}
function fauxToolCall(name, arguments_, options = {}) {
  return {
    type: "toolCall",
    id: options.id ?? randomId("tool"),
    name,
    arguments: arguments_
  };
}
function normalizeFauxAssistantContent(content) {
  if (typeof content === "string") {
    return [fauxText(content)];
  }
  return Array.isArray(content) ? content : [content];
}
function fauxAssistantMessage(content, options = {}) {
  return {
    role: "assistant",
    content: normalizeFauxAssistantContent(content),
    api: DEFAULT_API,
    provider: DEFAULT_PROVIDER,
    model: DEFAULT_MODEL_ID,
    usage: DEFAULT_USAGE,
    stopReason: options.stopReason ?? "stop",
    ...options.deferred === void 0 ? {} : { deferred: options.deferred },
    ...options.errorMessage === void 0 ? {} : { errorMessage: options.errorMessage },
    ...options.responseId === void 0 ? {} : { responseId: options.responseId },
    timestamp: options.timestamp ?? Date.now()
  };
}
function estimateTokens(text) {
  return Math.ceil(text.length / 4);
}
function randomId(prefix) {
  return `${prefix}:${Date.now()}:${Math.random().toString(36).slice(2)}`;
}
function contentToText(content) {
  if (typeof content === "string") {
    return content;
  }
  return content.map((block) => {
    if (block.type === "text") {
      return block.text;
    }
    return `[image:${block.mimeType}:${block.data.length}]`;
  }).join("\n");
}
function assistantContentToText(content) {
  return content.map((block) => {
    if (block.type === "text") {
      return block.text;
    }
    if (block.type === "thinking") {
      return block.thinking;
    }
    return `${block.name}:${JSON.stringify(block.arguments)}`;
  }).join("\n");
}
function toolResultToText(message) {
  return [message.toolName, ...message.content.map((block) => contentToText([block]))].join("\n");
}
function messageToText(message) {
  if (message.role === "system") {
    return [
      getSystemMessageText(message),
      ...message.toolsRemoved?.map((tool) => `tool-:${JSON.stringify(tool)}`) ?? [],
      ...message.toolsAdded?.map((tool) => `tool+:${JSON.stringify(tool)}`) ?? []
    ].filter((part) => part.length > 0).join("\n");
  }
  if (message.role === "user") {
    return contentToText(message.content);
  }
  if (message.role === "assistant") {
    return assistantContentToText(message.content);
  }
  return toolResultToText(message);
}
function serializeContext(context) {
  return context.messages.map((message) => `${message.role}:${messageToText(message)}`).join("\n\n");
}
function commonPrefixLength(a, b) {
  const length = Math.min(a.length, b.length);
  let index = 0;
  while (index < length && a[index] === b[index]) {
    index++;
  }
  return index;
}
function withUsageEstimate(message, context, options, promptCache) {
  const promptText = serializeContext(context);
  const promptTokens = estimateTokens(promptText);
  const outputTokens = estimateTokens(assistantContentToText(message.content));
  let input = promptTokens;
  let cacheRead = 0;
  let cacheWrite = 0;
  const sessionId = options?.sessionId;
  if (sessionId && options?.cacheRetention !== "none") {
    const previousPrompt = promptCache.get(sessionId);
    if (previousPrompt) {
      const cachedChars = commonPrefixLength(previousPrompt, promptText);
      cacheRead = estimateTokens(previousPrompt.slice(0, cachedChars));
      cacheWrite = estimateTokens(promptText.slice(cachedChars));
      input = Math.max(0, promptTokens - cacheRead);
    } else {
      cacheWrite = promptTokens;
    }
    promptCache.set(sessionId, promptText);
  }
  return {
    ...message,
    usage: {
      input,
      output: outputTokens,
      cacheRead,
      cacheWrite,
      totalTokens: input + outputTokens + cacheRead + cacheWrite,
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 }
    }
  };
}
function splitStringByTokenSize(text, minTokenSize, maxTokenSize) {
  const chunks = [];
  let index = 0;
  while (index < text.length) {
    const tokenSize = minTokenSize + Math.floor(Math.random() * (maxTokenSize - minTokenSize + 1));
    const charSize = Math.max(1, tokenSize * 4);
    chunks.push(text.slice(index, index + charSize));
    index += charSize;
  }
  return chunks.length > 0 ? chunks : [""];
}
function cloneMessage(message, api, provider, modelId) {
  const cloned = structuredClone(message);
  return {
    ...cloned,
    api,
    provider,
    model: modelId,
    timestamp: cloned.timestamp ?? Date.now(),
    usage: cloned.usage ?? DEFAULT_USAGE
  };
}
function createDeferredMessage(model, handle) {
  return {
    role: "assistant",
    content: [],
    api: model.api,
    provider: model.provider,
    model: model.id,
    usage: DEFAULT_USAGE,
    stopReason: "deferred",
    deferred: handle,
    timestamp: Date.now()
  };
}
function createErrorMessage(error, api, provider, modelId) {
  return {
    role: "assistant",
    content: [],
    api,
    provider,
    model: modelId,
    usage: DEFAULT_USAGE,
    stopReason: "error",
    errorMessage: error instanceof Error ? error.message : String(error),
    timestamp: Date.now()
  };
}
function createAbortedMessage(partial) {
  return {
    ...partial,
    stopReason: "aborted",
    errorMessage: "Request was aborted",
    timestamp: Date.now()
  };
}
function scheduleChunk(chunk, tokensPerSecond) {
  if (!tokensPerSecond || tokensPerSecond <= 0) {
    return new Promise((resolve) => queueMicrotask(resolve));
  }
  const delayMs = estimateTokens(chunk) / tokensPerSecond * 1e3;
  return new Promise((resolve) => setTimeout(resolve, delayMs));
}
async function streamWithDeltas(stream3, message, minTokenSize, maxTokenSize, tokensPerSecond, signal) {
  const partial = { ...message, content: [], stopReason: "pending" };
  if (signal?.aborted) {
    const aborted = createAbortedMessage(partial);
    stream3.push({ type: "error", reason: "aborted", error: aborted });
    stream3.end(aborted);
    return;
  }
  stream3.push({ type: "start", partial: { ...partial } });
  for (let index = 0; index < message.content.length; index++) {
    if (signal?.aborted) {
      const aborted = createAbortedMessage(partial);
      stream3.push({ type: "error", reason: "aborted", error: aborted });
      stream3.end(aborted);
      return;
    }
    const block = message.content[index];
    if (block.type === "thinking") {
      partial.content = [...partial.content, { type: "thinking", thinking: "" }];
      stream3.push({ type: "thinking_start", contentIndex: index, partial: { ...partial } });
      for (const chunk of splitStringByTokenSize(block.thinking, minTokenSize, maxTokenSize)) {
        await scheduleChunk(chunk, tokensPerSecond);
        if (signal?.aborted) {
          const aborted = createAbortedMessage(partial);
          stream3.push({ type: "error", reason: "aborted", error: aborted });
          stream3.end(aborted);
          return;
        }
        partial.content[index].thinking += chunk;
        stream3.push({ type: "thinking_delta", contentIndex: index, delta: chunk, partial: { ...partial } });
      }
      stream3.push({
        type: "thinking_end",
        contentIndex: index,
        content: block.thinking,
        partial: { ...partial }
      });
      continue;
    }
    if (block.type === "text") {
      partial.content = [...partial.content, { type: "text", text: "" }];
      stream3.push({ type: "text_start", contentIndex: index, partial: { ...partial } });
      for (const chunk of splitStringByTokenSize(block.text, minTokenSize, maxTokenSize)) {
        await scheduleChunk(chunk, tokensPerSecond);
        if (signal?.aborted) {
          const aborted = createAbortedMessage(partial);
          stream3.push({ type: "error", reason: "aborted", error: aborted });
          stream3.end(aborted);
          return;
        }
        partial.content[index].text += chunk;
        stream3.push({ type: "text_delta", contentIndex: index, delta: chunk, partial: { ...partial } });
      }
      stream3.push({ type: "text_end", contentIndex: index, content: block.text, partial: { ...partial } });
      continue;
    }
    partial.content = [...partial.content, { type: "toolCall", id: block.id, name: block.name, arguments: {} }];
    stream3.push({ type: "toolcall_start", contentIndex: index, partial: { ...partial } });
    for (const chunk of splitStringByTokenSize(JSON.stringify(block.arguments), minTokenSize, maxTokenSize)) {
      await scheduleChunk(chunk, tokensPerSecond);
      if (signal?.aborted) {
        const aborted = createAbortedMessage(partial);
        stream3.push({ type: "error", reason: "aborted", error: aborted });
        stream3.end(aborted);
        return;
      }
      stream3.push({ type: "toolcall_delta", contentIndex: index, delta: chunk, partial: { ...partial } });
    }
    partial.content[index].arguments = block.arguments;
    stream3.push({ type: "toolcall_end", contentIndex: index, toolCall: block, partial: { ...partial } });
  }
  if (message.stopReason === "pending") {
    throw new Error("Faux response ended without a stop reason");
  }
  if (message.stopReason === "error" || message.stopReason === "aborted") {
    stream3.push({ type: "error", reason: message.stopReason, error: message });
    stream3.end(message);
    return;
  }
  stream3.push({ type: "done", reason: message.stopReason, message });
  stream3.end(message);
}
function createFauxCore(options) {
  const api = options.api ?? randomId(DEFAULT_API);
  const provider = options.provider ?? DEFAULT_PROVIDER;
  const minTokenSize = Math.max(1, Math.min(options.tokenSize?.min ?? DEFAULT_MIN_TOKEN_SIZE, options.tokenSize?.max ?? DEFAULT_MAX_TOKEN_SIZE));
  const maxTokenSize = Math.max(minTokenSize, options.tokenSize?.max ?? DEFAULT_MAX_TOKEN_SIZE);
  let pendingResponses = [];
  const tokensPerSecond = options.tokensPerSecond;
  const state = { callCount: 0, deferredFetchCount: 0, cancelledDeferred: [] };
  const promptCache = /* @__PURE__ */ new Map();
  const deferredResponses = /* @__PURE__ */ new Map();
  const modelDefinitions = options.models?.length ? options.models : [
    {
      id: DEFAULT_MODEL_ID,
      name: DEFAULT_MODEL_NAME,
      reasoning: false,
      input: ["text", "image"],
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
      contextWindow: 128e3,
      maxTokens: 16384
    }
  ];
  const models = modelDefinitions.map((definition) => ({
    id: definition.id,
    name: definition.name ?? definition.id,
    api,
    provider,
    baseUrl: DEFAULT_BASE_URL,
    reasoning: definition.reasoning ?? false,
    input: definition.input ?? ["text", "image"],
    inputLimits: definition.inputLimits,
    cost: definition.cost ?? { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    contextWindow: definition.contextWindow ?? 128e3,
    maxTokens: definition.maxTokens ?? 16384
  }));
  const resolveResponse = async (step, context, streamOptions, requestModel) => {
    const resolved = typeof step === "function" ? await step(context, streamOptions, state, requestModel) : step;
    return withUsageEstimate(cloneMessage(resolved, api, provider, requestModel.id), context, streamOptions, promptCache);
  };
  const stream3 = (requestModel, context, streamOptions) => {
    const outer = createAssistantMessageEventStream();
    const step = pendingResponses.shift();
    state.callCount++;
    queueMicrotask(async () => {
      try {
        await streamOptions?.onResponse?.({ status: 200, headers: {} }, requestModel);
        if (!step) {
          let message2 = createErrorMessage(new Error("No more faux responses queued"), api, provider, requestModel.id);
          message2 = withUsageEstimate(message2, context, streamOptions, promptCache);
          outer.push({ type: "error", reason: "error", error: message2 });
          outer.end(message2);
          return;
        }
        if (streamOptions?.deferred) {
          const handle = {
            provider: requestModel.provider,
            modelId: requestModel.id,
            api: requestModel.api,
            id: randomId("deferred"),
            ...options.deferred?.pollAfterMs !== void 0 ? { pollAfterMs: options.deferred.pollAfterMs } : {}
          };
          deferredResponses.set(handle.id, {
            handle,
            step,
            context,
            options: streamOptions,
            model: requestModel,
            pendingFetches: Math.max(0, Math.floor(options.deferred?.pendingFetches ?? 0)),
            cancelled: false
          });
          await streamWithDeltas(outer, createDeferredMessage(requestModel, handle), minTokenSize, maxTokenSize, tokensPerSecond, streamOptions.signal);
          return;
        }
        const message = await resolveResponse(step, context, streamOptions, requestModel);
        await streamWithDeltas(outer, message, minTokenSize, maxTokenSize, tokensPerSecond, streamOptions?.signal);
      } catch (error) {
        const message = createErrorMessage(error, api, provider, requestModel.id);
        outer.push({ type: "error", reason: "error", error: message });
        outer.end(message);
      }
    });
    return outer;
  };
  const streamSimple3 = (streamModel, context, streamOptions) => stream3(streamModel, context, streamOptions);
  const fetchDeferred = (requestModel, handle, fetchOptions) => {
    const outer = createAssistantMessageEventStream();
    state.deferredFetchCount++;
    queueMicrotask(async () => {
      try {
        await fetchOptions?.onResponse?.({ status: 200, headers: {} }, requestModel);
        const entry = deferredResponses.get(handle.id);
        if (!entry || entry.handle.provider !== handle.provider || entry.handle.modelId !== handle.modelId || entry.handle.api !== handle.api) {
          throw new Error(`Unknown faux deferred response: ${handle.id}`);
        }
        if (entry.cancelled)
          throw new Error(`Faux deferred response was cancelled: ${handle.id}`);
        if (entry.pendingFetches > 0) {
          entry.pendingFetches--;
          await streamWithDeltas(outer, createDeferredMessage(requestModel, entry.handle), minTokenSize, maxTokenSize, tokensPerSecond, fetchOptions?.signal);
          return;
        }
        if (!entry.final) {
          const { deferred: _deferred, signal: _submissionSignal, onResponse: _submissionOnResponse, ...submissionOptions } = entry.options ?? {};
          try {
            entry.final = await resolveResponse(entry.step, entry.context, submissionOptions, entry.model);
          } catch (error) {
            entry.final = createErrorMessage(error, api, provider, entry.model.id);
          }
        }
        await streamWithDeltas(outer, entry.final, minTokenSize, maxTokenSize, tokensPerSecond, fetchOptions?.signal);
      } catch (error) {
        const message = createErrorMessage(error, api, provider, requestModel.id);
        outer.push({ type: "error", reason: "error", error: message });
        outer.end(message);
      }
    });
    return outer;
  };
  const cancelDeferred = async (requestModel, handle, cancelOptions) => {
    state.cancelledDeferred.push(structuredClone(handle));
    const entry = deferredResponses.get(handle.id);
    if (entry)
      entry.cancelled = true;
    await cancelOptions?.onResponse?.({ status: 200, headers: {} }, requestModel);
  };
  function getModel(requestedModelId) {
    if (!requestedModelId) {
      return models[0];
    }
    return models.find((candidate) => candidate.id === requestedModelId);
  }
  return {
    api,
    provider,
    models,
    stream: stream3,
    streamSimple: streamSimple3,
    fetchDeferred,
    cancelDeferred,
    getModel,
    state,
    setResponses(responses) {
      pendingResponses = [...responses];
    },
    appendResponses(responses) {
      pendingResponses.push(...responses);
    },
    getPendingResponseCount() {
      return pendingResponses.length;
    }
  };
}
function fauxProvider(options = {}) {
  const core = createFauxCore(options);
  const provider = createProvider({
    id: core.provider,
    auth: { apiKey: { name: "Faux", resolve: async () => ({ auth: {} }) } },
    models: core.models,
    api: {
      stream: core.stream,
      streamSimple: core.streamSimple,
      fetchDeferred: core.fetchDeferred,
      cancelDeferred: core.cancelDeferred
    }
  });
  return {
    provider,
    api: core.api,
    models: core.models,
    getModel: core.getModel,
    state: core.state,
    setResponses: core.setResponses,
    appendResponses: core.appendResponses,
    getPendingResponseCount: core.getPendingResponseCount
  };
}

// target/vendor-pi/pi-ai/entry.mjs
setBedrockProviderModule(bedrock_converse_stream_exports);
registerBundledOAuthFlowLoaders({
  anthropic: async () => (await import("./chunk-PGLP2ZTS.mjs")).anthropicOAuth,
  openaiCodex: async () => (await import("./chunk-GBTGKDGZ.mjs")).openaiCodexOAuth,
  openaiChatGPT: async () => (await import("./chunk-OGDY2RHW.mjs")).openaiChatGPTOAuth,
  githubCopilot: async () => (await import("./chunk-5VHIA765.mjs")).githubCopilotOAuth,
  openrouter: async () => (await import("./chunk-RJELU67E.mjs")).openRouterOAuth,
  kimiCoding: async () => (await import("./chunk-AOAAX7MO.mjs")).kimiCodingOAuth,
  meta: async () => (await import("./chunk-QSQ533LI.mjs")).metaOAuth,
  xai: async () => (await import("./chunk-RCUWQ7KH.mjs")).xaiOAuth,
  radius: async (options) => (await import("./chunk-5RVKTJ5S.mjs")).createRadiusOAuth(options)
});
export {
  AMAZON_BEDROCK_CLASSIFIER_MODELS,
  AMAZON_BEDROCK_IMAGE_MODELS,
  AMAZON_BEDROCK_MODELS,
  ANTHROPIC_API_KEY_ENV,
  ANTHROPIC_AUTH_TOKEN_ENV,
  ANTHROPIC_CLASSIFIER_MODELS,
  ANTHROPIC_FEDERATION_RULE_ID_ENV,
  ANTHROPIC_IDENTITY_TOKEN_FILE_ENV,
  ANTHROPIC_IMAGE_MODELS,
  ANTHROPIC_MODELS,
  ANTHROPIC_OAUTH_TOKEN_ENV,
  ANTHROPIC_ORGANIZATION_ID_ENV,
  ANTHROPIC_SERVICE_ACCOUNT_ID_ENV,
  ANTHROPIC_WORKSPACE_ID_ENV,
  ANT_LING_CLASSIFIER_MODELS,
  ANT_LING_IMAGE_MODELS,
  ANT_LING_MODELS,
  AZURE_OPENAI_RESPONSES_CLASSIFIER_MODELS,
  AZURE_OPENAI_RESPONSES_IMAGE_MODELS,
  AZURE_OPENAI_RESPONSES_MODELS,
  AssistantMessageEventStream,
  AssistantMessageFrameEncoder,
  BASETEN_CLASSIFIER_MODELS,
  BASETEN_IMAGE_MODELS,
  BASETEN_MODELS,
  CEREBRAS_CLASSIFIER_MODELS,
  CEREBRAS_IMAGE_MODELS,
  CEREBRAS_MODELS,
  CLOUDFLARE_AI_GATEWAY_ANTHROPIC_BASE_URL,
  CLOUDFLARE_AI_GATEWAY_CLASSIFIER_MODELS,
  CLOUDFLARE_AI_GATEWAY_COMPAT_BASE_URL,
  CLOUDFLARE_AI_GATEWAY_IMAGE_MODELS,
  CLOUDFLARE_AI_GATEWAY_MODELS,
  CLOUDFLARE_AI_GATEWAY_OPENAI_BASE_URL,
  CLOUDFLARE_GATEWAY_BINDING_AUTH_SENTINEL,
  CLOUDFLARE_WORKERS_AI_BASE_URL,
  CLOUDFLARE_WORKERS_AI_CLASSIFIER_MODELS,
  CLOUDFLARE_WORKERS_AI_IMAGE_MODELS,
  CLOUDFLARE_WORKERS_AI_MODELS,
  CLOUDFLARE_WORKERS_AI_REST_BASE_URL,
  DEEPSEEK_CLASSIFIER_MODELS,
  DEEPSEEK_IMAGE_MODELS,
  DEEPSEEK_MODELS,
  DEFAULT_MAX_AGENT_RETRY_DELAY_MS,
  DEFAULT_RADIUS_GATEWAY,
  DEFAULT_THINKING_BUDGETS,
  EventStream,
  FIREWORKS_CLASSIFIER_MODELS,
  FIREWORKS_IMAGE_MODELS,
  FIREWORKS_MODELS,
  GITHUB_COPILOT_CLASSIFIER_MODELS,
  GITHUB_COPILOT_IMAGE_MODELS,
  GITHUB_COPILOT_MODELS,
  GOOGLE_CLASSIFIER_MODELS,
  GOOGLE_IMAGE_MODELS,
  GOOGLE_MODELS,
  GOOGLE_VERTEX_CLASSIFIER_MODELS,
  GOOGLE_VERTEX_IMAGE_MODELS,
  GOOGLE_VERTEX_MODELS,
  GROQ_CLASSIFIER_MODELS,
  GROQ_IMAGE_MODELS,
  GROQ_MODELS,
  HUGGINGFACE_CLASSIFIER_MODELS,
  HUGGINGFACE_IMAGE_MODELS,
  HUGGINGFACE_MODELS,
  InMemoryCredentialStore,
  InMemoryModelsStore,
  KIMI_CODING_CLASSIFIER_MODELS,
  KIMI_CODING_IMAGE_MODELS,
  KIMI_CODING_MODELS,
  MAX_PROVIDER_ERROR_BODY_CHARS,
  META_CLASSIFIER_MODELS,
  META_IMAGE_MODELS,
  META_MODELS,
  MINIMAX_CLASSIFIER_MODELS,
  MINIMAX_CN_CLASSIFIER_MODELS,
  MINIMAX_CN_IMAGE_MODELS,
  MINIMAX_CN_MODELS,
  MINIMAX_IMAGE_MODELS,
  MINIMAX_MODELS,
  MIN_ANSWER_TOKENS,
  MISTRAL_CLASSIFIER_MODELS,
  MISTRAL_IMAGE_MODELS,
  MISTRAL_MODELS,
  MOONSHOTAI_CLASSIFIER_MODELS,
  MOONSHOTAI_CN_CLASSIFIER_MODELS,
  MOONSHOTAI_CN_IMAGE_MODELS,
  MOONSHOTAI_CN_MODELS,
  MOONSHOTAI_IMAGE_MODELS,
  MOONSHOTAI_MODELS,
  ModelsError,
  NVIDIA_CLASSIFIER_MODELS,
  NVIDIA_IMAGE_MODELS,
  NVIDIA_MODELS,
  OPENAI_CLASSIFIER_MODELS,
  OPENAI_CODEX_CLASSIFIER_MODELS,
  OPENAI_CODEX_IMAGE_MODELS,
  OPENAI_CODEX_MODELS,
  OPENAI_IMAGE_MODELS,
  OPENAI_MODELS,
  OPENAI_PROMPT_CACHE_KEY_MAX_LENGTH,
  OPENCODE_CLASSIFIER_MODELS,
  OPENCODE_GO_CLASSIFIER_MODELS,
  OPENCODE_GO_IMAGE_MODELS,
  OPENCODE_GO_MODELS,
  OPENCODE_IMAGE_MODELS,
  OPENCODE_MODELS,
  OPENROUTER_CLASSIFIER_MODELS,
  OPENROUTER_IMAGE_MODELS,
  OPENROUTER_MODELS,
  PiMessagesResponseError,
  QWEN_TOKEN_PLAN_CLASSIFIER_MODELS,
  QWEN_TOKEN_PLAN_CN_CLASSIFIER_MODELS,
  QWEN_TOKEN_PLAN_CN_IMAGE_MODELS,
  QWEN_TOKEN_PLAN_CN_MODELS,
  QWEN_TOKEN_PLAN_IMAGE_MODELS,
  QWEN_TOKEN_PLAN_INDIVIDUAL_CLASSIFIER_MODELS,
  QWEN_TOKEN_PLAN_INDIVIDUAL_IMAGE_MODELS,
  QWEN_TOKEN_PLAN_INDIVIDUAL_MODELS,
  QWEN_TOKEN_PLAN_MODELS,
  RADIUS_CLASSIFIER_MODELS,
  RADIUS_IMAGE_MODELS,
  RADIUS_MODELS,
  StringEnum,
  TOGETHER_CLASSIFIER_MODELS,
  TOGETHER_IMAGE_MODELS,
  TOGETHER_MODELS,
  TYPESAFE_CLASSIFIER_MODELS,
  TYPESAFE_IMAGE_MODELS,
  TYPESAFE_MODELS,
  UNSUPPORTED_PROXY_PROTOCOL_MESSAGE,
  VERCEL_AI_GATEWAY_CLASSIFIER_MODELS,
  VERCEL_AI_GATEWAY_IMAGE_MODELS,
  VERCEL_AI_GATEWAY_MODELS,
  XAI_CLASSIFIER_MODELS,
  XAI_IMAGE_MODELS,
  XAI_MODELS,
  XIAOMI_CLASSIFIER_MODELS,
  XIAOMI_IMAGE_MODELS,
  XIAOMI_MODELS,
  XIAOMI_TOKEN_PLAN_AMS_CLASSIFIER_MODELS,
  XIAOMI_TOKEN_PLAN_AMS_IMAGE_MODELS,
  XIAOMI_TOKEN_PLAN_AMS_MODELS,
  XIAOMI_TOKEN_PLAN_CN_CLASSIFIER_MODELS,
  XIAOMI_TOKEN_PLAN_CN_IMAGE_MODELS,
  XIAOMI_TOKEN_PLAN_CN_MODELS,
  XIAOMI_TOKEN_PLAN_SGP_CLASSIFIER_MODELS,
  XIAOMI_TOKEN_PLAN_SGP_IMAGE_MODELS,
  XIAOMI_TOKEN_PLAN_SGP_MODELS,
  ZAI_CLASSIFIER_MODELS,
  ZAI_CODING_CN_CLASSIFIER_MODELS,
  ZAI_CODING_CN_IMAGE_MODELS,
  ZAI_CODING_CN_MODELS,
  ZAI_IMAGE_MODELS,
  ZAI_MODELS,
  adjustMaxTokensForThinking,
  amazonBedrockProvider,
  answerFromProbabilities,
  antLingProvider,
  anthropicMessagesApi,
  anthropicProvider,
  appendAssistantMessageDiagnostic,
  appendGrammarToolInputJsonDelta,
  assertChatModel,
  assertClassifierModel,
  assertImageModel,
  azureOpenAIResponsesApi,
  azureOpenAIResponsesProvider,
  basetenProvider,
  bedrockConverseStreamApi,
  buildBaseOptions,
  buildCopilotDynamicHeaders,
  builtinModels,
  builtinProviders,
  calculateContextTokens,
  calculateCost,
  cerebrasProvider,
  clampMaxTokensToContext,
  clampOpenAIPromptCacheKey,
  clampReasoning,
  clampThinkingBudgetToAnswerRoom,
  clampThinkingLevel,
  classifierErrorResult,
  classify,
  classifySystemOne,
  cleanupSessionResources,
  cloudflareAIGatewayAuth,
  cloudflareAIGatewayProvider,
  cloudflareClassifier,
  cloudflareStreams,
  cloudflareWorkersAIAuth,
  cloudflareWorkersAIProvider,
  cloudflareWorkersAISystemOneApi,
  collapseSystemMessages,
  combineAbortSignals,
  contentText,
  convertMessages,
  convertResponsesMessages,
  convertResponsesTools,
  convertTools,
  createAiBindingFetch,
  createAssistantMessageDiagnostic,
  createAssistantMessageEventStream,
  createFauxCore,
  createGrammarToolInputProperties,
  createInitialSystemMessage,
  createModels,
  createProvider,
  declarationsEqual,
  deepseekProvider,
  defaultProviderAuthContext,
  envApiKeyAuth,
  estimateContextTokens,
  estimateMessageTokens,
  estimateTextAndImageContentTokens,
  estimateTextTokens,
  extractDiagnosticError,
  fauxAssistantMessage,
  fauxProvider,
  fauxText,
  fauxThinking,
  fauxToolCall,
  findEnvKeys,
  fireworksProvider,
  formatProviderError,
  formatThrownValue,
  generateImages,
  generateImagesOpenRouter,
  getAllBuiltinModels,
  getBuiltinClassifierModel,
  getBuiltinClassifierModels,
  getBuiltinImageModel,
  getBuiltinImageModels,
  getBuiltinModel,
  getBuiltinModelDataGeneratedAt,
  getBuiltinModels,
  getBuiltinProviders,
  getCurrentSystemMessage,
  getCurrentSystemPrompt,
  getCurrentTools,
  getDeclaredTools,
  getDisabledGoogleThinkingConfig,
  getEnvApiKey,
  getGrammarToolInput,
  getImageModel,
  getImageModels,
  getImageProviders,
  getImagesApiProvider,
  getInitialSystemMessage,
  getJsonSchemaToolParameters,
  getModelType,
  getOverflowPatterns,
  getPiUserAgent,
  getProviderEnvValue,
  getRadiusCredentialConfig,
  getRadiusModels,
  getRadiusModelsFromConfig,
  getSupportedThinkingLevels,
  getSystemMessageText,
  getToolStateChanges,
  githubCopilotProvider,
  googleGenerativeAIApi,
  googleProvider,
  googleVertexApi,
  googleVertexProvider,
  groqProvider,
  hasApi,
  hasCopilotVisionInput,
  hasNonAdditiveToolChanges,
  hasToolRedefinitions,
  headersToRecord,
  huggingfaceProvider,
  imageErrorResult,
  inferCopilotInitiator,
  isContextOverflow,
  isModelType,
  isRecord,
  isRecoverableLength,
  isRetryableAssistantError,
  isThinkingPart,
  kimiCodingProvider,
  labelProbabilities,
  lazyApi,
  lazyOAuth,
  lazyStream,
  llamaCppClassifyApi,
  llamaServerRoot,
  loadRadiusGatewayConfig,
  makeStrictJsonSchema,
  mapStopReason,
  mapStopReasonString,
  mapToolChoice,
  metaProvider,
  minimaxCnProvider,
  minimaxProvider,
  mistralConversationsApi,
  mistralProvider,
  modelsAreEqual,
  moonshotaiCnProvider,
  moonshotaiProvider,
  normalizeContext,
  normalizeProviderError,
  normalizeRadiusGatewayUrl,
  nvidiaProvider,
  oauthErrorHtml,
  oauthSuccessHtml,
  openAICodexResponsesApi,
  openAICompletionsApi,
  openAIResponsesApi,
  openaiCodexProvider,
  openaiProvider,
  opencodeGoProvider,
  opencodeProvider,
  openrouterImagesApi,
  openrouterProvider,
  operationSignal,
  peakConfidence,
  piMessagesApi,
  processResponsesStream,
  providerHeadersToRecord,
  qwenTokenPlanCnProvider,
  qwenTokenPlanIndividualProvider,
  qwenTokenPlanProvider,
  raceWithAbortSignal,
  radiusProvider,
  reduceAssistantMessageFrames,
  registerBuiltInImagesApiProviders,
  registerImagesApiProvider,
  registerSessionResourceCleanup,
  renderQuestion,
  renderSystemMessageUpdate,
  requiresToolCallId,
  resolveCloudflareModel,
  resolveGoogleFunctionCallingMode,
  resolveGoogleThinkingLevel,
  resolveGrammarConstrainedSampling,
  resolveHttpProxyUrlForTarget,
  resolveJsonSchemaStrictSampling,
  resolveTranscript,
  resolveTranscriptTools,
  retainThoughtSignature,
  retryAssistantCall,
  retryDelayMs,
  retryGoogleRequest,
  retryProviderRequest,
  safeJsonStringify,
  sanitizeSurrogates,
  setBedrockProviderModule,
  shortHash,
  sleep,
  stream,
  streamAnthropic,
  streamAzureOpenAIResponses,
  streamGoogle,
  streamGoogleVertex,
  streamMistral,
  streamOpenAICodexResponses,
  streamOpenAICompletions,
  streamOpenAIResponses,
  streamSimple,
  streamSimpleAnthropic,
  streamSimpleAzureOpenAIResponses,
  streamSimpleGoogle,
  streamSimpleGoogleVertex,
  streamSimpleMistral,
  streamSimpleOpenAICodexResponses,
  streamSimpleOpenAICompletions,
  streamSimpleOpenAIResponses,
  supportsGoogleStrictToolSampling,
  thinkingBudgetForLevel,
  toGoogleSdkThinkingLevel,
  toGoogleThinkingLevel,
  toToolDeclaration,
  togetherProvider,
  transformMessages,
  truncateErrorText,
  typesafeProvider,
  typesafeSystemOneApi,
  usesGoogleThinkingLevel,
  uuidv7,
  vercelAIGatewayProvider,
  withOpenCodeSessionHeader,
  withoutInitialSystemMessage,
  xaiProvider,
  xiaomiProvider,
  xiaomiTokenPlanAmsProvider,
  xiaomiTokenPlanCnProvider,
  xiaomiTokenPlanSgpProvider,
  zaiCodingCnProvider,
  zaiProvider
};
