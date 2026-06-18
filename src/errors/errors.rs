//! Typed error types for the clicker library.

use std::error::Error;
use std::fmt;
use std::time::Duration;

/// Returned when a connection to the browser fails.
#[derive(Debug)]
pub struct ConnectionError {
    pub url: String,
    pub cause: Option<Box<dyn Error + Send + Sync>>,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "failed to connect to {}: {}", self.url, cause),
            None => write!(f, "failed to connect to {}", self.url),
        }
    }
}

impl Error for ConnectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_deref().map(|c| c as &(dyn Error + 'static))
    }
}

/// Returned when a wait operation times out.
#[derive(Debug)]
pub struct TimeoutError {
    pub selector: String,
    pub timeout: Duration,
    pub reason: String,
}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = format_go_duration(self.timeout);
        if !self.reason.is_empty() {
            write!(
                f,
                "timeout after {} waiting for '{}': {}",
                d, self.selector, self.reason
            )
        } else {
            write!(f, "timeout after {} waiting for '{}'", d, self.selector)
        }
    }
}

impl Error for TimeoutError {}

/// Returned when a selector matches no elements.
#[derive(Debug)]
pub struct ElementNotFoundError {
    pub selector: String,
    /// browsing context ID
    pub context: String,
}

impl fmt::Display for ElementNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.context.is_empty() {
            write!(
                f,
                "element not found: {} (context: {})",
                self.selector, self.context
            )
        } else {
            write!(f, "element not found: {}", self.selector)
        }
    }
}

impl Error for ElementNotFoundError {}

/// Returned when the browser process dies unexpectedly.
#[derive(Debug)]
pub struct BrowserCrashedError {
    pub exit_code: i32,
    pub output: String,
}

impl fmt::Display for BrowserCrashedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.output.is_empty() {
            write!(
                f,
                "browser crashed with exit code {}: {}",
                self.exit_code, self.output
            )
        } else {
            write!(f, "browser crashed with exit code {}", self.exit_code)
        }
    }
}

impl Error for BrowserCrashedError {}

/// Formats a duration exactly like Go's `time.Duration.String()` so error text
/// matches the Go binary byte-for-byte (e.g. "1.5s", "500ms", "2m3s").
pub(crate) fn format_go_duration(d: Duration) -> String {
    // Go durations are int64 nanoseconds; clamp to that representable range.
    let total_nanos = d.as_nanos();
    let u = if total_nanos > i64::MAX as u128 {
        i64::MAX as u64
    } else {
        total_nanos as u64
    };

    const NS_PER_US: u64 = 1_000;
    const NS_PER_MS: u64 = 1_000_000;
    const NS_PER_S: u64 = 1_000_000_000;

    // buffer filled from the right, mirroring Go's implementation
    let mut buf = [0u8; 32];
    let mut w = buf.len();

    if u < NS_PER_S {
        // less than one second: use ns, µs, or ms
        let prec: usize;
        w -= 1;
        buf[w] = b's';
        w -= 1;
        if u == 0 {
            return "0s".to_string();
        } else if u < NS_PER_US {
            prec = 0;
            buf[w] = b'n';
        } else if u < NS_PER_MS {
            prec = 3;
            // U+00B5 'µ' (micro sign) == bytes 0xC2 0xB5
            buf[w] = 0xB5;
            w -= 1;
            buf[w] = 0xC2;
        } else {
            prec = 6;
            buf[w] = b'm';
        }
        let (nw, nu) = fmt_frac(&mut buf, w, u, prec);
        w = nw;
        w = fmt_int(&mut buf, w, nu);
    } else {
        let mut u = u;
        w -= 1;
        buf[w] = b's';
        let (nw, nu) = fmt_frac(&mut buf, w, u, 9);
        w = nw;
        u = nu;
        // u is now integer seconds
        w = fmt_int(&mut buf, w, u % 60);
        u /= 60;
        if u > 0 {
            w -= 1;
            buf[w] = b'm';
            w = fmt_int(&mut buf, w, u % 60);
            u /= 60;
            if u > 0 {
                w -= 1;
                buf[w] = b'h';
                w = fmt_int(&mut buf, w, u);
            }
        }
    }

    String::from_utf8_lossy(&buf[w..]).into_owned()
}

/// Port of Go's `fmtFrac`: formats the fraction of v/10^prec, omitting trailing
/// zeros. Returns the new write index and the remaining integer value.
fn fmt_frac(buf: &mut [u8; 32], mut w: usize, mut v: u64, prec: usize) -> (usize, u64) {
    let mut print = false;
    for _ in 0..prec {
        let digit = v % 10;
        print = print || digit != 0;
        if print {
            w -= 1;
            buf[w] = digit as u8 + b'0';
        }
        v /= 10;
    }
    if print {
        w -= 1;
        buf[w] = b'.';
    }
    (w, v)
}

/// Port of Go's `fmtInt`: formats v in decimal into the buffer from the right.
fn fmt_int(buf: &mut [u8; 32], mut w: usize, mut v: u64) -> usize {
    if v == 0 {
        w -= 1;
        buf[w] = b'0';
    } else {
        while v > 0 {
            w -= 1;
            buf[w] = (v % 10) as u8 + b'0';
            v /= 10;
        }
    }
    w
}
