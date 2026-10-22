const require = globalThis.__yapi_require_for('yapi:vendor');
import "./chunk-G7YUWZG4.mjs";

// target/vendor-pi/pi-ai/dist/api/google-vertex.js
var host = () => globalThis.__yapi.hostApi("google-vertex");
var stream = (model, context, options) => host().stream(model, context, options);
var streamSimple = (model, context, options) => host().streamSimple(model, context, options);
export {
  stream,
  streamSimple
};
