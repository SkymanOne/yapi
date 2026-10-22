const require = globalThis.__yapi_require_for('yapi:vendor');

// target/vendor-pi/pi-ai/dist/api/pi-messages.pi.js
var PiMessagesResponseError = class extends Error {
  code;
  diagnosticDetails;
  constructor(message, code, diagnosticDetails) {
    super(message);
    this.name = "PiMessagesResponseError";
    this.code = code;
    this.diagnosticDetails = diagnosticDetails;
  }
};

// target/vendor-pi/pi-ai/dist/api/pi-messages.js
var host = () => globalThis.__yapi.hostApi("pi-messages");
var stream = (model, context, options) => host().stream(model, context, options);
var streamSimple = (model, context, options) => host().streamSimple(model, context, options);

export {
  PiMessagesResponseError,
  stream,
  streamSimple
};
