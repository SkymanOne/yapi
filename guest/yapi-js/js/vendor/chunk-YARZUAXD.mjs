// target/vendor-pi/pi-ai/dist/providers/data/github-copilot.json
var github_copilot_default = {};

// target/vendor-pi/pi-ai/dist/model-catalog.js
function catalog(type, provider) {
  let models;
  const load = () => models ??= Object.fromEntries(globalThis.__yapi.request("models.list", { provider, type }).map((model) => [model.id, model]));
  return new Proxy({}, {
    get: (_target, key) => load()[key],
    has: (_target, key) => key in load(),
    ownKeys: () => Reflect.ownKeys(load()),
    getOwnPropertyDescriptor: (_target, key) => {
      const descriptor = Reflect.getOwnPropertyDescriptor(load(), key);
      if (descriptor)
        descriptor.configurable = true;
      return descriptor;
    }
  });
}
function flattenChatModelCatalog(provider) {
  return catalog("chat", provider);
}
function flattenImageModelCatalog(provider) {
  return catalog("image", provider);
}
function flattenClassifierModelCatalog(provider) {
  return catalog("classifier", provider);
}

// target/vendor-pi/pi-ai/dist/providers/github-copilot.models.js
var GITHUB_COPILOT_MODELS = flattenChatModelCatalog("github-copilot", github_copilot_default);
var GITHUB_COPILOT_IMAGE_MODELS = flattenImageModelCatalog("github-copilot", github_copilot_default);
var GITHUB_COPILOT_CLASSIFIER_MODELS = flattenClassifierModelCatalog("github-copilot", github_copilot_default);

export {
  flattenChatModelCatalog,
  flattenImageModelCatalog,
  flattenClassifierModelCatalog,
  GITHUB_COPILOT_MODELS,
  GITHUB_COPILOT_IMAGE_MODELS,
  GITHUB_COPILOT_CLASSIFIER_MODELS
};
