# Choose a model

yapi supports Pi's providers and models, from the same built-in catalog. A model generates the responses, and a provider is the service or account that gives access to it.

## Sign in with a subscription

Run `/login` and choose "Sign in with an account" to use a Claude Pro or Max, ChatGPT Plus or Pro, GitHub Copilot, Kimi For Coding, Meta, SuperGrok or X Premium account, or to create an OpenRouter key. yapi opens the browser sign-in, or shows a device code, and refreshes the token when it expires.

"Sign in with Radius" signs in to Pi's Radius gateway, whose models come from the gateway's own catalog. Providers registered by extensions can add their own sign-ins. `/logout` removes a stored credential.

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

- Amazon Bedrock reads `AWS_BEARER_TOKEN_BEDROCK`, or AWS credentials from the environment, `~/.aws` profiles (including SSO and assumed roles), container credentials or instance metadata, with `AWS_REGION` or the profile's region.
- Google Vertex AI reads `GOOGLE_CLOUD_API_KEY`, or Application Default Credentials with `GOOGLE_CLOUD_PROJECT` and `GOOGLE_CLOUD_LOCATION`.
- Cloudflare Workers AI and AI Gateway read `CLOUDFLARE_API_KEY` and `CLOUDFLARE_ACCOUNT_ID`, and `CLOUDFLARE_GATEWAY_ID` for the gateway.

`/login` can store these settings in `auth.json` instead. With `ANTHROPIC_FEDERATION_RULE_ID` and the related variables set, Anthropic requests use workload identity federation instead of a key.

## Run local models

yapi supports the [llama.cpp](https://github.com/ggml-org/llama.cpp) router server. Start `llama-server` without `--model`:

```sh
llama-server --models-dir ~/models --no-models-autoload --jinja --host 127.0.0.1 --port 8080
```

Run `/login llama.cpp` and enter the server URL and an optional API key, or set `LLAMA_BASE_URL` and `LLAMA_API_KEY`. `/llama` loads and unloads the server's models and downloads new ones from Hugging Face. Loaded models appear in `/model`. Pi's [llama.cpp guide](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/llama-cpp.md) covers the server options.

To remove the provider and `/llama`, disable `llama.cpp` in `yapi config`.

## Select a model

```sh
yapi --list-models sonnet            # search the catalog
yapi --model anthropic/claude-sonnet-4-5
yapi --model sonnet:high             # a pattern with a thinking level
yapi --models "sonnet,gpt-5*"        # the models Ctrl+P cycles through
```

Inside yapi, `/model` or Ctrl+L opens the model selector, Ctrl+P cycles through models, and Shift+Tab cycles the thinking level.

## Add providers and models

Add providers, models and overrides to `~/.yapi/agent/models.json`. The format is described in Pi's [models](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/models.md) and [custom provider](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/custom-provider.md) documentation. Extensions can also [register providers](extensions.md#providers-from-extensions), including ones with their own wire API.

## Catalog updates

Pi publishes catalog updates between releases. yapi fetches them for the providers you have configured: at startup, when the model selector opens, after `/login` and with `yapi update --models`. It keeps them in `~/.yapi/agent/models-store.json` for offline use. `--offline` or `PI_OFFLINE=1` turns fetching off.

## Classifier and image models

The catalog also lists classifiers, which answer typed questions about JSON data, and image models on OpenRouter. Codemode scripts run them through the `models` global, and extensions through `ctx.modelRegistry.classify()` and `generateImages()`. See [Codemode](mcp.md#codemode).

## Credentials for other tools

`yapi auth` gives scripts a credential without starting a session:

```sh
yapi auth check --provider anthropic --json
yapi auth print-api-key --provider openai
yapi auth print-bearer-token --provider openai-codex --min-expiry 30m
```
