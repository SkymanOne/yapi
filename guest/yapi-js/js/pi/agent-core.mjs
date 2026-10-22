// `@earendil-works/pi-agent-core` for extensions in yapi: pi's own agent and
// agent loop, which run inside the extension over the stream function and
// tools it passes. As in pi, where the coding agent installs pi-ai's
// `streamSimple` as the default, a loop without one streams through the facade.
import { streamSimple } from "@earendil-works/pi-ai";
import { setDefaultStreamFn } from "yapi:vendor/pi-agent-core";

export * from "yapi:vendor/pi-agent-core";

setDefaultStreamFn(streamSimple);
