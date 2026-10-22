// yapi: the bundle keeps the helpers of pi-ai's OpenAI modules, whose requests
// run on the host, so it needs no OpenAI client.
export default class OpenAI {
    constructor() {
        throw new Error("The OpenAI SDK is not available in yapi extensions");
    }
}
