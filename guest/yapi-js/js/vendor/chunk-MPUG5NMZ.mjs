import {
  AssistantMessageEventStream,
  formatThrownValue
} from "./chunk-VLRLU4IK.mjs";

// target/vendor-pi/pi-ai/dist/api/lazy.js
function createSetupErrorMessage(model, error) {
  return {
    role: "assistant",
    content: [],
    api: model.api,
    provider: model.provider,
    model: model.id,
    usage: {
      input: 0,
      output: 0,
      cacheRead: 0,
      cacheWrite: 0,
      totalTokens: 0,
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 }
    },
    stopReason: "error",
    errorMessage: error instanceof Error ? error.message : String(error),
    timestamp: Date.now()
  };
}
function hasResult(source) {
  return typeof source.result === "function";
}
async function forwardStream(target, source) {
  for await (const event of source) {
    target.push(event);
  }
  target.end(hasResult(source) ? await source.result() : void 0);
}
function lazyStream(model, setup) {
  const outer = new AssistantMessageEventStream();
  setup().then((inner) => forwardStream(outer, inner)).catch((error) => {
    const message = createSetupErrorMessage(model, error);
    outer.push({ type: "error", reason: "error", error: message });
    outer.end(message);
  });
  return outer;
}
function lazyApi(load, capabilities) {
  const api = {
    stream: (model, context, options) => lazyStream(model, async () => (await load()).stream(model, context, options)),
    streamSimple: (model, context, options) => lazyStream(model, async () => (await load()).streamSimple(model, context, options))
  };
  if (capabilities?.fetchDeferred) {
    api.fetchDeferred = (model, handle, options) => lazyStream(model, async () => {
      const implementation = await load();
      if (!implementation.fetchDeferred)
        throw new Error("API does not support deferred responses");
      return implementation.fetchDeferred(model, handle, options);
    });
  }
  if (capabilities?.cancelDeferred) {
    api.cancelDeferred = async (model, handle, options) => {
      const implementation = await load();
      if (!implementation.cancelDeferred)
        throw new Error("API cannot cancel deferred responses");
      await implementation.cancelDeferred(model, handle, options);
    };
  }
  return api;
}

// target/vendor-pi/pi-ai/dist/utils/abort.js
function abortReason(signal) {
  if (signal.reason !== void 0)
    return signal.reason;
  const error = new Error("The operation was aborted");
  error.name = "AbortError";
  return error;
}
function operationSignal(signal) {
  return signal ?? new AbortController().signal;
}
function raceWithAbortSignal(operation, signal) {
  if (signal.aborted) {
    void operation.catch(() => {
    });
    return Promise.reject(abortReason(signal));
  }
  return new Promise((resolve, reject) => {
    let settled = false;
    const cleanup = () => signal.removeEventListener("abort", onAbort);
    const onAbort = () => {
      if (settled)
        return;
      settled = true;
      cleanup();
      reject(abortReason(signal));
    };
    signal.addEventListener("abort", onAbort, { once: true });
    void operation.then((value) => {
      if (settled)
        return;
      settled = true;
      cleanup();
      resolve(value);
    }, (error) => {
      if (settled)
        return;
      settled = true;
      cleanup();
      reject(error);
    });
    if (signal.aborted)
      onAbort();
  });
}

// target/vendor-pi/pi-ai/dist/auth/credential-store.js
var InMemoryCredentialStore = class {
  credentials = /* @__PURE__ */ new Map();
  chains = /* @__PURE__ */ new Map();
  /** Serialize tasks per provider id without releasing the chain before active work settles. */
  enqueue(providerId, task, options) {
    const signal = operationSignal(options?.signal);
    const previous = this.chains.get(providerId) ?? Promise.resolve();
    const queued = (async () => {
      await previous.catch(() => {
      });
      signal.throwIfAborted();
      return task();
    })();
    const tail = queued.catch(() => {
    });
    this.chains.set(providerId, tail);
    void tail.then(() => {
      if (this.chains.get(providerId) === tail)
        this.chains.delete(providerId);
    });
    return raceWithAbortSignal(queued, signal);
  }
  async read(providerId, options) {
    options?.signal?.throwIfAborted();
    return this.credentials.get(providerId);
  }
  async list(options) {
    options?.signal?.throwIfAborted();
    return [...this.credentials].map(([providerId, credential]) => ({ providerId, type: credential.type }));
  }
  modify(providerId, fn, options) {
    return this.enqueue(providerId, async () => {
      const current = this.credentials.get(providerId);
      const next = await fn(current);
      options?.signal?.throwIfAborted();
      if (next !== void 0)
        this.credentials.set(providerId, next);
      return next ?? current;
    }, options);
  }
  delete(providerId, options) {
    return this.enqueue(providerId, async () => {
      this.credentials.delete(providerId);
    }, options);
  }
};

// target/vendor-pi/pi-ai/dist/utils/models-error.js
var ModelsError = class extends Error {
  code;
  constructor(code, message, options) {
    super(withCauseDetail(message, options?.cause), options);
    this.name = "ModelsError";
    this.code = code;
  }
};
function withCauseDetail(message, cause) {
  if (cause === void 0 || cause === null)
    return message;
  const detail = formatThrownValue(cause).trim();
  if (!detail || message.includes(detail))
    return message;
  return `${message}: ${detail}`;
}

// target/vendor-pi/pi-ai/dist/auth/resolve.js
var DEFAULT_OAUTH_MINIMUM_VALIDITY_MS = 5 * 60 * 1e3;

// target/vendor-pi/pi-ai/dist/models-store.js
var InMemoryModelsStore = class {
  entries = /* @__PURE__ */ new Map();
  async read(providerId, options) {
    options?.signal?.throwIfAborted();
    const entry = this.entries.get(providerId);
    return entry ? structuredClone(entry) : void 0;
  }
  async write(providerId, entry, options) {
    options?.signal?.throwIfAborted();
    this.entries.set(providerId, structuredClone(entry));
  }
  async delete(providerId, options) {
    options?.signal?.throwIfAborted();
    this.entries.delete(providerId);
  }
};

// target/vendor-pi/pi-ai/dist/utils/model-operations.js
function getModelType(model) {
  return model.type ?? "chat";
}
function isModelType(model, type) {
  return getModelType(model) === type;
}
function assertChatModel(model) {
  if (!isModelType(model, "chat")) {
    throw new ModelsError("provider", `Model ${model.provider}/${model.id} is not a chat model`);
  }
}
function assertImageModel(model) {
  if (!isModelType(model, "image")) {
    throw new ModelsError("provider", `Model ${model.provider}/${model.id} is not an image model`);
  }
}
function assertClassifierModel(model) {
  if (!isModelType(model, "classifier")) {
    throw new ModelsError("provider", `Model ${model.provider}/${model.id} is not a classifier model`);
  }
}
function imageErrorResult(model, error, aborted = false) {
  return {
    api: model.api,
    provider: model.provider,
    model: model.id,
    output: [],
    stopReason: aborted ? "aborted" : "error",
    errorMessage: error instanceof Error ? error.message : String(error),
    timestamp: Date.now()
  };
}
function classifierErrorResult(model, error, aborted = false) {
  return {
    api: model.api,
    provider: model.provider,
    model: model.id,
    answers: {},
    stopReason: aborted ? "aborted" : "error",
    errorMessage: error instanceof Error ? error.message : String(error),
    timestamp: Date.now()
  };
}

// target/vendor-pi/pi-ai/dist/utils/text.js
function contentText(content, separator = "\n") {
  if (typeof content === "string")
    return content;
  return content.filter((block) => block.type === "text").map((block) => block.text).join(separator);
}
function getSystemMessageText(message) {
  const parts = [contentText(message.content)];
  for (const text of Object.values(message.sections ?? {})) {
    if (text !== null)
      parts.push(text);
  }
  return parts.filter((part) => part.length > 0).join("\n\n");
}
function renderSystemMessageUpdate(message) {
  const parts = [];
  const text = contentText(message.content);
  if (text.length > 0)
    parts.push(text);
  for (const [name, value] of Object.entries(message.sections ?? {})) {
    parts.push(value === null ? `Removed system prompt section "${name}".` : `Updated system prompt section "${name}":

${value}`);
  }
  return parts.join("\n\n");
}

// target/vendor-pi/pi-ai/dist/utils/transcript.js
function createInitialSystemMessage(systemPrompt, tools) {
  const hasSystemPrompt = systemPrompt !== void 0 && systemPrompt.length > 0;
  const hasTools = tools !== void 0 && tools.length > 0;
  if (!hasSystemPrompt && !hasTools)
    return void 0;
  return {
    role: "system",
    content: systemPrompt ?? "",
    ...hasTools ? { toolsAdded: tools } : {},
    timestamp: 0
  };
}
function normalizeContext(context) {
  const initialMessage = createInitialSystemMessage(context.systemPrompt, context.tools);
  const messages = initialMessage ? [initialMessage, ...context.messages] : context.messages;
  return { messages };
}
function isSystemMessage(message) {
  return message.role === "system";
}
function getInitialSystemMessage(messages) {
  const first = messages[0];
  return first && isSystemMessage(first) ? first : void 0;
}
function withoutInitialSystemMessage(messages) {
  return getInitialSystemMessage(messages) ? messages.slice(1) : messages;
}
function getCurrentTools(messages) {
  const tools = /* @__PURE__ */ new Map();
  for (const message of messages) {
    if (!isSystemMessage(message))
      continue;
    for (const tool of message.toolsRemoved ?? [])
      tools.delete(tool.name);
    for (const tool of message.toolsAdded ?? [])
      tools.set(tool.name, tool);
  }
  return [...tools.values()];
}
function getCurrentSystemMessage(messages) {
  const content = [];
  const sections = /* @__PURE__ */ new Map();
  let timestamp;
  for (const message of messages) {
    if (!isSystemMessage(message))
      continue;
    timestamp ??= message.timestamp;
    const text = contentText(message.content);
    if (text.length > 0)
      content.push(text);
    for (const [name, value] of Object.entries(message.sections ?? {})) {
      if (value === null)
        sections.delete(name);
      else
        sections.set(name, value);
    }
  }
  const tools = getCurrentTools(messages);
  if (timestamp === void 0 && tools.length === 0)
    return void 0;
  return {
    role: "system",
    content: content.join("\n\n"),
    ...sections.size > 0 ? { sections: Object.fromEntries(sections) } : {},
    ...tools.length > 0 ? { toolsAdded: tools } : {},
    timestamp: timestamp ?? 0
  };
}
function getCurrentSystemPrompt(messages) {
  const message = getCurrentSystemMessage(messages);
  return message ? getSystemMessageText(message) : "";
}
function collapseSystemMessages(context) {
  const head = getCurrentSystemMessage(context.messages);
  const messages = context.messages.filter((message) => message.role !== "system");
  return { messages: head ? [head, ...messages] : messages };
}
function resolveTranscript(context, supportsMidConvoSystemMessages) {
  return supportsMidConvoSystemMessages ? context : collapseSystemMessages(context);
}
function toToolDeclaration(tool) {
  return {
    name: tool.name,
    description: tool.description,
    parameters: JSON.parse(JSON.stringify(tool.parameters)),
    ...tool.constrainedSampling === void 0 ? {} : { constrainedSampling: tool.constrainedSampling }
  };
}
function declarationsEqual(left, right) {
  return JSON.stringify(toToolDeclaration(left)) === JSON.stringify(toToolDeclaration(right));
}
function getToolStateChanges(previous, current) {
  const previousTools = new Map(previous.map((tool) => [tool.name, tool]));
  const currentTools = new Map(current.map((tool) => [tool.name, tool]));
  return {
    toolsAdded: current.filter((tool) => {
      const previousTool = previousTools.get(tool.name);
      return previousTool === void 0 || !declarationsEqual(previousTool, tool);
    }).map(toToolDeclaration),
    toolsRemoved: previous.filter((tool) => {
      const currentTool = currentTools.get(tool.name);
      return currentTool === void 0 || !declarationsEqual(tool, currentTool);
    }).map((tool) => ({ name: tool.name }))
  };
}
function getDeclaredTools(messages) {
  const definitions = /* @__PURE__ */ new Map();
  for (const message of messages) {
    if (!isSystemMessage(message))
      continue;
    for (const tool of message.toolsAdded ?? [])
      definitions.set(tool.name, tool);
  }
  return [...definitions.values()];
}
function hasToolRedefinitions(messages) {
  const declared = /* @__PURE__ */ new Map();
  for (const message of messages) {
    if (!isSystemMessage(message))
      continue;
    for (const tool of message.toolsAdded ?? []) {
      const previous = declared.get(tool.name);
      if (previous !== void 0 && !declarationsEqual(previous, tool))
        return true;
      declared.set(tool.name, tool);
    }
  }
  return false;
}
function hasNonAdditiveToolChanges(messages) {
  const declared = /* @__PURE__ */ new Set();
  for (const message of messages) {
    if (!isSystemMessage(message))
      continue;
    if ((message.toolsRemoved?.length ?? 0) > 0)
      return true;
    for (const tool of message.toolsAdded ?? []) {
      if (declared.has(tool.name))
        return true;
      declared.add(tool.name);
    }
  }
  return false;
}
function resolveTranscriptTools(messages, supportsToolAdditions) {
  const anchorsAdditions = supportsToolAdditions && !hasNonAdditiveToolChanges(messages);
  return {
    requestTools: anchorsAdditions ? getInitialSystemMessage(messages)?.toolsAdded ?? [] : getCurrentTools(messages),
    anchorsAdditions
  };
}

// target/vendor-pi/pi-ai/dist/models.js
var KNOWN_MODEL_TYPES = { chat: true, image: true, classifier: true };
function hasKnownModelType(model) {
  return Object.hasOwn(KNOWN_MODEL_TYPES, getModelType(model));
}
function createProvider(input) {
  const single = input.api && typeof input.api.stream === "function" ? input.api : void 0;
  const byApi = single || !input.api ? void 0 : input.api;
  const images = input.images;
  const classifiers = input.classifiers;
  const streams = single ? [single] : Object.values(byApi ?? {}).filter((entry) => entry !== void 0);
  const imageImplementations = Object.values(images ?? {}).filter((entry) => entry !== void 0);
  const classifierImplementations = Object.values(classifiers ?? {}).filter((entry) => entry !== void 0);
  if (streams.length === 0 && imageImplementations.length === 0 && classifierImplementations.length === 0) {
    throw new Error(`Provider ${input.id}: at least one of "api", "images", or "classifiers" is required.`);
  }
  const baselineModels = input.models;
  let dynamicModels = [];
  const fetchModels = input.fetchModels;
  const currentModels = () => {
    const merged = [...baselineModels];
    for (const model of dynamicModels) {
      const index = merged.findIndex((entry) => getModelType(entry) === getModelType(model) && entry.id === model.id);
      if (index >= 0)
        merged[index] = model;
      else
        merged.push(model);
    }
    return merged;
  };
  const apiFor = (model) => single ?? byApi?.[model.api];
  const dispatch = (model, run) => {
    const streams2 = apiFor(model);
    if (!streams2) {
      return lazyStream(model, async () => {
        throw new ModelsError("stream", `Provider ${input.id} has no API implementation for "${model.api}"`);
      });
    }
    return run(streams2);
  };
  const provider = {
    id: input.id,
    name: input.name ?? input.id,
    baseUrl: input.baseUrl,
    headers: input.headers,
    auth: input.auth,
    getModels: () => currentModels().filter((model) => isModelType(model, "chat")),
    getAllModels: currentModels,
    refreshModels: fetchModels ? async (context) => {
      if (context.stored) {
        const restored = context.stored.models.filter((model) => model.provider === input.id).map((model) => model);
        if (!await context.publish({
          update: () => {
            dynamicModels = restored;
          }
        })) {
          return;
        }
      }
      if (!context.allowNetwork || context.signal.aborted)
        return;
      const fetched = await fetchModels(context);
      if (context.signal.aborted)
        return;
      const refreshed = fetched.filter(hasKnownModelType);
      await context.publish({
        persist: { models: refreshed, checkedAt: Date.now() },
        update: () => {
          dynamicModels = refreshed;
        }
      });
    } : void 0,
    filterModels: input.filterModels,
    filterAllModels: input.filterAllModels,
    stream: (model, context, options) => dispatch(model, (streams2) => streams2.stream(model, context, options)),
    streamSimple: (model, context, options) => dispatch(model, (streams2) => streams2.streamSimple(model, context, options))
  };
  if (streams.some((entry) => entry.fetchDeferred !== void 0)) {
    provider.fetchDeferred = (model, handle, options) => lazyStream(model, async () => {
      const implementation = apiFor(model);
      if (!implementation?.fetchDeferred) {
        throw new ModelsError("provider", `Provider ${input.id} does not support deferred responses for "${model.api}"`);
      }
      return implementation.fetchDeferred(model, handle, options);
    });
  }
  if (streams.some((entry) => entry.cancelDeferred !== void 0)) {
    provider.cancelDeferred = async (model, handle, options) => {
      const implementation = apiFor(model);
      if (!implementation?.cancelDeferred) {
        throw new ModelsError("provider", `Provider ${input.id} cannot cancel deferred responses for "${model.api}"`);
      }
      await implementation.cancelDeferred(model, handle, options);
    };
  }
  if (images && imageImplementations.length > 0) {
    provider.generateImages = async (model, context, options) => {
      const implementation = images[model.api];
      if (!implementation) {
        return imageErrorResult(model, new ModelsError("provider", `Provider ${input.id} has no image generation implementation for "${model.api}"`));
      }
      return implementation.generateImages(model, context, options);
    };
  }
  if (classifiers && classifierImplementations.length > 0) {
    provider.classify = async (model, context, options) => {
      const implementation = classifiers[model.api];
      if (!implementation) {
        return classifierErrorResult(model, new ModelsError("provider", `Provider ${input.id} has no classifier implementation for "${model.api}"`));
      }
      return implementation.classify(model, context, options);
    };
  }
  return provider;
}
function hasApi(model, api) {
  return isModelType(model, "chat") && model.api === api;
}
function calculateCost(model, usage) {
  const inputTokens = usage.input + usage.cacheRead + usage.cacheWrite;
  let rates = model.cost;
  let matchedThreshold = -1;
  for (const tier of model.cost.tiers ?? []) {
    if (inputTokens > tier.inputTokensAbove && tier.inputTokensAbove > matchedThreshold) {
      rates = tier;
      matchedThreshold = tier.inputTokensAbove;
    }
  }
  const longWrite = usage.cacheWrite1h ?? 0;
  const shortWrite = usage.cacheWrite - longWrite;
  usage.cost.input = rates.input / 1e6 * usage.input;
  usage.cost.output = rates.output / 1e6 * usage.output;
  usage.cost.cacheRead = rates.cacheRead / 1e6 * usage.cacheRead;
  usage.cost.cacheWrite = (rates.cacheWrite * shortWrite + rates.input * 2 * longWrite) / 1e6;
  usage.cost.total = usage.cost.input + usage.cost.output + usage.cost.cacheRead + usage.cost.cacheWrite;
  return usage.cost;
}
var EXTENDED_THINKING_LEVELS = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];
function getSupportedThinkingLevels(model) {
  if (!model.reasoning)
    return ["off"];
  return EXTENDED_THINKING_LEVELS.filter((level) => {
    const mapped = model.thinkingLevelMap?.[level];
    if (mapped === null)
      return false;
    if (level === "xhigh" || level === "max")
      return mapped !== void 0;
    return true;
  });
}
function clampThinkingLevel(model, level) {
  const availableLevels = getSupportedThinkingLevels(model);
  if (availableLevels.includes(level))
    return level;
  const requestedIndex = EXTENDED_THINKING_LEVELS.indexOf(level);
  if (requestedIndex === -1)
    return availableLevels[0] ?? "off";
  for (let i = requestedIndex; i < EXTENDED_THINKING_LEVELS.length; i++) {
    const candidate = EXTENDED_THINKING_LEVELS[i];
    if (availableLevels.includes(candidate))
      return candidate;
  }
  for (let i = requestedIndex - 1; i >= 0; i--) {
    const candidate = EXTENDED_THINKING_LEVELS[i];
    if (availableLevels.includes(candidate))
      return candidate;
  }
  return availableLevels[0] ?? "off";
}
function modelsAreEqual(a, b) {
  if (!a || !b)
    return false;
  return getModelType(a) === getModelType(b) && a.id === b.id && a.provider === b.provider;
}

export {
  lazyStream,
  lazyApi,
  operationSignal,
  raceWithAbortSignal,
  InMemoryCredentialStore,
  ModelsError,
  InMemoryModelsStore,
  getModelType,
  isModelType,
  assertChatModel,
  assertImageModel,
  assertClassifierModel,
  imageErrorResult,
  classifierErrorResult,
  contentText,
  getSystemMessageText,
  renderSystemMessageUpdate,
  createInitialSystemMessage,
  normalizeContext,
  getInitialSystemMessage,
  withoutInitialSystemMessage,
  getCurrentTools,
  getCurrentSystemMessage,
  getCurrentSystemPrompt,
  collapseSystemMessages,
  resolveTranscript,
  toToolDeclaration,
  declarationsEqual,
  getToolStateChanges,
  getDeclaredTools,
  hasToolRedefinitions,
  hasNonAdditiveToolChanges,
  resolveTranscriptTools,
  createProvider,
  hasApi,
  calculateCost,
  getSupportedThinkingLevels,
  clampThinkingLevel,
  modelsAreEqual
};
