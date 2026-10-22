// `@earendil-works/pi-ai` and its subpaths for extensions in yapi: pi-ai
// 1.0.0's own modules (MIT, Copyright (c) Mario Zechner), bundled as
// `yapi:vendor/pi-ai`, in which wire APIs, image generation and the generated
// catalogs run on yapi's providers through the host. This module adds what
// yapi provides in pi's place: completions on yapi's providers, the API
// provider registry they consult, and JSON parsing and tool validation from
// the host.
import { Value } from "typebox/value";
import {
	AssistantMessageEventStream,
	cleanupSessionResources,
	createFauxCore,
	getBuiltinModel,
	getBuiltinModels,
	getBuiltinProviders,
	imageErrorResult,
	normalizeContext,
} from "yapi:vendor/pi-ai";

export * from "yapi:vendor/pi-ai";
export { Type } from "typebox";

const yapi = globalThis.__yapi;

// The host ends a disposed session's resources, as pi's `AgentSession.dispose` does.
yapi.cleanupSessionResources = cleanupSessionResources;

// ----- completions on yapi's providers --------------------------------------------------------------
/**
 * Streams with yapi's implementation of `api`, or of the model's API. The
 * host sends the events that have arrived each time it is asked; the live
 * message is rebuilt here from them.
 */
function hostStream(model, context, options, api, kind = "ai.stream") {
	const stream = new AssistantMessageEventStream();
	const { signal, onPayload, onResponse, onProviderStreamEvent, ...rest } = options ?? {};
	// The session's hooks of the extension stream this request serves.
	const hooks = [onPayload, onResponse, onProviderStreamEvent].find((hook) => hook?.yapiStream !== undefined)?.yapiStream;
	(async () => {
		const id = await yapi.op(kind, { api, model, context: normalizeContext(context), options: rest, hooks });
		const abort = () => yapi.request("ai.abort", { id });
		if (signal?.aborted) abort();
		else signal?.addEventListener("abort", abort, { once: true });
		try {
			let partial;
			const arguments_ = [];
			for (;;) {
				const batch = await yapi.op("ai.next", { id });
				if (batch.length === 0) return;
				for (const wire of batch) {
					if (wire.type === "start") {
						partial = wire.message;
						stream.push({ type: "start", partial });
					} else if (wire.type === "done") stream.push({ type: "done", reason: wire.message.stopReason, message: wire.message });
					else if (wire.type === "error") stream.push({ type: "error", reason: wire.message.stopReason, error: wire.message });
					else {
						const { id: callId, toolName, ...event } = wire.event;
						const index = event.contentIndex;
						const content = partial.content;
						switch (event.type) {
							case "text_start":
								content[index] = { type: "text", text: "" };
								break;
							case "text_delta":
								content[index].text += event.delta;
								break;
							case "text_end":
								content[index].text = event.content;
								break;
							case "thinking_start":
								content[index] = { type: "thinking", thinking: "" };
								break;
							case "thinking_delta":
								content[index].thinking += event.delta;
								break;
							case "thinking_end":
								content[index].thinking = event.content;
								break;
							case "toolcall_start":
								content[index] = { type: "toolCall", id: callId, name: toolName, arguments: {} };
								arguments_[index] = "";
								break;
							case "toolcall_delta":
								arguments_[index] += event.delta;
								content[index].arguments = parseStreamingJson(arguments_[index]);
								break;
							case "toolcall_end":
								content[index] = event.toolCall;
								break;
						}
						partial.usage = wire.usage;
						stream.push({ ...event, partial });
					}
				}
			}
		} finally {
			signal?.removeEventListener("abort", abort);
		}
	})().catch((error) => stream.push({ type: "error", reason: "error", error: yapi.setupError(model, error) }));
	return stream;
}
/** `ctx.modelRegistry`'s streams: the session's provider of `model`, with the registry's credentials. */
yapi.registryStream = (model, context, options) => hostStream(model, context, options, undefined, "models.stream");
/** yapi's implementation of wire API `api`, which pi-ai's wire API modules stream with in the bundle. */
const hostApi = (yapi.hostApi = (api) => {
	const run = (model, context, options) => hostStream(model, context, options, api);
	return { stream: run, streamSimple: run };
});

let nextImages = 0;
/**
 * pi-ai's `generateImages` of `api/openrouter-images` on yapi's port of it.
 * Failures, and the host refusing the request, are reported in the result.
 */
yapi.generateImages = async (model, context, options) => {
	const { signal, ...rest } = options ?? {};
	const id = nextImages++;
	const result = yapi.op("ai.images", { id, model, context, options: rest });
	const abort = () => yapi.request("ai.imagesAbort", { id });
	if (signal?.aborted) abort();
	else signal?.addEventListener("abort", abort, { once: true });
	try {
		return await result;
	} catch (error) {
		return imageErrorResult(model, error, signal?.aborted);
	} finally {
		signal?.removeEventListener("abort", abort);
	}
};

// ----- API providers -----------------------------------------------------------------------------
// pi-ai's registry. A stream an extension registers for an API serves its
// own calls to `stream` and `streamSimple`, and the session's models of that
// API when yapi has no provider for it.
const BUILTIN_APIS = [
	"anthropic-messages",
	"openai-completions",
	"openai-responses",
	"openai-codex-responses",
	"azure-openai-responses",
	"google-generative-ai",
	"google-vertex",
	"mistral-conversations",
	"bedrock-converse-stream",
];
const apiProviders = new Map();
// The extension host streams session models through these.
yapi.apiProviders = apiProviders;
const forApi = (api, fn) => (model, context, options) => {
	if (model.api !== api) throw new Error(`Mismatched api: ${model.api} expected ${api}`);
	return fn(model, context, options);
};
export function registerApiProvider(provider, sourceId) {
	apiProviders.set(provider.api, {
		provider: { api: provider.api, stream: forApi(provider.api, provider.stream), streamSimple: forApi(provider.api, provider.streamSimple) },
		sourceId,
	});
}
export function getApiProvider(api) {
	return apiProviders.get(api)?.provider ?? (BUILTIN_APIS.includes(api) ? { api, ...hostApi(api) } : undefined);
}
export const getApiProviders = () => Array.from(apiProviders.values(), (entry) => entry.provider);
export function unregisterApiProviders(sourceId) {
	for (const [api, entry] of apiProviders) if (entry.sourceId === sourceId) apiProviders.delete(api);
}
export function resetApiProviders() {
	apiProviders.clear();
}
export function registerBuiltInApiProviders() {}
export function registerFauxProvider(options = {}) {
	const core = createFauxCore(options);
	const sourceId = `faux-provider-${Math.random().toString(36).slice(2, 10)}`;
	registerApiProvider({ api: core.api, stream: core.stream, streamSimple: core.streamSimple }, sourceId);
	return {
		api: core.api,
		models: core.models,
		getModel: core.getModel,
		state: core.state,
		setResponses: core.setResponses,
		appendResponses: core.appendResponses,
		getPendingResponseCount: core.getPendingResponseCount,
		unregister() {
			unregisterApiProviders(sourceId);
		},
	};
}
export function streamSimple(model, context, options) {
	const custom = apiProviders.get(model?.api);
	return custom ? custom.provider.streamSimple(model, context, options) : hostStream(model, context, options);
}
export function stream(model, context, options) {
	const custom = apiProviders.get(model?.api);
	return custom ? custom.provider.stream(model, context, options) : hostStream(model, context, options);
}
export const complete = (model, context, options) => streamSimple(model, context, options).result();
export const completeSimple = complete;

// pi-ai's deprecated static catalog reads.
export const getModel = getBuiltinModel;
export const getModels = getBuiltinModels;
export const getProviders = getBuiltinProviders;
/** The provider's key from the environment, as yapi's own requests find it. */
export const getEnvApiKey = (provider) => yapi.request("models.envApiKey", { provider }) ?? undefined;

// ----- JSON and tool validation from the host ----------------------------------------------------
export const repairJson = (json) => yapi.request("json.repair", { text: json });
export const parseJsonWithRepair = (json) => yapi.request("json.parseWithRepair", { text: json });
export const parseStreamingJson = (partial) => yapi.request("json.partial", { text: partial ?? "" });

export function validateToolArguments(tool, toolCall) {
	const args = structuredClone(toolCall.arguments ?? {});
	Value.Convert(tool.parameters, args);
	if (Value.Check(tool.parameters, args)) return args;
	const errors = [...Value.Errors(tool.parameters, args)].map((error) => `  - ${error.instancePath || "root"}: ${error.message}`).join("\n");
	throw new Error(`Validation failed for tool "${toolCall.name}":\n${errors}\n\nReceived arguments:\n${JSON.stringify(toolCall.arguments, null, 2)}`);
}
export function validateToolCall(tools, toolCall) {
	const tool = tools.find((candidate) => candidate.name === toolCall.name);
	if (!tool) throw new Error(`Tool "${toolCall.name}" not found`);
	return validateToolArguments(tool, toolCall);
}

// ----- the host's Codex WebSocket cache ----------------------------------------------------------
export const closeOpenAICodexWebSocketSessions = (sessionId) => {
	yapi.request("codex.closeSessions", { sessionId });
};
export const getOpenAICodexWebSocketDebugStats = (sessionId) => yapi.request("codex.stats", { sessionId }) ?? undefined;
export const resetOpenAICodexWebSocketDebugStats = (sessionId) => {
	yapi.request("codex.resetStats", { sessionId });
};
