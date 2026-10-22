import {
  classifySystemOne,
  isRecord
} from "./chunk-ZUFRIJAQ.mjs";

// target/vendor-pi/pi-ai/dist/api/cloudflare-workers-ai-system-one.js
var LABEL = "Cloudflare Workers AI";
function cloudflareErrorMessage(errors) {
  if (Array.isArray(errors)) {
    const messages = errors.map((error) => isRecord(error) && typeof error.message === "string" ? error.message : void 0).filter((message) => message !== void 0);
    if (messages.length > 0)
      return `${LABEL} error: ${messages.join("; ")}`;
  }
  return `${LABEL} request failed`;
}
var transport = {
  api: "cloudflare-workers-ai-system-one",
  label: LABEL,
  url: (model) => new URL("run", `${model.baseUrl.replace(/\/+$/u, "")}/`),
  payload: (model, request) => ({ model: model.id, input: request }),
  output: (body) => {
    if (!isRecord(body))
      throw new Error(`${LABEL} returned an unexpected response`);
    if (body.success === false)
      throw new Error(cloudflareErrorMessage(body.errors));
    const run = body.result;
    if (!isRecord(run))
      throw new Error(`${LABEL} returned an unexpected response`);
    if (run.state !== "Completed") {
      throw new Error(`${LABEL} run did not complete (state: ${String(run.state)})`);
    }
    if (!isRecord(run.result))
      throw new Error(`${LABEL} returned an unexpected response`);
    return run.result;
  }
};
var classify = (model, context, options) => classifySystemOne(transport, model, context, options);

export {
  classify
};
