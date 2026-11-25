const __terminiBridgeEnv = (name) => (globalThis.Deno ? globalThis.Deno.env.get(name) : globalThis.process.env[name]);
const __terminiBridgeUrl = __terminiBridgeEnv("TERMINI_TOOL_BRIDGE_URL");
const __terminiBridgeToken = __terminiBridgeEnv("TERMINI_TOOL_BRIDGE_TOKEN");
const __terminiToolCall = async (name, args) => {
  const response = await fetch(__terminiBridgeUrl, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ token: __terminiBridgeToken, name, args: args ?? {} }),
  });
  const body = await response.json();
  if (body.code !== 200) {
    throw new Error(`tool ${name} failed: ${body.message ?? "unknown error"}`);
  }
  return body.data;
};
globalThis.tools = new Proxy({}, { get: (_target, name) => (args) => __terminiToolCall(String(name), args) });
