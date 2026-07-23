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
    borrow::Cow,
    io::{self, Write},
    panic::{self, PanicHookInfo},
    sync::{
        Once,
        atomic::{AtomicBool, Ordering},
    },
};

use url::Url;

const CHATGPT_URL: &str = "https://chatgpt.com/";
const MAX_PANIC_MESSAGE_BYTES: usize = 2_000;
const TRUNCATION_MARKER: &str = "\n… [panic message truncated]";
static INSTALL: Once = Once::new();
static BROWSER_OPENING: AtomicBool = AtomicBool::new(false);

/// Installs the ChatGPT panic hook.
///
/// Installation is process-wide and idempotent. The hook first invokes the previously installed
/// panic hook, then asks the system's default browser to open ChatGPT with the panic context. A
/// browser launch failure is reported to standard error on a best-effort basis without replacing
/// the original panic output.
///
/// Browser launch runs synchronously inside the panic hook and may block when the default browser
/// is text-based. Panics that overlap an active browser launch preserve their normal panic output
/// without opening additional tabs.
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

            let Some(_launch) = BrowserLaunch::acquire() else {
                return;
            };
            let prompt = panic_prompt(info);
            let url = chatgpt_url(&prompt);
            if let Err(error) = webbrowser::open(url.as_str()) {
                report_browser_error(&error);
            }
        }));
    });
}

fn panic_prompt(info: &PanicHookInfo<'_>) -> String {
    let message = bounded_panic_message(panic_message(info.payload()));
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

fn bounded_panic_message(message: &str) -> Cow<'_, str> {
    if message.len() <= MAX_PANIC_MESSAGE_BYTES {
        return Cow::Borrowed(message);
    }

    let mut end = MAX_PANIC_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }

    let mut truncated = String::with_capacity(end + TRUNCATION_MARKER.len());
    truncated.push_str(&message[..end]);
    truncated.push_str(TRUNCATION_MARKER);
    Cow::Owned(truncated)
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

fn report_browser_error(error: &io::Error) {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(
        stderr,
        "chatgpt-panic-hook: could not open ChatGPT: {error}"
    );
}

struct BrowserLaunch;

impl BrowserLaunch {
    fn acquire() -> Option<Self> {
        BROWSER_OPENING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            .then_some(Self)
    }
}

impl Drop for BrowserLaunch {
    fn drop(&mut self) {
        BROWSER_OPENING.store(false, Ordering::Release);
    }
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

    #[test]
    fn oversized_panic_message_is_truncated_at_a_utf8_boundary() {
        let message = format!("{}🦀", "a".repeat(MAX_PANIC_MESSAGE_BYTES - 1));

        let bounded = bounded_panic_message(&message);

        assert!(bounded.ends_with(TRUNCATION_MARKER));
        assert!(!bounded.contains('🦀'));
    }

    #[test]
    fn overlapping_browser_launch_is_suppressed() {
        let first = BrowserLaunch::acquire().unwrap();

        assert!(BrowserLaunch::acquire().is_none());

        drop(first);
        assert!(BrowserLaunch::acquire().is_some());
    }
}
