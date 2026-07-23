# chatgpt-panic-hook

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![CI][ci-badge]][ci-url]
[![MSRV][msrv-badge]][manifest-url]
[![License][license-badge]][license-url]

The last panic hook you will ever need.

When your Rust program panics, `chatgpt-panic-hook` preserves the normal panic output and opens
ChatGPT in your default browser with the panic message and source location:

```rust
fn main() {
    chatgpt_panic_hook::install();

    panic!("borrow checker escaped");
}
```

Add it to your project:

```console
cargo add chatgpt-panic-hook
```

That is the entire API. Calling `install` more than once has no additional effect.

Browser launch runs synchronously inside the panic hook and may block for text-based browsers.
Overlapping panics retain their normal panic output but do not open additional tabs. Panic messages
larger than 2,000 bytes are truncated before being added to the URL.

From a repository checkout, run the example:

```console
cargo run --example panic
```

The command intentionally panics and opens ChatGPT in the default browser.

## Privacy

This crate sends panic messages and source paths to `chatgpt.com` as a URL query parameter. Those
values can contain secrets, private user data, or details about your machine. Use it for local
experiments, not production.

## Origin

Inspired by the JavaScript error-handling strategy:

```javascript
try {
    // ...
} catch (error) {
    window.location.href = `https://chatgpt.com/?q=${error}`;
}
```

Rust does not have `try`/`catch`, but it does have panic hooks.

## License

MIT

[crates-badge]: https://img.shields.io/crates/v/chatgpt-panic-hook.svg
[crates-url]: https://crates.io/crates/chatgpt-panic-hook
[ci-badge]: https://github.com/joshka/chatgpt-panic-hook/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/joshka/chatgpt-panic-hook/actions/workflows/ci.yml
[docs-badge]: https://docs.rs/chatgpt-panic-hook/badge.svg
[docs-url]: https://docs.rs/chatgpt-panic-hook
[license-badge]: https://img.shields.io/crates/l/chatgpt-panic-hook.svg
[license-url]: LICENSE
[manifest-url]: Cargo.toml
[msrv-badge]: https://img.shields.io/crates/msrv/chatgpt-panic-hook
