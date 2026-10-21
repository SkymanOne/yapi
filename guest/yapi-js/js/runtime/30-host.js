// The extension host inside the guest: loads pi extensions, gives each its
// `pi` API object, and runs its handlers, tools, commands and shortcuts when
// the host dispatches them. Port of `core/extensions/loader.ts` and the
// per-extension part of `runner.ts` in pi `v1.0.0`; the host chains results
// across extensions.
(() => {
	"use strict";
	const yapi = globalThis.__yapi;
	const native = globalThis.__yapi_native;
	const EventEmitter = yapi.EventEmitter;

	const extensions = new Map();
	/** pi's extension cache: the factory of each file whose module loaded. */
	const factories = new Map();
	/** How many times each file was imported. */
	const imports = new Map();
	const flagValues = new Map();
	let bound = false;

	const errorMessage = (error) => (error instanceof Error ? error.message : String(error));
	const errorInfo = (error) => ({ error: errorMessage(error), stack: error instanceof Error ? error.stack : undefined });
	const plain = (value) => (value === undefined ? undefined : JSON.parse(JSON.stringify(value)));
	const orUndefined = (value) => value ?? undefined;

	// ----- event buses --------------------------------------------------------
	/** pi's `createEventBus`. */
	yapi.createEventBus = () => {
		const emitter = new EventEmitter();
		return {
			emit: (channel, data) => {
				emitter.emit(channel, data);
			},
			on: (channel, handler) => {
				const safe = async (data) => {
					try {
						await handler(data);
					} catch (error) {
						console.error(`Event handler error (${channel}):`, error);
					}
				};
				emitter.on(channel, safe);
				return () => emitter.off(channel, safe);
			},
			clear: () => {
				emitter.removeAllListeners();
			},
		};
	};
	// `pi.events`, which every extension shares. The host passes events on to
	// the listeners in other runtimes, as JSON, after the listeners here.
	const eventBus = yapi.createEventBus();
	const emitEvent = (channel, data) => {
		eventBus.emit(channel, data);
		try {
			yapi.request("events.emit", { channel, data });
		} catch {
			// Data JSON cannot carry stays in this runtime, as do all events
			// where no other runtimes run.
		}
	};

	// ----- runtime actions -----------------------------------------------------------
	const notInitialized = () => {
		throw new Error("Extension runtime not initialized. Action methods cannot be called during extension loading.");
	};
	const action = (kind) => (payload) => (bound ? yapi.request(kind, payload) : notInitialized());
	/** Whether the host has the descriptions of the latest run of the factories. */
	let described = false;
	// Provider changes made since then reach the session as it binds, as pi
	// applies them once its runner binds.
	const unboundChanges = [];
	const providerChange = (change) => (bound ? change() : unboundChanges.push(change));
	// The requests behind `sendMessage` and `sendUserMessage`.
	const messageRequest = (message, options) => ({ message: plain(message), options: plain(options) });
	const userMessageRequest = (content, options) => ({ content: plain(content), options: plain(options) });

	// ----- staleness ---------------------------------------------------------------------------
	// pi stops a session's extension API and contexts once another session
	// replaces it. Each session's extensions belong to one generation.
	const STALE =
		"This extension ctx is stale after session replacement or reload. Do not use a captured pi or command ctx after ctx.newSession(), ctx.fork(), ctx.switchSession(), or ctx.reload(). For newSession, fork, and switchSession, move post-replacement work into withSession and use the ctx passed to withSession. For reload, do not use the old ctx after await ctx.reload().";
	let generation = { stale: false };
	const assertActive = (born) => {
		if (born.stale) throw new Error(STALE);
	};

	// ----- MCP servers -----------------------------------------------------------------------
	/** pi's `getMcpServers`: the servers the loaded extensions registered, in registration order. */
	let mcpServerSeq = 0;
	const mcpServers = () =>
		[...extensions.values()]
			.flatMap((extension) =>
				[...extension.mcpServers].map(([name, { config, seq }]) => ({ seq, name, config: plain(config), extensionPath: extension.path })),
			)
			.sort((a, b) => a.seq - b.seq)
			.map(({ seq, ...server }) => server);
	const mcpNamespace = (name) => `mcp__${name.replace(/-/g, "_")}`;
	/** The host connects the servers of the bound session. */
	const reportMcpServers = () => yapi.request("mcp.servers", { servers: mcpServers() });

	// ----- the pi API ------------------------------------------------------------------------
	function createApi(extension) {
		const pendingFlagValues = new Map();
		let state = "loading";
		const born = generation;
		// Every method throws once the extension has failed to load or its
		// session was replaced.
		const active = (methods) => {
			for (const [name, method] of Object.entries(methods)) {
				if (typeof method !== "function") continue;
				methods[name] = (...args) => {
					if (state === "failed") throw new Error(`Extension "${extension.path}" failed to load and its API is no longer active.`);
					assertActive(born);
					return method(...args);
				};
			}
			return methods;
		};
		const api = active({
			on(event, handler) {
				const registered = (...args) => handler(...args);
				const list = extension.handlers.get(event) ?? [];
				list.push(registered);
				extension.handlers.set(event, list);
				return () => {
					const handlers = extension.handlers.get(event);
					if (!handlers) return;
					const index = handlers.indexOf(registered);
					if (index !== -1) handlers.splice(index, 1);
					if (handlers.length === 0) extension.handlers.delete(event);
				};
			},
			registerTool(tool) {
				if (typeof tool.parameters !== "object" || tool.parameters === null || Array.isArray(tool.parameters)) {
					throw new Error(`Tool "${tool.name}" registered by extension "${extension.path}" must define an object parameter schema.`);
				}
				extension.tools.set(tool.name, tool);
				if (bound) yapi.request("tools.refresh", { extension: extension.id, tools: [describeTool(tool)] });
			},
			registerCommand(name, options) {
				if (typeof name !== "string" || name.length === 0) {
					throw new Error(`Command registered by extension "${extension.path}" must have a non-empty string name. Use pi.registerCommand("name", { description, handler }).`);
				}
				if (typeof options?.handler !== "function") {
					throw new Error(`Command "/${name}" registered by extension "${extension.path}" must define handler().`);
				}
				extension.commands.set(name, { name, ...options });
			},
			registerShortcut(shortcut, options) {
				extension.shortcuts.set(shortcut, { shortcut, ...options });
			},
			registerFlag(name, options) {
				if (options.default !== undefined && typeof options.default !== options.type) {
					throw new Error(`Invalid default for flag "${name}": expected ${options.type}, got ${typeof options.default}`);
				}
				extension.flags.set(name, { name, ...options });
				if (options.default !== undefined && !flagValues.has(name)) {
					if (state === "loading") {
						if (!pendingFlagValues.has(name)) pendingFlagValues.set(name, options.default);
					} else flagValues.set(name, options.default);
				}
			},
			registerMessageRenderer(customType, renderer) {
				extension.messageRenderers.set(customType, renderer);
				refreshRenderers(extension);
			},
			registerMarkdownTransformer(transformer) {
				extension.markdownTransformer = transformer;
				refreshRenderers(extension);
			},
			registerEntryRenderer(customType, renderer) {
				extension.entryRenderers.set(customType, renderer);
				refreshRenderers(extension);
			},
			getFlag(name) {
				if (!extension.flags.has(name)) return undefined;
				return flagValues.has(name) ? flagValues.get(name) : pendingFlagValues.get(name);
			},
			sendMessage(message, options) {
				action("session.sendMessage")(messageRequest(message, options));
			},
			sendUserMessage(content, options) {
				if (!bound) notInitialized();
				const payload = userMessageRequest(content, options);
				// pi's `prompt` queues or starts the message after its first await, once the caller's code has run.
				queueMicrotask(() => yapi.request("session.sendUserMessage", payload));
			},
			appendEntry(customType, data) {
				action("session.appendEntry")({ customType, data: plain(data) });
			},
			setSessionName(name) {
				action("session.setName")({ name });
			},
			getSessionName() {
				return action("session.getName")({}) ?? undefined;
			},
			setLabel(entryId, label) {
				action("session.setLabel")({ entryId, label });
			},
			exec(command, args, options) {
				return execCommand(command, args, options?.cwd ?? yapi.cwd, options);
			},
			getActiveTools() {
				return action("tools.getActive")({});
			},
			getAllTools() {
				return action("tools.getAll")({});
			},
			getSettings() {
				return action("settings.get")({});
			},
			setActiveTools(names) {
				action("tools.setActive")({ names });
			},
			getCommands() {
				return action("commands.list")({});
			},
			setModel(model) {
				if (!bound) return Promise.reject(new Error("Extension runtime not initialized"));
				return yapi.op("model.set", { provider: model.provider, id: model.id });
			},
			getThinkingLevel() {
				return action("thinking.get")({});
			},
			setThinkingLevel(level) {
				action("thinking.set")({ level });
			},
			registerProvider(nameOrProvider, config) {
				let provider;
				if (typeof nameOrProvider === "string") {
					if (!config) throw new Error("Provider config is required when registering by name");
					provider = { name: nameOrProvider, config: describeProvider(config) };
				} else provider = { name: nameOrProvider.id, native: true };
				// The code stays here; the host gets the description.
				Object.defineProperty(provider, "implementation", { value: config ?? nameOrProvider });
				extension.providers.push(provider);
				if (!described) return;
				providerChange(() => {
					if (provider.native) provider.config = describeNative(provider.implementation);
					try {
						yapi.request("providers.register", { extension: extension.id, provider });
					} catch (error) {
						extension.providers = extension.providers.filter((entry) => entry !== provider);
						throw error;
					}
				});
			},
			unregisterProvider(name) {
				extension.providers = extension.providers.filter((provider) => provider.name !== name);
				if (!described) return;
				providerChange(() => {
					// pi removes the provider whichever extension registered it.
					for (const other of extensions.values()) other.providers = other.providers.filter((provider) => provider.name !== name);
					yapi.request("providers.unregister", { name });
				});
			},
			registerMcpServer(name, config) {
				// The host checks the config, the grant its transport needs and
				// the servers of other runtimes, while extensions load too, and
				// gives the config with legacy exposure names replaced.
				const validated = yapi.request("mcp.validate", { name, config: plain(config), extension: extension.path });
				const owner = mcpServers().find((server) => server.name === name)?.extensionPath;
				if (owner !== undefined && owner !== extension.path) {
					throw new Error(`MCP server "${name}" is already registered by extension "${owner}"`);
				}
				// Names that differ only in `-` and `_` would share a namespace.
				const clash = mcpServers().find((server) => server.name !== name && mcpNamespace(server.name) === mcpNamespace(name));
				if (clash) throw new Error(`MCP server "${name}" conflicts with registered server "${clash.name}"`);
				// A server registered again keeps its place, as in pi's Map.
				const previous = extension.mcpServers.get(name);
				extension.mcpServers.set(name, { config: validated, seq: previous?.seq ?? mcpServerSeq++ });
				if (!bound) return;
				try {
					reportMcpServers();
				} catch (error) {
					// The host refuses a name, or a tool namespace, that another runtime's extension registered.
					if (previous) extension.mcpServers.set(name, previous);
					else extension.mcpServers.delete(name);
					throw error;
				}
			},
			unregisterMcpServer(name) {
				if (extension.mcpServers.delete(name) && bound) reportMcpServers();
			},
			getMcpServers() {
				return mcpServers();
			},
			registerVirtualModel(model) {
				extension.virtualModels.push(model);
			},
			unregisterVirtualModel(provider, id) {
				extension.virtualModels = extension.virtualModels.filter((model) => model.provider !== provider || model.id !== id);
			},
			events: active({
				emit(channel, data) {
					emitEvent(channel, data);
				},
				on(channel, handler) {
					return eventBus.on(channel, handler);
				},
			}),
			// yapi's own API, which pi lacks: packages call it as `pi.yapi?.…`.
			yapi: active({
				registerCodeRenderer(render) {
					extension.codeRenderer = render;
					refreshRenderers(extension);
				},
			}),
		});
		return {
			api,
			commit() {
				for (const [name, value] of pendingFlagValues) if (!flagValues.has(name)) flagValues.set(name, value);
				state = "active";
			},
			discard() {
				state = "failed";
			},
		};
	}

	/** pi's execCommand: a timeout or `signal` kills the process, and a process that cannot start reports code 1. */
	function execCommand(command, args, cwd, options = {}) {
		return new Promise((resolve) => {
			const proc = globalThis.__yapi_builtins.child_process.spawn(command, args ?? [], { cwd, stdio: ["ignore", "pipe", "pipe"] });
			let stdout = "";
			let stderr = "";
			let killed = false;
			const killProcess = () => {
				if (killed) return;
				killed = true;
				proc.kill("SIGTERM");
				setTimeout(() => {
					if (!proc.killed) proc.kill("SIGKILL");
				}, 5000);
			};
			if (options.signal?.aborted) killProcess();
			else options.signal?.addEventListener("abort", killProcess, { once: true });
			const timeout = options.timeout > 0 ? setTimeout(killProcess, options.timeout) : undefined;
			proc.stdout.on("data", (data) => {
				stdout += data.toString();
			});
			proc.stderr.on("data", (data) => {
				stderr += data.toString();
			});
			const finish = (code) => {
				clearTimeout(timeout);
				options.signal?.removeEventListener("abort", killProcess);
				resolve({ stdout, stderr, code, killed });
			};
			proc.on("close", (code) => finish(code ?? 0));
			proc.on("error", () => finish(1));
		});
	}

	// ----- descriptions sent to the host -----------------------------------------------
	function describeTool(tool) {
		return {
			name: tool.name,
			label: tool.label ?? null,
			description: tool.description,
			promptSnippet: tool.promptSnippet,
			promptGuidelines: tool.promptGuidelines,
			parameters: plain(tool.parameters),
			outputSchema: plain(tool.outputSchema),
			constrainedSampling: tool.constrainedSampling,
			renderShell: tool.renderShell,
			exposure: tool.exposure,
			namespace: tool.namespace,
			annotations: tool.annotations,
			defaultActive: tool.defaultActive,
			executionMode: tool.executionMode,
			hasRenderCall: typeof tool.renderCall === "function",
			hasRenderResult: typeof tool.renderResult === "function",
		};
	}
	function describeProvider(config) {
		const out = {};
		for (const [key, value] of Object.entries(config)) {
			if (typeof value === "function") continue;
			if (key === "oauth" && value) out.oauth = { name: value.name, isSubscription: value.isSubscription };
			else out[key] = plain(value);
		}
		out.streamApis = typeof config.streamSimple === "function" && config.api ? [config.api] : [];
		out.hasRefreshModels = typeof config.refreshModels === "function";
		out.hasModifyModels = typeof config.oauth?.modifyModels === "function";
		return out;
	}
	/** Whether `implementation` is a pi-ai `Provider` rather than a provider configuration. */
	const isProvider = (implementation) => typeof implementation?.getModels === "function";
	/** A pi-ai `Provider`'s models of every type; none when listing throws, as in pi. */
	function providerModels(provider) {
		try {
			return provider.getAllModels?.() ?? provider.getModels();
		} catch {
			return [];
		}
	}
	/** pi-ai's `defaultProviderAuthContext`. */
	const authContext = {
		env: async (name) => (process.env[name]?.trim() ? process.env[name] : undefined),
		fileExists: async (path) => {
			const { fs, os } = globalThis.__yapi_builtins;
			return fs.existsSync(path.startsWith("~") ? os.homedir() + path.slice(1) : path);
		},
	};
	/** pi-ai's `isModelType`. */
	const isType = (model, type) => (model.type ?? "chat") === type;
	/**
	 * A pi-ai `Provider` described as `describeProvider` describes a
	 * configuration: its models, the APIs its streams and operations serve,
	 * its API key method, and whether its API key check or model filters
	 * decide what is available. The host asks `availability` for that.
	 */
	function describeNative(provider) {
		const models = providerModels(provider);
		const apis = (type) => [...new Set(models.filter((model) => isType(model, type)).map((model) => model.api))];
		const byApi = (implemented, type) => (implemented ? Object.fromEntries(apis(type).map((api) => [api, true])) : undefined);
		const { apiKey, oauth } = provider.auth ?? {};
		return {
			name: provider.name,
			baseUrl: provider.baseUrl,
			// pi-ai's `null` removes a header, which a configuration cannot.
			headers: provider.headers && Object.fromEntries(Object.entries(provider.headers).filter(([, value]) => value != null)),
			models: plain(models),
			oauth: oauth && { name: oauth.name, isSubscription: oauth.isSubscription },
			keyAuth: apiKey && { name: apiKey.name, login: typeof apiKey.login === "function" },
			// shortcut: an API only refreshed models use streams through yapi's API of that name; describe the APIs again after a refresh once providers add some.
			streamApis: apis("chat"),
			images: byApi(typeof provider.generateImages === "function", "image"),
			classifiers: byApi(typeof provider.classify === "function", "classifier"),
			hasRefreshModels: typeof provider.refreshModels === "function",
			hasFilter: typeof provider.filterModels === "function" || typeof provider.filterAllModels === "function",
		};
	}
	/** The configuration of provider `name` as last registered, with its code. */
	function providerConfig(name) {
		let found;
		for (const extension of extensions.values()) {
			for (const provider of extension.providers) if (provider.name === name && provider.implementation) found = provider.implementation;
		}
		return found;
	}
	/** What `extension` draws in the transcript. */
	function describeRenderers(extension) {
		return {
			messageRenderers: [...extension.messageRenderers.keys()],
			entryRenderers: [...extension.entryRenderers.keys()],
			markdownTransformer: typeof extension.markdownTransformer === "function",
			codeRenderer: typeof extension.codeRenderer === "function",
		};
	}
	/** Reports renderers registered once the session started: pi looks them up as it draws. */
	function refreshRenderers(extension) {
		if (bound) yapi.request("renderers.refresh", { extension: extension.id, ...describeRenderers(extension) });
	}
	function describe(extension) {
		return {
			id: extension.id,
			path: extension.path,
			tools: [...extension.tools.values()].map(describeTool),
			commands: [...extension.commands.values()].map((command) => ({
				name: command.name,
				description: command.description ?? null,
				hasCompletions: typeof command.getArgumentCompletions === "function",
			})),
			flags: [...extension.flags.values()].map((flag) => ({
				name: flag.name,
				type: flag.type,
				default: flag.default ?? null,
				description: flag.description ?? null,
			})),
			shortcuts: [...extension.shortcuts.values()].map((shortcut) => ({ shortcut: shortcut.shortcut, description: shortcut.description ?? null })),
			events: [...extension.handlers.keys()].sort(),
			...describeRenderers(extension),
			providers: extension.providers,
			mcpServers: [...extension.mcpServers].map(([name, { config }]) => ({ name, config })),
		};
	}

	// ----- loading --------------------------------------------------------------------------------
	/** Runs the factory of file `path`, imported once its module loads, as pi caches it. */
	async function loadOne({ id, path }) {
		let factory = factories.get(path);
		if (!factory) {
			// QuickJS keeps a module that failed. pi imports the file again,
			// so a later import goes by a new name.
			const count = imports.get(path) ?? 0;
			imports.set(path, count + 1);
			let module;
			try {
				module = await import(count === 0 ? path : `${path}?retry=${count}`);
			} catch (error) {
				return { id, path, error: `Failed to load extension: ${errorMessage(error)}` };
			}
			factory = module?.default;
			if (typeof factory !== "function") return { id, path, error: `Extension does not export a valid factory function: ${path}` };
			factories.set(path, factory);
		}
		return instantiate(id, path, factory);
	}

	/** Runs `factory` with a fresh API: pi does this for every session. */
	async function instantiate(id, path, factory) {
		const extension = {
			factory,
			id,
			path,
			handlers: new Map(),
			tools: new Map(),
			commands: new Map(),
			flags: new Map(),
			shortcuts: new Map(),
			messageRenderers: new Map(),
			entryRenderers: new Map(),
			markdownTransformer: undefined,
			codeRenderer: undefined,
			providers: [],
			mcpServers: new Map(),
			virtualModels: [],
		};
		const load = createApi(extension);
		try {
			await factory(load.api);
			load.commit();
		} catch (error) {
			load.discard();
			return { id, path, error: `Failed to load extension: ${errorMessage(error)}` };
		}
		extensions.set(id, extension);
		for (const provider of extension.providers) if (provider.native) provider.config = describeNative(provider.implementation);
		return describe(extension);
	}

	// ----- theme ------------------------------------------------------------------------------------------
	const THINKING_TOKENS = { minimal: "thinkingMinimal", low: "thinkingLow", medium: "thinkingMedium", high: "thinkingHigh", xhigh: "thinkingXhigh", max: "thinkingMax" };
	// What `setTheme` sends for each theme: its name, appearance and colors.
	const themeColors = new WeakMap();
	/**
	 * pi's `Theme` over the escape sequences of each token. pi's constructor
	 * computes them from colors; yapi's hosts report them (`new Theme(spec)`).
	 * Without them text stays plain.
	 */
	class Theme {
		#fg = {};
		#bg = {};
		#dim = new Set();
		#styled = false;
		#mode = "truecolor";
		constructor(fgColors, bgColors, mode, options = {}) {
			if (bgColors === undefined) {
				this.load(fgColors);
				return;
			}
			const fg = {
				...fgColors,
				scrollbarTrack: fgColors.scrollbarTrack ?? fgColors.muted,
				scrollbarThumb: fgColors.scrollbarThumb ?? fgColors.text,
				thinkingMax: fgColors.thinkingMax ?? fgColors.thinkingXhigh,
				searchMatchText: fgColors.searchMatchText ?? fgColors.text,
			};
			const bg = { ...bgColors, searchMatchBg: bgColors.searchMatchBg ?? bgColors.selectedBg };
			// "" is the terminal's default color.
			const sequences = (colors, background) =>
				Object.fromEntries(
					Object.entries(colors).map(([token, value]) => [token, value === "" ? `\x1b[${background ? 49 : 39}m` : yapi.colorAnsi(value, mode, background)]),
				);
			this.load({ name: options.name, mode, fg: sequences(fg, false), bg: sequences(bg, true), dim: options.dim, appearance: options.appearance, colors: { ...fg, ...bg } });
			this.sourcePath = options.sourcePath;
			this.sourceInfo = options.sourceInfo;
		}
		/** Takes the host's `{ name, mode, fg, bg, dim, colors }`, or `null` for plain text. */
		load(spec) {
			this.#styled = !!spec;
			this.#fg = spec?.fg ?? {};
			this.#bg = spec?.bg ?? {};
			this.#dim = new Set(spec?.dim ?? []);
			this.#mode = spec?.mode ?? "truecolor";
			this.name = spec?.name;
			themeColors.set(this, spec?.colors && { name: spec.name, appearance: spec.appearance, colors: spec.colors });
		}
		#sequence(map, token) {
			const sequence = map[token];
			if (sequence === undefined) throw new Error(`Unknown theme color: ${token}`);
			return sequence;
		}
		fg(token, text) {
			if (!this.#styled) return text;
			const sequence = this.#sequence(this.#fg, token);
			return this.#dim.has(token) ? `${sequence}\x1b[2m${text}\x1b[22;39m` : `${sequence}${text}\x1b[39m`;
		}
		bg(token, text) {
			return this.#styled ? `${this.#sequence(this.#bg, token)}${text}\x1b[49m` : text;
		}
		#wrap(open, close, text) {
			return this.#styled ? `\x1b[${open}m${text}\x1b[${close}m` : text;
		}
		bold(text) {
			return this.#wrap(1, 22, text);
		}
		italic(text) {
			return this.#wrap(3, 23, text);
		}
		underline(text) {
			return this.#wrap(4, 24, text);
		}
		strikethrough(text) {
			return this.#wrap(9, 29, text);
		}
		inverse(text) {
			return this.#wrap(7, 27, text);
		}
		getFgAnsi(token) {
			if (!this.#styled) return "";
			const sequence = this.#sequence(this.#fg, token);
			return this.#dim.has(token) ? `${sequence}\x1b[2m` : sequence;
		}
		getBgAnsi(token) {
			return this.#styled ? this.#sequence(this.#bg, token) : "";
		}
		getColorMode() {
			return this.#mode;
		}
		getThinkingBorderColor(level) {
			const token = THINKING_TOKENS[level] ?? "thinkingOff";
			return (text) => this.fg(token, text);
		}
		getBashModeBorderColor() {
			return (text) => this.fg("bashMode", text);
		}
	}
	yapi.Theme = Theme;
	const theme = new Theme(null);
	globalThis.__yapi_theme = theme;

	// ----- components ---------------------------------------------------------------------------------------
	// Components live here, by handle; the host renders them through
	// `yapi.render` and delivers keys through `yapi.input`.
	const components = new Map();
	let nextHandle = 1;
	const widgetHandles = new Map();
	const slots = { footer: undefined, header: undefined };
	// Components of transcript items, by tool call or message.
	const transcriptViews = new Map();
	function mount(component) {
		const handle = nextHandle++;
		components.set(handle, component);
		return handle;
	}
	function unmount(handle) {
		const component = components.get(handle);
		components.delete(handle);
		try {
			component?.dispose?.();
		} catch (error) {
			console.error("Component dispose error:", error);
		}
	}
	const tui = {
		requestRender: () => {
			if (bound) yapi.request("ui.requestRender", {});
		},
		// pi-tui's `ProcessTerminal` size.
		terminal: {
			get columns() {
				return process.stdout.columns || Number(process.env.COLUMNS) || 80;
			},
			get rows() {
				return process.stdout.rows || Number(process.env.LINES) || 24;
			},
			write() {},
			setTitle: (title) => bound && yapi.request("ui.setTitle", { title }),
		},
		// pi-tui's focused component, which listeners check before taking keys
		// from the prompt: an `Editor` while the prompt editor has the keyboard,
		// another object while a selector, dialog or component has it, and
		// `null` without a terminal.
		get focusedComponent() {
			if (editorFocused === null) return null;
			if (!editorFocused) return otherFocus;
			// This runtime's editor in the built-in one's place, else a stand-in.
			return components.get(editorSlot.handle) ?? (editorStandIn ??= tuiModule && Object.create(tuiModule.Editor.prototype));
		},
		getFocusedComponent() {
			return this.focusedComponent;
		},
		setFocus() {},
		showOverlay() {
			return overlayHandle(undefined);
		},
		hideOverlay() {},
		start() {},
		stop() {},
	};
	let editorFocused = null;
	let editorStandIn;
	const otherFocus = {};
	/**
	 * The terminal's `{ columns, rows, editorFocused }`, or `null` without
	 * one: stdout is a TTY of that size, which emits `resize` when it
	 * changes, as in Node.
	 */
	function setTerminal(size) {
		editorFocused = size ? !!size.editorFocused : null;
		const stdout = process.stdout;
		const resized = !!size && stdout.isTTY && (stdout.columns !== size.columns || stdout.rows !== size.rows);
		stdout.isTTY = !!size;
		stdout.columns = size?.columns;
		stdout.rows = size?.rows;
		if (!resized) return;
		tui.requestRender();
		stdout.emit("resize");
	}
	function overlayHandle(handle) {
		let hidden = false;
		return {
			hide: () => handle !== undefined && yapi.request("ui.close", { handle }),
			setHidden: (value) => void (hidden = value),
			isHidden: () => hidden,
			focus() {},
			unfocus() {},
			isFocused: () => !hidden,
		};
	}
	let tuiModule;
	let facade;
	const keybindings = async () => {
		tuiModule ??= await import("@earendil-works/pi-tui");
		return tuiModule.getKeybindings();
	};
	const request = (kind, payload) => yapi.request(`ui.${kind}`, payload);
	// pi-tui's input listeners: `onTerminalInput` handlers, which see raw
	// input before the editor and may consume or transform it.
	const terminalListeners = new Set();
	// `addAutocompleteProvider` factories and the provider they compose over
	// the host's, as pi's `setupAutocompleteProvider` does.
	const completionWrappers = [];
	const hostCompletions = {
		getSuggestions: (lines, cursorLine, cursorCol, options) =>
			yapi.op("ui.suggestions", { lines, cursorLine, cursorCol, force: !!options?.force }).then((suggestions) => suggestions ?? null),
		applyCompletion: (lines, cursorLine, cursorCol, item, prefix) => request("applyCompletion", { lines, cursorLine, cursorCol, item: plain(item), prefix }),
	};
	let completions = hostCompletions;
	function addCompletions(factory) {
		completionWrappers.push(factory);
		let provider = hostCompletions;
		const triggers = [];
		for (const wrap of completionWrappers) {
			provider = wrap(provider);
			triggers.push(...(provider.triggerCharacters ?? []));
		}
		if (triggers.length > 0) provider.triggerCharacters = [...new Set(triggers)];
		completions = provider;
		components.get(editorSlot.handle)?.setAutocompleteProvider?.(provider);
		request("setAutocomplete", { triggerCharacters: provider.triggerCharacters ?? [] });
	}
	// The extension editor in the built-in one's place, and the app actions
	// it asks the host for; pi's `setCustomEditorComponent`. The modules it
	// needs load when the session binds, so the factory runs at once.
	const editorSlot = { factory: undefined, handle: undefined };
	let editorActions = [];
	function setEditor(factory) {
		const text = request("getEditorText", {}) ?? "";
		const editor = typeof factory === "function" ? factory(tui, facade.getEditorTheme(), tuiModule.getKeybindings()) : undefined;
		editorSlot.factory = editor ? factory : undefined;
		if (editorSlot.handle !== undefined) unmount(editorSlot.handle);
		editorSlot.handle = undefined;
		if (!editor) {
			request("setEditor", {});
			return;
		}
		editor.onSubmit = (value) => request("editorSubmit", { text: value });
		editor.onChange = () => request("editorChange", { text: editor.getExpandedText?.() ?? editor.getText() });
		editor.setText(text);
		editor.setAutocompleteProvider?.(completions);
		// An editor extending `CustomEditor` triggers the app's actions, as the built-in editor does.
		if (editor.actionHandlers instanceof Map) {
			const action = (id) => () => request("editorAction", { action: id });
			editor.onEscape ??= action("app.interrupt");
			editor.onCtrlD ??= action("app.exit");
			editor.onPasteImage ??= action("app.clipboard.pasteImage");
			editor.onExtensionShortcut ??= (data) => !!request("editorShortcut", { data });
			for (const id of editorActions) editor.actionHandlers.set(id, action(id));
		}
		editorSlot.handle = mount(editor);
		request("setEditor", { handle: editorSlot.handle, embedsStatus: editor.embedWorkingStatus === true });
	}
	yapi.render = (handle, width) => {
		const component = components.get(handle);
		if (!component) return [];
		try {
			return component.render(width).map(String);
		} catch (error) {
			return [`Render error: ${errorMessage(error)}`];
		}
	};
	yapi.input = (handle, data) => {
		try {
			components.get(handle)?.handleInput?.(data);
		} catch (error) {
			console.error("Component input error:", error);
		}
	};

	// ----- contexts --------------------------------------------------------------------------------------
	let nextDialog = 1;
	function createUi(data) {
		const shown = !!(data.hasUI && data.components);
		const replaceSlot = (slot, factory, ...args) => {
			if (slots[slot] !== undefined) unmount(slots[slot]);
			slots[slot] = undefined;
			if (typeof factory === "function" && shown) slots[slot] = mount(factory(tui, theme, ...args));
			if (shown) request(slot === "footer" ? "setFooter" : "setHeader", { handle: slots[slot] });
		};
		const footerData = {
			getGitBranch: () => request("footerData", {})?.gitBranch ?? null,
			getExtensionStatuses: () => new Map(request("footerData", {})?.statuses ?? []),
			getAvailableProviderCount: () => request("footerData", {})?.providers ?? 0,
			onBranchChange: () => () => {},
		};
		// pi's dialog options: `timeout` and `signal` close the dialog with its
		// default answer, and an aborted `signal` opens none.
		const dialog = (kind, payload, opts, fallback) => {
			const signal = opts?.signal;
			if (!data.hasUI || signal?.aborted) return Promise.resolve(fallback);
			const id = signal ? nextDialog++ : undefined;
			const dismiss = () => request("cancelDialog", { id });
			signal?.addEventListener("abort", dismiss, { once: true });
			return yapi.op(`ui.${kind}`, { ...payload, timeout: opts?.timeout, dialog: id }).finally(() => signal?.removeEventListener("abort", dismiss));
		};
		return {
			// A cancelled dialog resolves to `undefined`, as in pi.
			select: (title, options, opts) => dialog("select", { title, options }, opts, undefined).then(orUndefined),
			confirm: (title, message, opts) => dialog("confirm", { title, message }, opts, false),
			input: (title, placeholder, opts) => dialog("input", { title, placeholder }, opts, undefined).then(orUndefined),
			editor: (title, prefill) => (data.hasUI ? yapi.op("ui.editor", { title, prefill }).then(orUndefined) : Promise.resolve(undefined)),
			notify: (message, type) => request("notify", type === undefined ? { message } : { message, type }),
			onTerminalInput(handler) {
				if (!shown) return () => {};
				if (terminalListeners.size === 0) request("setTerminalInput", { listening: true });
				terminalListeners.add(handler);
				return () => {
					if (terminalListeners.delete(handler) && terminalListeners.size === 0) request("setTerminalInput", { listening: false });
				};
			},
			setStatus: (key, text) => request("setStatus", { key, text }),
			setWorkingMessage: (message) => request("setWorkingMessage", { message }),
			setWorkingVisible: (visible) => request("setWorkingVisible", { visible }),
			setWorkingIndicator: (options) => request("setWorkingIndicator", { options: plain(options) }),
			setHiddenThinkingLabel: (label) => request("setHiddenThinkingLabel", { label }),
			setWidget(key, content, options) {
				const previous = widgetHandles.get(key);
				widgetHandles.delete(key);
				if (typeof content === "function") {
					// Modes without components show no component widgets.
					if (!shown) return;
					const handle = mount(content(tui, theme));
					widgetHandles.set(key, handle);
					request("setWidget", { key, handle, options: plain(options) });
				} else {
					request("setWidget", { key, lines: Array.isArray(content) ? content.map(String) : undefined, options: plain(options) });
				}
				if (previous !== undefined) unmount(previous);
			},
			setFooter: (factory) => replaceSlot("footer", factory, footerData),
			setHeader: (factory) => replaceSlot("header", factory),
			setTitle: (title) => request("setTitle", { title }),
			custom(factory, options) {
				if (!shown) return Promise.resolve(undefined);
				return new Promise((resolve, reject) => {
					let handle;
					let finished = false;
					const done = (result) => {
						if (finished) return;
						finished = true;
						if (handle !== undefined) {
							request("close", { handle });
							unmount(handle);
						}
						resolve(result);
					};
					keybindings()
						.then((manager) => factory(tui, theme, manager, done))
						.then((component) => {
							if (finished) {
								component?.dispose?.();
								return;
							}
							handle = mount(component);
							// pi-tui focuses the component it shows.
							if (component && "focused" in component) component.focused = true;
							// Without options an overlay takes the component's `width`, as in pi.
							const overlayOptions = options?.overlayOptions
								? typeof options.overlayOptions === "function"
									? options.overlayOptions()
									: options.overlayOptions
								: component?.width
									? { width: component.width }
									: undefined;
							request("custom", { handle, overlay: !!options?.overlay, overlayOptions: plain(overlayOptions) });
							options?.onHandle?.(overlayHandle(handle));
						})
						.catch(reject);
				});
			},
			pasteToEditor: (text) => request("pasteToEditor", { text }),
			setEditorText: (text) => request("setEditorText", { text }),
			getEditorText: () => request("getEditorText", {}) ?? "",
			addAutocompleteProvider(factory) {
				if (shown) addCompletions(factory);
			},
			setEditorComponent(factory) {
				if (shown) setEditor(factory);
			},
			getEditorComponent: () => (shown ? editorSlot.factory : undefined),
			theme,
			getAllThemes: () => request("getAllThemes", {}) ?? [],
			getTheme(name) {
				const spec = request("getTheme", { name });
				return spec ? new Theme(spec) : undefined;
			},
			// A `Theme` object goes to the host as its colors, never its escape sequences.
			setTheme: (theme) => request("setTheme", { theme: typeof theme === "string" ? theme : (themeColors.get(theme) ?? null) }),
			getToolsExpanded: () => !!request("getToolsExpanded", {}),
			setToolsExpanded: (expanded) => request("setToolsExpanded", { expanded }),
		};
	}

	const sessionMethods = [
		"getCwd",
		"getSessionDir",
		"getSessionId",
		"getSessionFile",
		"getLeafId",
		"getLeafEntry",
		"getEntry",
		"getLabel",
		"getBranch",
		"getHeader",
		"getEntries",
		"getTree",
		"getSessionName",
		"getContext",
		"buildSessionContext",
		"isPersisted",
		"getChildren",
	];
	// pi's writes, for `newSession`'s `setup` only. The host makes appendMessage,
	// appendCustomEntry and appendSessionInfo and names the others in its error.
	const sessionWrites = [
		"setSessionFile",
		"newSession",
		"appendMessage",
		"appendThinkingLevelChange",
		"appendModelChange",
		"appendUsage",
		"appendCompaction",
		"appendCustomEntry",
		"appendSessionInfo",
		"appendCustomMessageEntry",
		"appendContextEdit",
		"appendLabelChange",
		"branch",
		"resetLeaf",
		"branchWithSummary",
		"createBranchedSession",
	];
	function createSessionManager(writable = false) {
		const manager = {};
		for (const method of sessionMethods) {
			manager[method] = (...args) => {
				const value = yapi.request("session.read", { method, args: plain(args) });
				// pi answers a missing value with undefined, except a missing leaf or header.
				return value === null && method !== "getLeafId" && method !== "getHeader" ? undefined : value;
			};
		}
		if (writable) {
			for (const method of sessionWrites) {
				manager[method] = (...args) => yapi.request("session.write", { method, args: plain(args) });
			}
		}
		return manager;
	}
	function createModelRegistry() {
		return {
			find: (provider, id) => yapi.request("models.find", { provider, id }) ?? undefined,
			getAll: () => yapi.request("models.all", {}),
			getAvailable: () => yapi.request("models.available", {}),
			getApiKey: (model) => yapi.op("models.apiKey", { provider: model.provider, id: model.id }),
			getApiKeyForProvider: (provider) => yapi.op("models.apiKey", { provider }),
			getProviderAuth: async (provider) => (await yapi.op("models.auth", { provider })) ?? undefined,
			// pi reports a failure in the result and never rejects.
			getApiKeyAndHeaders: (model) =>
				yapi
					.op("models.requestAuth", { provider: model.provider, id: model.id })
					.catch((error) => ({ ok: false, error: error.message })),
			isUsingOAuth: (model) => !!yapi.request("models.usingOAuth", { provider: model.provider }),
			hasConfiguredAuth: (model) => !!yapi.request("models.hasAuth", { provider: model.provider }),
			getError: () => yapi.request("models.error", {}) ?? undefined,
			getProviderDisplayName: (provider) => yapi.request("models.providerName", { provider }) ?? provider,
			getModelsOfType: (type, provider) => yapi.request("models.ofType", { type, provider }) ?? [],
			getModelOfType: (type, provider, id) => ofType(type, provider, id),
			findOfType: (type, provider, id) => ofType(type, provider, id),
			getAvailableOfType: (type, provider) => yapi.op("models.availableOfType", { type, provider }),
			// Only the provider and id cross: the host runs its own catalog entry with its credentials.
			classify: (model, context, options) =>
				yapi.op("models.classify", {
					provider: model?.provider,
					id: model?.id,
					context,
					temperature: options?.temperature,
				}),
			generateImages: (model, context) =>
				yapi.op("models.generateImages", { provider: model?.provider, id: model?.id, context }),
			refresh: async (options) => {
				const result = await yapi.op("models.refresh", {
					providers: options?.providers,
					allowNetwork: options?.allowNetwork,
					force: options?.force,
				});
				const errors = new Map(Object.entries(result?.errors ?? {}).map(([id, message]) => [id, new Error(message)]));
				return { aborted: !!result?.aborted, errors };
			},
		};
	}
	function ofType(type, provider, id) {
		return (yapi.request("models.ofType", { type, provider }) ?? []).find((model) => model.id === id);
	}
	// The current run's abort signal, by the host's id, as pi's `ctx.signal`.
	let runAbort;
	function runSignal() {
		let run = null;
		try {
			run = yapi.request("agent.signal", {});
		} catch {
			// Hosts without a session have no run.
		}
		if (!run) return undefined;
		if (runAbort?.id !== run.id) {
			if (runAbort) aborts.delete(runAbort.id);
			runAbort = { id: run.id, controller: new AbortController() };
			aborts.set(run.id, runAbort.controller);
		}
		if (run.aborted) runAbort.controller.abort();
		return runAbort.controller.signal;
	}
	/** What the session holds now for the `key` of the host's context data, else `data`'s. */
	function current(data, key) {
		if (bound) {
			try {
				return yapi.request("session.context", {})?.[key] ?? undefined;
			} catch {
				// Hosts without a session answer from the call's data.
			}
		}
		return data[key] ?? undefined;
	}
	function createContext(data = {}, extra = {}) {
		// pi reads the model, thinking level, scoped models and run signal as they are read.
		const context = {
			ui: createUi(data),
			mode: data.mode ?? "print",
			hasUI: !!data.hasUI,
			cwd: data.cwd ?? yapi.cwd,
			sessionManager: createSessionManager(),
			modelRegistry: createModelRegistry(),
			get model() {
				return current(data, "model");
			},
			get scopedModels() {
				return current(data, "scopedModels") ?? [];
			},
			get thinkingLevel() {
				return current(data, "thinkingLevel");
			},
			get signal() {
				return runSignal();
			},
			isIdle: () => !!yapi.request("agent.isIdle", {}),
			isProjectTrusted: () => !!data.projectTrusted,
			abort: () => yapi.request("agent.abort", {}),
			hasPendingMessages: () => !!yapi.request("agent.hasPendingMessages", {}),
			shutdown: () => yapi.request("agent.shutdown", {}),
			getContextUsage: () => yapi.request("agent.contextUsage", {}) ?? undefined,
			compact: (options) => {
				yapi.op("agent.compact", { customInstructions: options?.customInstructions }).then(
					(result) => options?.onComplete?.(result),
					(error) => options?.onError?.(error),
				);
			},
			getSystemPrompt: () => yapi.request("agent.systemPrompt", {}) ?? "",
			...extra,
		};
		// Every member throws once the session is replaced.
		const born = generation;
		const guarded = {};
		for (const [key, descriptor] of Object.entries(Object.getOwnPropertyDescriptors(context))) {
			const read = descriptor.get ?? (() => descriptor.value);
			Object.defineProperty(guarded, key, {
				enumerable: true,
				get() {
					assertActive(born);
					return read();
				},
			});
		}
		return guarded;
	}
	// `newSession`'s `setup`s, by id, until the host runs them against the replacement.
	const setups = new Map();
	let nextSetup = 1;
	function createCommandContext(data, extra = {}) {
		// pi's session replacements: the host runs `newSession`'s `setup`
		// before the replacement starts, and `withSession` runs once it has,
		// with a context bound to it.
		const replace = async (kind, payload, options) => {
			const setup = typeof options?.setup === "function" ? { run: options.setup } : undefined;
			const id = setup ? nextSetup++ : undefined;
			if (setup) setups.set(id, setup);
			const result = await yapi.op(kind, { ...payload, setup: id }).finally(() => setups.delete(id));
			// pi's `newSession` rejects with what `setup` threw.
			await setup?.done;
			if (!result?.cancelled) {
				if (typeof options?.withSession === "function") await options.withSession(createReplacedContext());
				await yapi.op("session.replaced", {});
			}
			return { cancelled: !!result?.cancelled };
		};
		return createContext(data, {
			getSystemPromptOptions: () => yapi.request("agent.systemPromptOptions", {}) ?? {},
			waitForIdle: () => yapi.op("agent.waitForIdle", {}),
			newSession: (options) => replace("session.new", { parentSession: options?.parentSession }, options),
			fork: (entryId, options) => replace("session.fork", { entryId, position: options?.position }, options),
			navigateTree: async (targetId, options) => {
				const result = await yapi.op("session.navigateTree", { targetId, ...plain(options) });
				return { cancelled: !!result?.cancelled };
			},
			switchSession: (sessionPath, options) => replace("session.switch", { sessionPath }, options),
			reload: async () => {
				await yapi.op("session.reload", {});
				await yapi.op("session.replaced", {});
			},
			...extra,
		});
	}
	/** pi's `ReplacedSessionContext`: a command context for the session now current. */
	function createReplacedContext() {
		return createCommandContext(yapi.request("session.context", {}) ?? {}, {
			// These settle as the run they start ends.
			sendMessage: async (message, options) => {
				await yapi.op("session.sendMessage", messageRequest(message, options));
			},
			sendUserMessage: async (content, options) => {
				await yapi.op("session.sendUserMessage", userMessageRequest(content, options));
			},
		});
	}

	// ----- bash operations -----------------------------------------------------------------------
	// Operations `user_bash` handlers returned, by id, until the host runs them.
	const bashOperations = new Map();
	let nextBashOperations = 1;
	const bashAborts = new Map();
	/** pi's `isUserBashEventResult`. */
	function isUserBashEventResult(value) {
		if (typeof value !== "object" || value === null) return false;
		const hasOperations = value.operations !== undefined;
		const hasResult = value.result !== undefined;
		if (hasOperations === hasResult) return false;
		if (hasOperations) {
			return typeof value.operations === "object" && value.operations !== null && typeof value.operations.exec === "function";
		}
		const result = value.result;
		if (typeof result !== "object" || result === null) return false;
		return (
			typeof result.output === "string" &&
			"exitCode" in result &&
			(result.exitCode === undefined || typeof result.exitCode === "number") &&
			typeof result.cancelled === "boolean" &&
			typeof result.truncated === "boolean" &&
			(result.fullOutputPath === undefined || typeof result.fullOutputPath === "string")
		);
	}

	// ----- emitting to one extension -----------------------------------------------------------
	/** Runs `extension`'s handlers for `event` as pi's runner does for each handler. */
	async function emit(extension, event, ctx, signal) {
		const handlers = [...(extension.handlers.get(event.type) ?? [])];
		// What JSON cannot carry: the abort signal and pi's file operation sets.
		if (event.type === "session_before_compact" || event.type === "session_before_tree") event.signal = signal;
		const fileOps = event.type === "session_before_compact" ? event.preparation?.fileOps : undefined;
		if (fileOps) for (const key of ["read", "written", "edited"]) fileOps[key] = new Set(fileOps[key] ?? []);
		const errors = [];
		const guard = async (run) => {
			try {
				return await run();
			} catch (error) {
				errors.push(errorInfo(error));
				return undefined;
			}
		};
		let result;
		switch (event.type) {
			case "tool_call": {
				// Errors propagate: a failing guard blocks the call. Handlers
				// change the arguments by editing `event.input` in place.
				for (const handler of handlers) {
					const handlerResult = await handler(event, ctx);
					if (handlerResult) {
						result = handlerResult;
						if (result.block) break;
					}
				}
				result = { ...result, input: event.input };
				break;
			}
			case "tool_result": {
				// pi's event holds details and usage even when undefined, so
				// values handlers give them take those places.
				const { details, structuredContent, isError, usage, ...head } = event;
				const current = { ...head, details, ...(structuredContent === undefined ? {} : { structuredContent }), isError, usage };
				let modified = false;
				for (const handler of handlers) {
					const handlerResult = await guard(() => handler(current, ctx));
					if (!handlerResult) continue;
					for (const key of ["content", "details", "structuredContent", "isError", "usage"]) {
						if (handlerResult[key] !== undefined) {
							current[key] = handlerResult[key];
							modified = true;
						}
					}
					if (handlerResult.content !== undefined && handlerResult.structuredContent === undefined) delete current.structuredContent;
				}
				if (modified) result = { content: current.content, details: current.details, structuredContent: current.structuredContent, isError: current.isError, usage: current.usage };
				break;
			}
			case "message_end": {
				let message = event.message;
				let modified = false;
				for (const handler of handlers) {
					const handlerResult = await guard(() => handler({ ...event, message }, ctx));
					if (!handlerResult?.message) continue;
					if (handlerResult.message.role !== message.role) {
						errors.push({ error: "message_end handlers must return a message with the same role" });
						continue;
					}
					message = handlerResult.message;
					modified = true;
				}
				if (modified) result = { message };
				break;
			}
			case "input": {
				let text = event.text;
				let images = event.images;
				for (const handler of handlers) {
					const handlerResult = await guard(() => handler({ ...event, text, images }, ctx));
					if (handlerResult?.action === "handled") {
						result = handlerResult;
						break;
					}
					if (handlerResult?.action === "transform") {
						text = handlerResult.text;
						images = handlerResult.images ?? images;
					}
				}
				result ??= text !== event.text || images !== event.images ? { action: "transform", text, images } : { action: "continue" };
				break;
			}
			case "before_agent_start": {
				// As in pi, handlers share the live prompt options and may edit them; a
				// returned `systemPrompt` forces the prompt through `forceSystemPrompt`.
				// The prompt, and `ctx.getSystemPrompt()`, render the options as they are.
				const messages = [];
				const options = event.systemPromptOptions;
				const render = () => options.forceSystemPrompt ?? yapi.request("agent.renderSystemPrompt", { options: plain(options) });
				Object.defineProperty(event, "systemPrompt", { enumerable: true, get: render });
				const local = Object.create(ctx, { getSystemPrompt: { value: render } });
				for (const handler of handlers) {
					const handlerResult = await guard(() => handler(event, local));
					if (handlerResult?.message) messages.push(handlerResult.message);
					if (handlerResult?.systemPrompt !== undefined) options.forceSystemPrompt = handlerResult.systemPrompt;
				}
				result = { messages, systemPrompt: options.forceSystemPrompt, systemPromptOptions: plain(options) };
				break;
			}
			case "context":
			case "context_with_system": {
				// Handlers return a new list or edit `event.messages` in place.
				let messages = event.messages;
				let changed = false;
				for (const handler of handlers) {
					const visible = messages;
					const snapshot = visible.slice();
					const handlerResult = await guard(() => handler({ ...event, messages: visible }, ctx));
					const edited = visible.length !== snapshot.length || visible.some((message, index) => message !== snapshot[index]);
					const returned = handlerResult?.messages ?? (edited ? visible : undefined);
					if (!returned) continue;
					messages = returned;
					changed = true;
				}
				if (changed) result = { messages };
				break;
			}
			case "before_provider_headers": {
				// Handlers change `event.headers` in place.
				for (const handler of handlers) await guard(() => handler(event, ctx));
				result = { headers: event.headers };
				break;
			}
			case "turn_end":
			case "agent_before_settle": {
				// pi's `emitBoundary` for this extension's handlers: each sees the
				// entries and continuation the earlier ones staged, and the context
				// they would make, built when read.
				let entries = event.entries;
				let shouldContinue = event.continue;
				for (const handler of handlers) {
					const staged = entries;
					const boundaryEvent = { ...event, entries: staged, continue: shouldContinue };
					Object.defineProperty(boundaryEvent, "context", {
						enumerable: true,
						get: () => yapi.request("session.boundaryContext", { type: event.type, entries: plain(staged) }),
					});
					const handlerResult = await guard(() => handler(boundaryEvent, ctx));
					if (handlerResult?.entries !== undefined) entries = handlerResult.entries;
					if (handlerResult?.continue !== undefined) shouldContinue = handlerResult.continue;
				}
				result = { entries, continue: shouldContinue };
				break;
			}
			case "before_provider_request": {
				let payload = event.payload;
				for (const handler of handlers) {
					const handlerResult = await guard(() => handler({ ...event, payload }, ctx));
					if (handlerResult !== undefined) payload = handlerResult;
				}
				result = { payload };
				break;
			}
			case "user_bash": {
				// Errors propagate: a failing handler stops the command.
				for (const handler of handlers) {
					const handlerResult = await handler(event, ctx);
					if (handlerResult === undefined) continue;
					if (!isUserBashEventResult(handlerResult)) {
						throw new Error(
							"Invalid user_bash handler result: return undefined for local execution or exactly one valid { operations } or { result } object",
						);
					}
					if (handlerResult.operations) {
						const id = nextBashOperations++;
						bashOperations.set(id, handlerResult.operations);
						result = { operations: { id } };
					} else {
						result = { result: handlerResult.result };
					}
					break;
				}
				break;
			}
			case "resources_discover": {
				const collected = { skillPaths: [], promptPaths: [], themePaths: [] };
				for (const handler of handlers) {
					const handlerResult = await guard(() => handler(event, ctx));
					for (const key of Object.keys(collected)) if (handlerResult?.[key]?.length) collected[key].push(...handlerResult[key]);
				}
				result = collected;
				break;
			}
			case "cache_warming_decision": {
				// Only an action a handler returned overrides pi's, so the host can
				// let the last extension that decided win.
				for (const handler of handlers) {
					const handlerResult = await guard(() => handler(event, ctx));
					if (handlerResult?.action !== undefined) result = { action: handlerResult.action };
				}
				break;
			}
			default: {
				const cancellable = event.type.startsWith("session_before_");
				for (const handler of handlers) {
					const handlerResult = await guard(() => handler(event, ctx));
					if (cancellable && handlerResult) {
						result = handlerResult;
						if (result.cancel) break;
					}
				}
			}
		}
		return { result: plain(result), errors };
	}

	// ----- provider streams --------------------------------------------------------------------
	/** Aborts of running streams and sign-ins, by the host's id. */
	const aborts = new Map();
	/**
	 * An event for the host: `start` with its message, `done` and `error` with
	 * the final one, the others without the live message but with its usage.
	 */
	function wireEvent(event) {
		switch (event.type) {
			case "start":
				return { type: "start", message: plain(event.partial) };
			case "done":
				return { type: "done", message: plain(event.message) };
			case "error":
				return { type: "error", message: plain(event.error) };
			default: {
				const { partial, ...rest } = event;
				if (event.type === "toolcall_start") {
					const block = partial?.content?.[event.contentIndex];
					rest.id = block?.id;
					rest.toolName = block?.name;
				}
				return { type: "update", event: plain(rest), usage: plain(partial?.usage) };
			}
		}
	}
	/**
	 * pi's `onPayload`, `onResponse` and `onProviderStreamEvent` options for
	 * stream `id`, those of the session's hooks that `hooks` names. pi-ai's
	 * facade passes them on to yapi's providers by `yapiStream`.
	 */
	function requestHooks(id, hooks = {}) {
		const hook = (kind, toPayload) => Object.assign(async (value) => yapi.op(`provider.${kind}`, { id, ...toPayload(value) }), { yapiStream: id });
		const options = {};
		if (hooks.payload) options.onPayload = hook("payload", (body) => ({ payload: plain(body) ?? null }));
		if (hooks.response) {
			options.onResponse = hook("response", (response) => ({
				status: response?.status,
				headers: response?.headers instanceof Headers ? Object.fromEntries(response.headers) : (plain(response?.headers) ?? {}),
			}));
		}
		if (hooks.streamEvent) options.onProviderStreamEvent = hook("streamEvent", (data) => ({ data: plain(data) ?? null }));
		return options;
	}
	/** Runs `run` with an abort signal that the host's `abort` of `id` fires. */
	async function abortable(id, run) {
		const controller = new AbortController();
		aborts.set(id, controller);
		try {
			return await run(controller.signal);
		} finally {
			aborts.delete(id);
		}
	}
	/**
	 * Runs `run` with provider `payload.provider`'s `oauth`, and whether it is
	 * pi-ai's `OAuthAuth` rather than the legacy form, as operation `payload.id`.
	 */
	function withOAuth(payload, run) {
		const config = providerConfig(payload.provider);
		const oauth = isProvider(config) ? config.auth?.oauth : config?.oauth;
		if (!oauth) throw new Error(`Provider ${payload.provider} has no OAuth sign-in`);
		return abortable(payload.id, async (signal) => plain(await run(oauth, signal, isProvider(config))) ?? null);
	}
	/** pi-ai's `AuthInteraction` whose prompts and events reach the host's sign-in `payload.id`. */
	const signInInteraction = (payload) => ({
		notify: (event) => yapi.request("oauth.notify", { id: payload.id, event }),
		prompt: (question) => yapi.op("oauth.prompt", { id: payload.id, prompt: question }),
	});
	/** Classifies or generates images with `payload.model` through its provider's code, as pi's provider composer picks it. */
	function operate(payload, kind, method) {
		const { model, options } = payload;
		const config = providerConfig(model.provider);
		const implementation = isProvider(config) ? config : config?.[kind === "image" ? "images" : "classifiers"]?.[model.api];
		if (typeof implementation?.[method] !== "function") throw new Error(`Provider ${model.provider} has no ${kind} implementation for "${model.api}"`);
		return abortable(payload.id, async (signal) =>
			plain(
				await implementation[method](model, payload.context, {
					apiKey: orUndefined(options.apiKey),
					headers: options.headers,
					temperature: orUndefined(options.temperature),
					signal,
				}),
			),
		);
	}
	/** pi's `lazyStream` message for a stream that failed; pi-ai's facade uses it too. */
	const setupError = (yapi.setupError = (model, error) => ({
		role: "assistant",
		content: [],
		api: model.api,
		provider: model.provider,
		model: model.id,
		usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } },
		stopReason: "error",
		errorMessage: errorMessage(error),
		timestamp: Date.now(),
	}));

	// ----- dispatch -------------------------------------------------------------------------------
	const extensionOf = (id) => {
		const extension = extensions.get(id);
		if (!extension) throw new Error(`Unknown extension ${id}`);
		return extension;
	};
	const kinds = {
		async load(payload) {
			yapi.cwd = payload.cwd;
			// Code that runs as extensions load reads the terminal's size. Who
			// has the keyboard is known once the session binds.
			const size = payload.terminalSize;
			if (size) Object.assign(process.stdout, { isTTY: true, columns: size.columns, rows: size.rows });
			for (const [name, value] of Object.entries(payload.flags ?? {})) flagValues.set(name, value);
			const results = [];
			for (const entry of payload.extensions) results.push(await loadOne(entry));
			described = true;
			// Wire APIs registered with pi-ai's `registerApiProvider`.
			return { extensions: results, apis: [...(yapi.apiProviders?.keys() ?? [])] };
		},
		async bind() {
			bound = true;
			for (const change of unboundChanges.splice(0)) {
				try {
					change();
				} catch (error) {
					console.error(error);
				}
			}
			// The servers registered while loading. After a restart, the
			// session drops the servers registered since.
			try {
				reportMcpServers();
			} catch {
				// Hosts without a session connect no servers.
			}
			let spec = null;
			let keys = null;
			let size = null;
			try {
				spec = yapi.request("ui.theme", {}) ?? null;
				keys = yapi.request("ui.keybindings", {}) ?? null;
				size = yapi.request("ui.terminalSize", {}) ?? null;
			} catch {
				// Hosts without a UI leave text plain.
			}
			theme.load(spec);
			setTerminal(size);
			// Components match keys as the host's bindings do.
			if (keys) {
				tuiModule ??= await import("@earendil-works/pi-tui");
				facade ??= await import("@earendil-works/pi-coding-agent");
				tuiModule.setKittyProtocolActive(!!keys.kitty);
				const definitions = Object.fromEntries(Object.entries(keys.bindings ?? {}).map(([id, defaultKeys]) => [id, { defaultKeys }]));
				tuiModule.setKeybindings(new tuiModule.KeybindingsManager(definitions));
				editorActions = keys.actions ?? [];
			}
			return null;
		},
		/** Runs `newSession`'s `setup` `{ id }` with the replacement's writable session manager. */
		async setup(payload) {
			const setup = setups.get(payload.id);
			setup.done = (async () => setup.run(createSessionManager(true)))();
			// `newSession` rethrows it.
			await setup.done.catch(() => {});
			return null;
		},
		/** Runs the extensions of `{ extensions }`, entries as `load` takes them, for the next session. */
		async reload(payload) {
			bound = false;
			described = false;
			unboundChanges.length = 0;
			generation.stale = true;
			generation = { stale: false };
			// pi drops the stale extensions' `pi.events` listeners.
			eventBus.clear();
			for (const handle of [...components.keys()]) unmount(handle);
			widgetHandles.clear();
			transcriptViews.clear();
			slots.footer = slots.header = undefined;
			editorSlot.factory = editorSlot.handle = undefined;
			terminalListeners.clear();
			completionWrappers.length = 0;
			completions = hostCompletions;
			// As in pi's fresh runtime, the stale extensions' registrations,
			// such as MCP server names, do not count while the factories run,
			// and an extension that failed in an earlier session loads again:
			// new files and files that failed to import are imported.
			extensions.clear();
			const results = [];
			for (const entry of payload.extensions) results.push(await loadOne(entry));
			described = true;
			return { extensions: results, apis: [...(yapi.apiProviders?.keys() ?? [])] };
		},
		flags(payload) {
			for (const [name, value] of Object.entries(payload.values ?? {})) flagValues.set(name, value);
			return null;
		},
		/** `{ channel, data }` emitted with `pi.events` in another runtime. */
		events(payload) {
			eventBus.emit(payload.channel, payload.data);
			return null;
		},
		/** The terminal changed to `{ columns, rows, editorFocused }`. */
		terminal(payload) {
			setTerminal(payload);
			return null;
		},
		emit(payload) {
			const signal = payload.ctx?.aborted ? AbortSignal.abort() : new AbortController().signal;
			return emit(extensionOf(payload.extension), payload.event, createContext(payload.ctx), signal);
		},
		async tool(payload) {
			const extension = extensionOf(payload.extension);
			const tool = extension.tools.get(payload.name);
			if (!tool) throw new Error(`Tool ${payload.name} is not registered by ${extension.path}`);
			return abortable(payload.id, async (signal) => {
				const ctx = createContext(payload.ctx, {
					signal: payload.ctx?.aborted ? AbortSignal.abort() : signal,
					tools: payload.tools ?? [],
					executeTool: (name, args) => yapi.op("tool.execute", { toolCallId: payload.toolCallId, name, args: plain(args) }),
				});
				const onUpdate = (partial) => yapi.request("tool.update", { toolCallId: payload.toolCallId, partial: plain(partial) });
				let params = payload.params;
				if (typeof tool.prepareArguments === "function") params = tool.prepareArguments(params);
				const result = await tool.execute(payload.toolCallId, params, ctx.signal, onUpdate, ctx);
				return plain(result) ?? { content: [] };
			});
		},
		async command(payload) {
			const command = extensionOf(payload.extension).commands.get(payload.name);
			if (!command) throw new Error(`Command /${payload.name} is not registered`);
			// pi hands commands its live base prompt options, so the host
			// keeps what the command edited in them, unless the session was
			// replaced.
			const born = generation;
			let fetched, options;
			const getSystemPromptOptions = () => {
				const current = yapi.request("agent.systemPromptOptions", {}) ?? {};
				const json = JSON.stringify(current);
				if (json !== fetched) [fetched, options] = [json, current];
				return options;
			};
			try {
				await command.handler(payload.args ?? "", createCommandContext(payload.ctx, { getSystemPromptOptions }));
			} finally {
				if (options && !born.stale && JSON.stringify(options) !== fetched) {
					yapi.request("agent.setSystemPromptOptions", { fetched: JSON.parse(fetched), options });
				}
			}
			return null;
		},
		async complete(payload) {
			const command = extensionOf(payload.extension).commands.get(payload.name);
			if (typeof command?.getArgumentCompletions !== "function") return null;
			return plain(await command.getArgumentCompletions(payload.prefix ?? "")) ?? null;
		},
		/**
		 * The composed providers' suggestions for the host's editor, with what
		 * applying each item gives, since the editor applies one at once.
		 */
		async autocomplete(payload) {
			const { lines, cursorLine, cursorCol, force } = payload;
			const provider = completions;
			const suggestions = await provider.getSuggestions(lines, cursorLine, cursorCol, { signal: new AbortController().signal, force });
			if (!Array.isArray(suggestions?.items) || suggestions.items.length === 0) return null;
			const applied = suggestions.items.map((item) => {
				try {
					return plain(provider.applyCompletion(lines, cursorLine, cursorCol, item, suggestions.prefix)) ?? null;
				} catch (error) {
					console.error("Autocomplete error:", error);
					return null;
				}
			});
			return { prefix: suggestions.prefix ?? "", items: plain(suggestions.items), applied };
		},
		/** Runs the input listeners over each key as pi-tui does; `null` for a consumed key. */
		terminalInput(payload) {
			return (payload.keys ?? []).map((data) => {
				let current = data;
				for (const listener of terminalListeners) {
					let result;
					try {
						result = listener(current);
					} catch (error) {
						console.error("Terminal input listener error:", error);
						continue;
					}
					if (result?.consume) return null;
					if (result?.data !== undefined) current = result.data;
				}
				return current;
			});
		},
		/** An operation the host sends its editor; see `ComponentHost::editor_op`. */
		editor(payload) {
			const editor = components.get(payload.handle);
			if (payload.op === "setText") editor?.setText?.(payload.text ?? "");
			else if (payload.op === "addToHistory") editor?.addToHistory?.(payload.text ?? "");
			else if (payload.op === "insertTextAtCursor" && editor) {
				// pi's `handleClipboardPaste` sets the text apart at the editor's own cursor.
				const cursor = payload.apart ? editor.getCursor?.() : undefined;
				const line = cursor ? (editor.getText().split("\n")[cursor.line] ?? "") : "";
				const space = (char) => (char && !/\s/.test(char) ? " " : "");
				const before = cursor && cursor.col > 0 ? line[cursor.col - 1] : "";
				editor.insertTextAtCursor?.(`${space(before)}${payload.text ?? ""}${space(cursor ? line[cursor.col] : "")}`);
			} else if (payload.op === "configure" && editor) {
				editor.borderColor = payload.border === "bashMode" ? theme.getBashModeBorderColor() : theme.getThinkingBorderColor(payload.border);
				editor.setPaddingX?.(payload.paddingX);
				editor.setAutocompleteMaxVisible?.(payload.autocompleteMaxVisible);
				if ("focused" in editor) editor.focused = !!payload.focused;
			}
			return null;
		},
		/** A mouse event for a component; whether it took it, as pi-tui's `dispatchMouseEvent` decides. */
		mouse(payload) {
			let result;
			try {
				result = components.get(payload.handle)?.handleMouse?.(payload.event);
			} catch (error) {
				console.error("Component mouse error:", error);
			}
			return !!result && ("target" in result || !!(result.handled || result.capture || result.focus));
		},
		/** Runs a `!` command through the operations with `id`; `{ exitCode }`. */
		async bash(payload) {
			const operations = bashOperations.get(payload.id);
			bashOperations.delete(payload.id);
			if (!operations) throw new Error(`No bash operations ${payload.id}`);
			const controller = new AbortController();
			bashAborts.set(payload.id, controller);
			try {
				const onData = (data) => yapi.request("bash.data", { id: payload.id, data: yapi.base64Encode(Buffer.from(data)) });
				const result = await operations.exec(payload.command, payload.cwd, { onData, signal: controller.signal });
				return { exitCode: result?.exitCode ?? null };
			} finally {
				bashAborts.delete(payload.id);
			}
		},
		bashAbort(payload) {
			bashAborts.get(payload.id)?.abort();
			return null;
		},
		async shortcut(payload) {
			const shortcut = extensionOf(payload.extension).shortcuts.get(payload.shortcut);
			if (!shortcut) throw new Error(`Shortcut ${payload.shortcut} is not registered`);
			await shortcut.handler(createContext(payload.ctx));
			return null;
		},
		/**
		 * Streams a response for a model whose API an extension implements:
		 * its provider's `streamSimple` when the API matches, or is one its
		 * pi-ai `Provider`'s models use, else the API's `registerApiProvider`
		 * stream, as pi's provider composer picks. Each event goes to the
		 * host as it arrives. A stream that throws ends with pi's setup error.
		 */
		async stream(payload) {
			const { id, model, context } = payload;
			const send = (event) => yapi.request("provider.event", { id, ...event });
			await abortable(id, async (signal) => {
				try {
					const config = providerConfig(model.provider);
					const options = { ...payload.options, signal, ...requestHooks(id, payload.hooks) };
					let events;
					const own = isProvider(config)
						? config.getModels().some((entry) => entry.api === model.api)
						: typeof config?.streamSimple === "function" && config.api === model.api;
					if (own) events = config.streamSimple(model, context, options);
					else {
						const api = yapi.apiProviders?.get(model.api)?.provider;
						if (!api) throw new Error(`No API provider registered for api: ${model.api}`);
						events = api.streamSimple(model, context, options);
					}
					for await (const event of events) {
						send(wireEvent(event));
						if (event.type === "done" || event.type === "error") break;
					}
				} catch (error) {
					send({ type: "error", message: setupError(model, error) });
				}
			});
			return null;
		},
		/** Aborts stream or sign-in `id`. */
		abort(payload) {
			aborts.get(payload.id)?.abort();
			return null;
		},
		/**
		 * Runs provider `provider`'s `oauth.login` with an interaction, or
		 * legacy callbacks as pi's provider composer adapts them, that reach
		 * the host's sign-in `id`.
		 */
		oauthLogin(payload) {
			const { notify, prompt } = signInInteraction(payload);
			return withOAuth(payload, (oauth, signal, canonical) =>
				canonical
					? oauth.login({ prompt, notify, signal })
					: oauth.login({
							onAuth: (info) => notify({ type: "auth_url", ...info }),
							onDeviceCode: (info) => notify({ type: "device_code", ...info }),
							onPrompt: (question) => prompt({ type: "text", ...question }),
							onProgress: (message) => notify({ type: "progress", message }),
							onManualCodeInput: () => prompt({ type: "manual_code", message: "Paste the authorization code" }),
							onSelect: (question) => prompt({ type: "select", ...question }),
							signal,
						}),
			);
		},
		oauthRefresh(payload) {
			const credential = { type: "oauth", ...payload.credential };
			return withOAuth(payload, (oauth, signal, canonical) => (canonical ? oauth.refresh(credential, signal) : oauth.refreshToken(credential, signal)));
		},
		/** The key a legacy `getApiKey` derives, or the credentials of pi-ai's `toAuth`. */
		oauthApiKey(payload) {
			const credential = { type: "oauth", ...payload.credential };
			return withOAuth(payload, (oauth, _signal, canonical) => (canonical ? oauth.toAuth(credential) : oauth.getApiKey(credential)));
		},
		/** Provider `provider`'s legacy `oauth.modifyModels` of `models` for `credential`. */
		modifyModels(payload) {
			const oauth = providerConfig(payload.provider)?.oauth;
			if (typeof oauth?.modifyModels !== "function") throw new Error(`Provider ${payload.provider} has no modifyModels`);
			return plain(oauth.modifyModels(payload.models, { type: "oauth", ...payload.credential }));
		},
		/** Runs provider `provider`'s pi-ai `auth.apiKey.login` with an interaction that reaches the host's sign-in `id`. */
		keyLogin(payload) {
			const apiKey = providerConfig(payload.provider)?.auth?.apiKey;
			if (typeof apiKey?.login !== "function") throw new Error(`Provider ${payload.provider} has no API key login`);
			return abortable(payload.id, async (signal) => plain(await apiKey.login({ ...signInInteraction(payload), signal })) ?? null);
		},
		/**
		 * What provider `provider`, a pi-ai `Provider`, makes available for
		 * the stored `credential`, as pi's `Models` checks it: where its API
		 * key `check`, else its `resolve`, finds a key (`null` for none), and
		 * the ids of the chat models `filterModels` keeps and of the models
		 * of each type `getAllAvailable` keeps (`null` to keep every one).
		 */
		async availability(payload) {
			const provider = providerConfig(payload.provider);
			const credential = payload.credential ?? undefined;
			const apiKey = provider.auth?.apiKey;
			let source = null;
			// pi checks no key while an OAuth credential is stored.
			if (apiKey && credential?.type !== "oauth") {
				const input = { ctx: authContext, credential, signal: new AbortController().signal };
				const found = await (apiKey.check ? apiKey.check(input) : apiKey.resolve(input));
				source = found ? (found.source ?? "configured provider") : null;
			}
			const models = providerModels(provider);
			const chat = provider.filterModels?.(models.filter((model) => isType(model, "chat")), credential).map((model) => model.id) ?? null;
			let kept = null;
			if (provider.filterAllModels) kept = provider.filterAllModels(models, credential);
			else if (chat) kept = models.filter((model) => !isType(model, "chat") || chat.includes(model.id));
			let all = null;
			if (kept) {
				all = {};
				for (const model of kept) (all[model.type ?? "chat"] ??= []).push(model.id);
			}
			return { source, chat, all };
		},
		/** Provider `provider`'s pi-ai `auth.apiKey.resolve` with the stored `credential`: its result, or `null`. */
		async keyResolve(payload) {
			const apiKey = providerConfig(payload.provider)?.auth?.apiKey;
			if (!apiKey) throw new Error(`Provider ${payload.provider} has no API key auth`);
			const credential = payload.credential ? { type: "api_key", ...payload.credential } : undefined;
			return plain(await apiKey.resolve({ ctx: authContext, credential, signal: new AbortController().signal })) ?? null;
		},
		classify(payload) {
			return operate(payload, "classifier", "classify");
		},
		generateImages(payload) {
			return operate(payload, "image", "generateImages");
		},
		/**
		 * Runs one phase of provider `provider`'s `refreshModels`: `{ models }`,
		 * `null` when it returns none, with the last entry it asked `publish`
		 * to persist (`null` to delete) as `persist`. The host writes it. A
		 * pi-ai `Provider` returns nothing and lists its models after.
		 */
		async refreshModels(payload) {
			const config = providerConfig(payload.provider);
			if (typeof config?.refreshModels !== "function") return { models: null };
			const result = {};
			const models = await abortable(payload.id, (signal) =>
				config.refreshModels({
					credential: payload.credential ?? undefined,
					stored: payload.stored ?? undefined,
					allowNetwork: !!payload.allowNetwork,
					force: payload.allowNetwork ? payload.force : undefined,
					signal,
					publish: async (publication) => {
						if (publication.persist !== undefined) result.persist = plain(publication.persist) ?? null;
						publication.update?.();
						return true;
					},
				}),
			);
			result.models = plain(isProvider(config) ? providerModels(config) : models) ?? null;
			return result;
		},
		/**
		 * What extension `payload.extension`'s Markdown transformer makes of
		 * `payload.markdown`, or `null` to keep it, as pi keeps the text when
		 * a transformer throws or returns no string.
		 */
		markdown(payload) {
			const transformer = extensionOf(payload.extension).markdownTransformer;
			try {
				const markdown = transformer?.(payload.markdown, plain(payload.context ?? {}));
				return typeof markdown === "string" ? markdown : null;
			} catch {
				return null;
			}
		},
		/**
		 * The lines extension `payload.extension`'s code renderer draws for
		 * `{ code, lang, width }`, or `null` to leave the code to yapi. An
		 * error is the renderer's.
		 */
		code(payload) {
			const render = extensionOf(payload.extension).codeRenderer;
			const lines = render?.(payload.code, payload.lang ?? undefined, { width: payload.width }, theme);
			return Array.isArray(lines) ? lines.map(String) : null;
		},
		/**
		 * Builds the component a tool's `renderCall`/`renderResult` or a
		 * message renderer returns; `{ handle }`, or `null` for the built-in
		 * rendering. A renderer that returns its last component keeps its
		 * handle.
		 */
		async component(payload) {
			const extension = extensionOf(payload.extension);
			const key = payload.kind === "message" || payload.kind === "entry" ? `${payload.kind}:${payload.key}` : `tool:${payload.toolCallId}`;
			const views = transcriptViews.get(key) ?? { state: {}, call: undefined, result: undefined, message: undefined };
			transcriptViews.set(key, views);
			const slot = payload.kind === "toolCall" ? "call" : payload.kind === "toolResult" ? "result" : "message";
			// pi's `CustomEntryComponent` shows a renderer's error in a box.
			if (payload.kind === "entry") tuiModule ??= await import("@earendil-works/pi-tui");
			const previous = views[slot];
			let component;
			try {
				if (payload.kind === "entry") {
					const renderer = extension.entryRenderers.get(payload.entry?.customType);
					try {
						component = renderer?.(payload.entry, plain(payload.options ?? {}), theme);
					} catch (error) {
						component = new tuiModule.Box(1, 1, (text) => theme.bg("customMessageBg", text));
						component.addChild(
							new tuiModule.Text(theme.fg("error", `[${payload.entry.customType}] renderer failed: ${errorMessage(error)}`), 0, 0),
						);
					}
				} else if (slot === "message") {
					const renderer = extension.messageRenderers.get(payload.message?.customType);
					component = renderer?.(payload.message, plain(payload.options ?? {}), theme);
				} else {
					const tool = extension.tools.get(payload.name);
					const context = {
						...(payload.context ?? {}),
						args: payload.args,
						toolCallId: payload.toolCallId,
						invalidate: () => tui.requestRender(),
						lastComponent: previous?.component,
						state: views.state,
					};
					component =
						slot === "call"
							? tool?.renderCall?.(payload.args, theme, context)
							: tool?.renderResult?.(payload.result, plain(payload.options ?? {}), theme, context);
				}
			} catch {
				// pi falls back to the default rendering without a word.
				component = undefined;
			}
			if (!component) {
				if (previous) unmount(previous.handle);
				views[slot] = undefined;
				return null;
			}
			if (previous?.component === component) return { handle: previous.handle };
			if (previous) unmount(previous.handle);
			const handle = mount(component);
			views[slot] = { handle, component };
			return { handle };
		},
	};

	yapi.dispatch = (id, kind, payloadText) => {
		const handler = kinds[kind];
		Promise.resolve()
			.then(() => {
				if (!handler) throw new Error(`Unknown dispatch kind: ${kind}`);
				return handler(JSON.parse(payloadText));
			})
			.then(
				(value) => native.done(id, JSON.stringify(value ?? null) ?? "null"),
				(error) => native.fail(id, errorMessage(error)),
			);
	};
	yapi.extensions = extensions;
})();
