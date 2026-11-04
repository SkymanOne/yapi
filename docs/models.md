# Choose a model

yapi supports Pi's providers and models, from the same built-in catalog. A model generates the responses, and a provider is the service or account that gives access to it. This page covers signing in, API keys, cloud and local providers, selecting a model, adding providers and handing credentials to other tools.

## Sign in with a subscription

Run `/login` and choose "Sign in with an account" to use a Claude Pro or Max, ChatGPT Plus or Pro, GitHub Copilot, Kimi For Coding, Meta, SuperGrok or X Premium account, or to create an OpenRouter key. yapi opens the browser sign-in, or shows a device code, and refreshes the token when it expires.

"Sign in with Radius" signs in to Pi's Radius gateway, whose models come from the gateway's own catalog. Providers that extensions register can add their own sign-ins. `/logout` removes a stored credential.

## Use an API key

yapi reads the same environment variables as Pi:

| Provider | Variable |
|---|---|
| Anthropic | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| Google Gemini | `GEMINI_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| Groq | `GROQ_API_KEY` |
| xAI | `XAI_API_KEY` |
| DeepSeek | `DEEPSEEK_API_KEY` |
| Mistral | `MISTRAL_API_KEY` |
| OpenCode Zen and Go | `OPENCODE_API_KEY` |

`yapi --help` lists every provider, including Azure OpenAI and Vercel AI Gateway. To save a key instead, run `/login` and choose "Sign in with an API key". yapi stores it in `~/.yapi/agent/auth.json`.

## Use a cloud provider

Amazon Bedrock, Google Vertex AI and Cloudflare use their platform's credentials:

- **Amazon Bedrock**: reads `AWS_BEARER_TOKEN_BEDROCK`, or Amazon Web Services (AWS) credentials from the environment, `~/.aws` profiles (including single sign-on and assumed roles), container credentials or instance metadata. It takes the region from `AWS_REGION` or the profile. As in the AWS SDK, `AWS_CONTAINER_CREDENTIALS_FULL_URI` must use HTTPS unless it names a loopback address or the Elastic Container Service (ECS) or Elastic Kubernetes Service (EKS) container host.
- **Google Vertex AI**: reads `GOOGLE_CLOUD_API_KEY`, or Application Default Credentials with `GOOGLE_CLOUD_PROJECT` and `GOOGLE_CLOUD_LOCATION`.
- **Cloudflare Workers AI and AI Gateway**: read `CLOUDFLARE_API_KEY` and `CLOUDFLARE_ACCOUNT_ID`, and `CLOUDFLARE_GATEWAY_ID` for the gateway.

`/login` can store these settings in `auth.json` instead. With `ANTHROPIC_FEDERATION_RULE_ID` and the related variables set, Anthropic requests use workload identity federation instead of a key.

## Run local models

yapi supports the [llama.cpp](https://github.com/ggml-org/llama.cpp) router server. Start `llama-server` in router mode, without `--model`:

```sh
llama-server --models-dir ~/models --no-models-autoload --jinja \
  --host 127.0.0.1 --port 8080
```

Run `/login llama.cpp` and enter the server URL and an optional API key, or set `LLAMA_BASE_URL` and `LLAMA_API_KEY`. `/llama` loads and unloads the server's models and downloads new ones from Hugging Face. Loaded models appear in `/model`. Pi's [llama.cpp guide](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/llama-cpp.md) covers the server options.

To remove the provider and `/llama`, disable `llama.cpp` in `yapi config`.

## Select a model

`--list-models` searches the catalog, and `--model` takes an exact model or a pattern, with an optional thinking level after `:`. `--models` sets the models that Ctrl+P cycles through:

```sh
yapi --list-models sonnet
yapi --model anthropic/claude-sonnet-4-5
yapi --model sonnet:high
yapi --models "sonnet,gpt-5*"
```

Inside yapi, `/model` or Ctrl+L opens the model selector, Ctrl+P cycles through models, and Shift+Tab cycles the thinking level.

## Add providers and models

Add providers, models and overrides to `~/.yapi/agent/models.json`. Pi's [models](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/models.md) and [custom provider](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/custom-provider.md) documentation describe the format. Extensions can also [register providers](extensions.md#providers-from-extensions), including ones with their own wire API.

## Update the model catalog

Pi publishes catalog updates between releases. yapi fetches them for the providers you've configured: at startup, when the model selector opens, after `/login` and with `yapi update --models`. It keeps them in `~/.yapi/agent/models-store.json` for offline use.

`--offline`, or setting `PI_OFFLINE`, turns off the automatic fetches. `yapi update --models` fetches regardless.

## Use classifier and image models

The catalog also lists classifiers, which answer typed questions about JSON data, and image models on OpenRouter. Codemode scripts run them through the `models` global, and extensions through `ctx.modelRegistry.classify()` and `generateImages()`. Codemode lets the model call Model Context Protocol (MCP) tools from a script, as [Call MCP tools through codemode](mcp.md#call-mcp-tools-through-codemode) describes.

## Give credentials to other tools

`yapi auth` gives scripts a credential without starting a session. `check` reports whether a provider is ready, and the `print-` commands print its API key or bearer token:

```sh
yapi auth check --provider anthropic --json
yapi auth print-api-key --provider openai
yapi auth print-bearer-token --provider openai-codex --min-expiry 30m
```

`print-bearer-token` refreshes a token that expires within `--min-expiry`, which is 30 minutes by default and at least 5 minutes. It fails when the refreshed token doesn't last that long.
