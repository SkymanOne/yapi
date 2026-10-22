import {
	AssistantMessageFrameEncoder, InMemoryCredentialStore, InMemoryModelsStore, appendAssistantMessageDiagnostic, cleanupSessionResources,
	createAssistantMessageDiagnostic, createModels, declarationsEqual, defaultProviderAuthContext, extractDiagnosticError, fauxAssistantMessage,
	fauxProvider, fauxText, fauxThinking, fauxToolCall, getDeclaredTools, getOverflowPatterns, getToolStateChanges, hasNonAdditiveToolChanges,
	hasToolRedefinitions, isContextOverflow, isRecoverableLength, isRetryableAssistantError, reduceAssistantMessageFrames,
	registerSessionResourceCleanup, renderSystemMessageUpdate, resolveTranscriptTools, retryAssistantCall, retryDelayMs, toToolDeclaration,
} from "@earendil-works/pi-ai";
import { findEnvKeys, generateImages, getImageModel, getImageModels, getImageProviders, getImagesApiProvider, registerFauxProvider, completeSimple } from "@earendil-works/pi-ai/compat";
import { builtinProviders, builtinModels, getBuiltinImageModels, getBuiltinClassifierModels, getBuiltinModelDataGeneratedAt } from "@earendil-works/pi-ai/providers/all";
import { OPENROUTER_IMAGE_MODELS } from "@earendil-works/pi-ai/providers/openrouter.models";
import { anthropicProvider } from "@earendil-works/pi-ai/providers/anthropic";
import { cloudflareAIGatewayAuth, cloudflareWorkersAIAuth } from "@earendil-works/pi-ai/providers/cloudflare-auth";
import { resolveCloudflareModel } from "@earendil-works/pi-ai/providers/cloudflare-stream";
import { getRadiusModelsFromConfig, normalizeRadiusGatewayUrl } from "@earendil-works/pi-ai/providers/radius-config";
import { withOpenCodeSessionHeader } from "@earendil-works/pi-ai/providers/opencode-headers";
import { createAiBindingFetch } from "@earendil-works/pi-ai/api/cloudflare-ai-binding";
import { appendGrammarToolInputJsonDelta, createGrammarToolInputProperties, getGrammarToolInput, getJsonSchemaToolParameters, makeStrictJsonSchema, resolveGrammarConstrainedSampling, resolveJsonSchemaStrictSampling } from "@earendil-works/pi-ai/api/constrained-sampling";
import { buildCopilotDynamicHeaders, hasCopilotVisionInput, inferCopilotInitiator } from "@earendil-works/pi-ai/api/github-copilot-headers";
import * as google from "@earendil-works/pi-ai/api/google-shared";
import * as completions from "@earendil-works/pi-ai/api/openai-completions";
import { answerFromProbabilities, labelProbabilities, llamaServerRoot, peakConfidence, renderQuestion } from "@earendil-works/pi-ai/api/llama-cpp-classify";
import { llamaCppClassifyApi } from "@earendil-works/pi-ai/api/llama-cpp-classify.lazy";
import { clampOpenAIPromptCacheKey } from "@earendil-works/pi-ai/api/openai-prompt-cache";
import { convertResponsesMessages, convertResponsesTools, processResponsesStream } from "@earendil-works/pi-ai/api/openai-responses-shared";
import { generateImages as openRouterImages } from "@earendil-works/pi-ai/api/openrouter-images";
import { PiMessagesResponseError } from "@earendil-works/pi-ai/api/pi-messages";
import { adjustMaxTokensForThinking, buildBaseOptions, clampMaxTokensToContext, clampReasoning, clampThinkingBudgetToAnswerRoom, thinkingBudgetForLevel } from "@earendil-works/pi-ai/api/simple-options";
import { isRecord } from "@earendil-works/pi-ai/api/system-one-shared";
import { transformMessages } from "@earendil-works/pi-ai/api/transform-messages";
import { combineAbortSignals } from "@earendil-works/pi-ai/utils/abort-signals";
import { operationSignal, raceWithAbortSignal } from "@earendil-works/pi-ai/utils/abort";
import { formatProviderError, normalizeProviderError, safeJsonStringify, truncateErrorText } from "@earendil-works/pi-ai/utils/error-body";
import { calculateContextTokens, estimateContextTokens, estimateMessageTokens, estimateTextAndImageContentTokens, estimateTextTokens } from "@earendil-works/pi-ai/utils/estimate";
import { shortHash } from "@earendil-works/pi-ai/utils/hash";
import { headersToRecord, providerHeadersToRecord } from "@earendil-works/pi-ai/utils/headers";
import { assertImageModel, classifierErrorResult, imageErrorResult } from "@earendil-works/pi-ai/utils/model-operations";
import { resolveHttpProxyUrlForTarget } from "@earendil-works/pi-ai/utils/node-http-proxy";
import { oauthErrorHtml, oauthSuccessHtml } from "@earendil-works/pi-ai/utils/oauth-page";
import { getPiUserAgent } from "@earendil-works/pi-ai/utils/pi-user-agent";
import { retryProviderRequest } from "@earendil-works/pi-ai/utils/provider-retry";
import { sanitizeSurrogates } from "@earendil-works/pi-ai/utils/sanitize-unicode";

const usage = { input: 10, output: 0, cacheRead: 5, cacheWrite: 0, totalTokens: 15, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } };
const failed = (errorMessage, provider = "anthropic") => ({ role: "assistant", content: [], provider, usage, stopReason: "error", errorMessage, timestamp: 0 });
const user = { role: "user", content: [{ type: "text", text: "Describe this picture" }, { type: "image", data: "aGk=", mimeType: "image/png" }], timestamp: 0 };
const tool = { name: "read", description: "Reads a file", parameters: { type: "object", properties: { path: { type: "string", format: "uri" } }, required: ["path"] } };
const model = { id: "m", name: "M", api: "openai-completions", provider: "openai", baseUrl: "https://api.openai.com/v1", reasoning: true, input: ["text", "image"], cost: { input: 1, output: 2, cacheRead: 0, cacheWrite: 0 }, contextWindow: 1000, maxTokens: 400 };
const withoutTime = ({ timestamp, ...rest }) => rest;
const thrown = (run) => {
	try {
		run();
		return "ran";
	} catch (error) {
		return `${error.name}: ${error.message}`;
	}
};

async function probe() {
	const cleaned = [];
	const stop = registerSessionResourceCleanup((id) => cleaned.push(id));
	cleanupSessionResources("s1");
	stop();
	cleanupSessionResources("s2");

	// Faux streams, framed and reduced again.
	const faux = registerFauxProvider({ tokenSize: { min: 2, max: 2 } });
	faux.setResponses([fauxAssistantMessage([fauxThinking("hmm"), fauxText("hello there"), fauxToolCall("read", { path: "a" }, { id: "t1" })], { timestamp: 1 })]);
	const encoder = new AssistantMessageFrameEncoder();
	const frames = [];
	const stream = (await import("@earendil-works/pi-ai/compat")).streamSimple(faux.getModel(), { messages: [user] });
	for await (const event of stream) {
		const frame = encoder.encode(event);
		if (frame) frames.push(frame);
	}
	const reduced = reduceAssistantMessageFrames(frames);
	faux.unregister();
	const own = fauxProvider();
	const models = createModels();
	models.setProvider(own.provider);
	own.setResponses([fauxAssistantMessage("from models", { timestamp: 2 })]);
	const viaModels = await models.completeSimple(own.getModel(), { messages: [user] });

	const credentials = new InMemoryCredentialStore();
	await credentials.modify("p", async () => ({ type: "api_key", key: "k" }));
	const modelsStore = new InMemoryModelsStore();
	await modelsStore.write("p", { models: [], checkedAt: 3 });
	const builtins = builtinModels();
	const anthropic = anthropicProvider();

	const responses = processResponsesStream(
		(async function* () {
			yield { type: "response.created", response: { id: "r1" } };
			yield { type: "response.output_item.added", item: { type: "message", id: "m1", role: "assistant", content: [] } };
			yield { type: "response.content_part.added", part: { type: "output_text", text: "" } };
			yield { type: "response.output_text.delta", delta: "Hi" };
			yield { type: "response.output_item.done", item: { type: "message", id: "m1", role: "assistant", content: [{ type: "output_text", text: "Hi" }] } };
			yield { type: "response.completed", response: { id: "r1", status: "completed", usage: { input_tokens: 3, output_tokens: 1, total_tokens: 4 } } };
		})(),
		{ role: "assistant", content: [], api: "openai-responses", provider: "openai", model: "m", usage: structuredClone(usage), stopReason: "stop", timestamp: 0 },
		{ push: () => {} },
		{ ...model, api: "openai-responses" },
	).then(() => "processed", (error) => error.message);

	let attempts = 0;
	const retried = await retryProviderRequest(async () => {
		attempts++;
		if (attempts === 1) throw Object.assign(new Error("busy"), { status: 503, headers: new Headers({ "retry-after-ms": "1" }) });
		return "ok";
	}, { maxRetries: 2, maxRetryDelayMs: 1 }).catch((error) => error.message);
	const retriedCall = await retryAssistantCall(async () => fauxAssistantMessage("again", { timestamp: 4 }), { enabled: true, maxRetries: 1, baseDelayMs: 1 });

	const imageModel = getImageModel("openrouter", "black-forest-labs/flux.2-flex");
	const images = await generateImages(imageModel, { input: [{ type: "text", text: "a cat" }] }, {});
	const openRouter = await openRouterImages(imageModel, { input: [{ type: "text", text: "a cat" }] }, {});
	const llama = llamaCppClassifyApi();

	return {
		sessionResources: cleaned,
		faux: [reduced.content, reduced.stopReason, frames.map((frame) => frame.type).join(" "), withoutTime(viaModels).content],
		stores: [await credentials.read("p"), await modelsStore.read("p")],
		overflow: [isContextOverflow(failed("prompt is too long: 213462 tokens > 200000 maximum")), isContextOverflow({ ...failed(""), stopReason: "stop" }, 12), isRecoverableLength({ stopReason: "length", usage }, 10), getOverflowPatterns().length],
		retry: [isRetryableAssistantError(failed("529 overloaded")), isRetryableAssistantError(failed("insufficient_quota")), retryDelayMs({ baseDelayMs: 1000 }, 4), retryDelayMs({ baseDelayMs: 1000, maxAgentDelayMs: 5000 }, 10), withoutTime(retriedCall).content],
		diagnostics: [withoutTime(createAssistantMessageDiagnostic("d", "bad", { at: 1 })), (() => { const message = failed("x"); appendAssistantMessageDiagnostic(message, createAssistantMessageDiagnostic("e", "plain")); return message.diagnostics.map(withoutTime); })(), extractDiagnosticError(42)],
		transcript: [toToolDeclaration(tool), declarationsEqual(tool, { ...tool }), getToolStateChanges([tool], [{ ...tool, description: "Reads" }]), getDeclaredTools([{ role: "system", content: "", toolsAdded: [tool], timestamp: 0 }]).length, hasToolRedefinitions([{ role: "system", content: "", toolsAdded: [tool], timestamp: 0 }, { role: "system", content: "", toolsAdded: [{ ...tool, description: "x" }], timestamp: 1 }]), hasNonAdditiveToolChanges([{ role: "system", content: "", toolsRemoved: [tool], timestamp: 0 }]), resolveTranscriptTools([{ role: "system", content: "", toolsAdded: [tool], timestamp: 0 }], false).length, renderSystemMessageUpdate({ role: "system", content: "Be brief", sections: { a: "One" }, timestamp: 0 })],
		models: [builtinProviders().length, builtins.getModels("anthropic").length > 0, [anthropic.id, anthropic.name, anthropic.baseUrl, anthropic.auth.oauth.name], getBuiltinModelDataGeneratedAt(), models.getProvider("faux")?.name, await models.checkAuth("faux")],
		catalog: [getImageModels("openrouter").length, getImageProviders(), imageModel.name, getBuiltinImageModels("openrouter").length, Object.keys(OPENROUTER_IMAGE_MODELS).length, getBuiltinClassifierModels("typesafe").map((entry) => entry.id), typeof getImagesApiProvider("openrouter-images")?.generateImages, imageModel, builtins.getModel("anthropic", "claude-sonnet-4-5")],
		images: [withoutTime(images), withoutTime(openRouter)],
		envKeys: [findEnvKeys("anthropic", { ANTHROPIC_OAUTH_TOKEN: "t", ANTHROPIC_API_KEY: "k" }), findEnvKeys("nope", {})],
		auth: [cloudflareWorkersAIAuth().name, cloudflareAIGatewayAuth().name, typeof defaultProviderAuthContext().env, resolveCloudflareModel({ baseUrl: "https://x/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}" }, { CLOUDFLARE_ACCOUNT_ID: "acc" }).baseUrl],
		radius: [normalizeRadiusGatewayUrl("radius.pi.dev/"), getRadiusModelsFromConfig("radius", { models: [{ id: "a", name: "A", api: "pi-messages", baseUrl: "https://r", input: ["text"], contextWindow: 10, maxTokens: 5, reasoning: false, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 } }] }).map((entry) => [entry.id, entry.provider])],
		constrained: [makeStrictJsonSchema(tool.parameters), getJsonSchemaToolParameters(tool, true), resolveJsonSchemaStrictSampling(tool, true), resolveGrammarConstrainedSampling(tool, true), getGrammarToolInput("t", { input: "x" }, "input"), appendGrammarToolInputJsonDelta({ input: "", closed: false }, "input", "ab", true), createGrammarToolInputProperties([tool], true)],
		copilot: [inferCopilotInitiator([user]), hasCopilotVisionInput([user]), buildCopilotDynamicHeaders({ messages: [user], hasImages: true })],
		google: [google.convertTools([tool]), google.mapStopReason("MAX_TOKENS"), google.mapStopReasonString("SAFETY"), google.mapToolChoice("required"), google.isThinkingPart({ thought: true }), google.requiresToolCallId("claude-x"), google.toGoogleThinkingLevel("minimal"), google.convertMessages({ ...model, api: "google-generative-ai" }, { messages: [user] })],
		completions: completions.convertMessages(model, { messages: [user] }, {}),
		responses: [convertResponsesMessages({ ...model, api: "openai-responses" }, { messages: [user] }, new Set(["openai"])), convertResponsesTools([tool]), await responses],
		llama: [llamaServerRoot("http://localhost:8080/v1/"), labelProbabilities([-0.1, -2.3], 1), peakConfidence([0.9, 0.1]), renderQuestion({ state: { sky: "grey" }, questions: { q: { type: "choice", instructions: "Weather?", criteria: { sun: "clear", rain: "" } } } }, "q"), answerFromProbabilities({ type: "choice" }, ["sun", "rain"], [0.3, 0.7]), typeof llama.classify],
		options: [thinkingBudgetForLevel("medium"), clampReasoning("xhigh"), clampThinkingBudgetToAnswerRoom(5000, 4000), adjustMaxTokensForThinking(1000, 32000, "high"), clampMaxTokensToContext(model, { messages: [user] }, 5000), buildBaseOptions(model, { messages: [user] }, { maxTokens: 50 }, "key")],
		transform: transformMessages([user, { role: "assistant", content: [fauxToolCall("read", {}, { id: "a|b" })], api: "openai-responses", provider: "openai", model: "x", usage, stopReason: "toolUse", timestamp: 0 }], model, (id) => id.replace("|", "_")).map((message) => message.role),
		errors: [truncateErrorText("abcdef", 3), safeJsonStringify({ a: 1n }), formatProviderError(normalizeProviderError(Object.assign(new Error("boom"), { status: 500 })), "Prefix"), thrown(() => assertImageModel(model)), withoutTime(imageErrorResult(model, new Error("x"), true)), withoutTime(classifierErrorResult(model, "y"))],
		small: [shortHash("abc"), sanitizeSurrogates("a\uD800b"), clampOpenAIPromptCacheKey("k".repeat(80)).length, headersToRecord(new Headers({ "X-A": "1" })), providerHeadersToRecord({ a: "1" }, { a: null, b: "2" }), isRecord({}), new PiMessagesResponseError("nope", 429).message, getPiUserAgent().startsWith("pi ("), resolveHttpProxyUrlForTarget("https://example.com", { HTTPS_PROXY: "http://proxy:8080" })],
		pages: [oauthSuccessHtml("Done").length, oauthErrorHtml("Failed", "why").includes("why")],
		signals: [combineAbortSignals([AbortSignal.abort("x"), new AbortController().signal]).signal.aborted, operationSignal().aborted, await raceWithAbortSignal(Promise.resolve(5), new AbortController().signal)],
		subpaths: [typeof completions.stream, completions.convertMessages === google.convertMessages, typeof createAiBindingFetch, typeof withOpenCodeSessionHeader({ stream: () => {}, streamSimple: () => {} }).stream, [retried, attempts]],
	};
}

export default async function (pi) {
	const results = await probe();
	pi.registerCommand("probe", { description: JSON.stringify(results), handler: async () => {} });
}
