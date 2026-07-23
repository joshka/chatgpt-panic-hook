//! A panic hook that asks ChatGPT what went wrong.
//!
//! Call [`install`] near the start of `main`. When a thread panics, the crate preserves the
//! previously installed panic hook and opens a new browser tab containing the panic message and
//! source location.
//!
//! ```no_run
//! chatgpt_panic_hook::install();
//! panic!("whoops");
//! ```
//!
//! # Privacy
//!
//! Panic messages and source paths can contain sensitive information. Installing this hook sends
//! that text to `chatgpt.com` as a URL query parameter. Do not install it in programs that may
//! handle secrets or private user data.

use std::{
    any::Any,
    panic::{self, PanicHookInfo},
    sync::Once,
};

use url::Url;

const CHATGPT_URL: &str = "https://chatgpt.com/";
static INSTALL: Once = Once::new();

/// Installs the ChatGPT panic hook.
///
/// Installation is process-wide and idempotent. The hook first invokes the previously installed
/// panic hook, then asks the system's default browser to open ChatGPT with the panic context. A
/// browser launch failure is reported to standard error without replacing the original panic
/// output.
///
/// # Panics
///
/// Panics if called from a panicking thread, matching [`std::panic::set_hook`].
///
/// # Privacy
///
/// The panic message and source location are sent to `chatgpt.com` in the URL. They may contain
/// sensitive information.
pub fn install() {
    INSTALL.call_once(|| {
        let previous_hook = panic::take_hook();

        panic::set_hook(Box::new(move |info| {
            previous_hook(info);

            let prompt = panic_prompt(info);
            let url = chatgpt_url(&prompt);
            if let Err(error) = webbrowser::open(url.as_str()) {
                eprintln!("chatgpt-panic-hook: could not open ChatGPT: {error}");
            }
        }));
    });
}

fn panic_prompt(info: &PanicHookInfo<'_>) -> String {
    let message = panic_message(info.payload());
    let location = info
        .location()
        .map(|location| {
            format!(
                "{}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            )
        })
        .unwrap_or_else(|| "unknown".to_owned());
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("<unnamed>");

    format!(
        "A Rust thread named `{thread_name}` panicked.\n\n\
         Panic message: {message}\n\
         Source location: {location}\n\n\
         Explain the likely cause and suggest a fix."
    )
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<non-string panic payload>")
}

fn chatgpt_url(prompt: &str) -> Url {
    let mut url = Url::parse(CHATGPT_URL).expect("CHATGPT_URL must be a valid URL");
    url.query_pairs_mut().append_pair("q", prompt);
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_round_trips_punctuation_and_newlines() {
        let prompt = "panic: expected `&str`?\nTry again.";
        let url = chatgpt_url(prompt);

        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("chatgpt.com"));
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "q").unwrap().1,
            prompt
        );
    }

    #[test]
    fn string_panic_payload_is_preserved() {
        let payload = String::from("the crab escaped");

        assert_eq!(panic_message(&payload), "the crab escaped");
    }

    #[test]
    fn borrowed_string_panic_payload_is_preserved() {
        let payload = "the crab escaped";

        assert_eq!(panic_message(&payload), "the crab escaped");
    }

    #[test]
    fn non_string_panic_payload_has_a_safe_fallback() {
        let payload = 42_u32;

        assert_eq!(panic_message(&payload), "<non-string panic payload>");
    }
}
