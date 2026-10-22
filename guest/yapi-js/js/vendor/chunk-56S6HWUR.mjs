const require = globalThis.__yapi_require_for('yapi:vendor');
import "./chunk-G7YUWZG4.mjs";

// target/vendor-pi/pi-ai/dist/api/openai-codex-responses.js
var host = () => globalThis.__yapi.hostApi("openai-codex-responses");
var stream = (model, context, options) => host().stream(model, context, options);
var streamSimple = (model, context, options) => host().streamSimple(model, context, options);
export {
  stream,
  streamSimple
};
