import init, { wsPing } from "./ws-client/pkg/ws_client.js";

await init();

console.log(await wsPing("ws://localhost:8081/ws", "hello"));
