## Installation steps

`wasm-pack` is required to build the Rust code into WASM:
- https://rustwasm.github.io/wasm-pack/installer/

`deno` is required to execute the WASM code locally to test everything is working:
- https://docs.deno.com/runtime/getting_started/installation/

We need to compile the WASM with the following command run (from the workspace root):

```bash
wasm-pack build --target web ws-client
```

The `ws-client/pkg` directory should now exists and contains the compiled WASM.

## Execution

We first need to start the WS server in a terminal (from the workspace root):

```bash
cargo run -p ws-server
```

And we can test the connection from another terminal with `deno` and the `main.ts` script (from the workspace root):
```bash
deno run --allow-read --alow-net main.ts
```