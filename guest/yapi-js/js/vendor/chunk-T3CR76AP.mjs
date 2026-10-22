const require = globalThis.__yapi_require_for('yapi:vendor');
import {
  classifySystemOne,
  isRecord
} from "./chunk-VUSE5LAN.mjs";

// target/vendor-pi/pi-ai/dist/api/typesafe-system-one.js
var transport = {
  api: "typesafe-system-one",
  label: "System One API",
  url: (model) => new URL("systemone", `${model.baseUrl.replace(/\/+$/u, "")}/`),
  payload: (model, request) => ({ model: model.id, ...request }),
  output: (body) => {
    if (!isRecord(body))
      throw new Error("System One API returned an unexpected response");
    return body;
  }
};
var classify = (model, context, options) => classifySystemOne(transport, model, context, options);

export {
  classify
};
