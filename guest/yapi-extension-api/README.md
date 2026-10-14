# yapi-extension-api

The Rust SDK for native [yapi](https://github.com/SkymanOne/yapi) extensions. An extension built with it is a WebAssembly component that registers tools, commands, flags, event handlers and interface components, and runs in a sandboxed instance of its own.

```toml
[dependencies]
yapi-extension-api = "0.2.0"
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

Build the crate as a `cdylib` for `wasm32-wasip2` and load the result with `yapi -e <file>.wasm`. `yapi new <name>` creates a package that is ready to build. The [native extensions guide](https://skymanone.github.io/yapi/native-extensions.html) covers the API, and the [examples](https://github.com/SkymanOne/yapi/tree/main/guest/examples) show seventeen complete extensions.

The `widgets` feature adds yapi's ports of pi-tui's widgets, such as `SelectList` and `Editor`, from [`yapi-tui`](https://crates.io/crates/yapi-tui).

The API is unstable until yapi 1.0, and extensions may need to be rebuilt for a new yapi release. The crate needs Rust 1.88 or newer, or 1.91 with the `widgets` feature.

## License

MIT OR Apache-2.0, at your option.
