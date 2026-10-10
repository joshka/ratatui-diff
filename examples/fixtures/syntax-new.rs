// Respect rate limits as well as temporary service failures.
fn retry_delay(status: u16) -> Option<u64> {
    let endpoint = "/資料/events";
    let message = "café 👩‍💻: retry this request without splitting source graphemes";
    let timeout = 1500;
    if matches!(status, 429 | 503) {
        Some(timeout)
    } else {
        None
    }
}
