# yapi-extension-api

The Rust SDK for native extensions of [yapi](https://github.com/SkymanOne/yapi), a Rust coding agent that runs [Pi](https://pi.dev) packages. An extension built with it is a WebAssembly component that registers tools, commands, flags, event handlers and interface components, and runs in a sandboxed instance of its own.

```toml
[dependencies]
yapi-extension-api = "0.4.1"
```

```rust
use yapi_extension_api::{Api, notify};

fn init(api: &mut Api) {
    api.register_command("hello", "Says hello", |args, _ctx| async move {
        let name = if args.is_empty() { "world" } else { &args };
        notify(&format!("Hello, {name}!"), "info");
        Ok(())
    });
}

yapi_extension_api::extension!(init);
```

Run `yapi new <name>` to create a package that's ready to build, or set your own crate's type to `cdylib`. Build it for `wasm32-wasip2`, then load the result with `yapi -e <file>.wasm`. The [native extensions guide](https://skymanone.github.io/yapi/native-extensions.html) covers the API, and the [examples](https://github.com/SkymanOne/yapi/tree/main/guest/examples) show nineteen complete extensions.

The `widgets` feature adds yapi's ports of pi-tui's widgets, such as `SelectList` and `Editor`, from [`yapi-tui`](https://crates.io/crates/yapi-tui).

The API is unstable until yapi 1.0, and extensions may need to be rebuilt for a new yapi release. The crate needs Rust 1.88 or newer, or 1.91 with the `widgets` feature.

## License

MIT OR Apache-2.0, at your option.
