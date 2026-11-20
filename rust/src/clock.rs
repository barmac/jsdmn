//! Wall clock used by `now()` and `today()`.

/// Milliseconds since the Unix epoch (UTC).
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn now_millis() -> i128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i128)
        .unwrap_or(0)
}

/// Milliseconds since the Unix epoch (UTC), from the JavaScript clock.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub fn now_millis() -> i128 {
    js_sys::Date::now() as i128
}
