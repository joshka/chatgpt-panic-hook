# chatgpt-panic-hook

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
