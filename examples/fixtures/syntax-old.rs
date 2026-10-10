// Keep retries bounded when the upstream service is busy.
fn retry_delay(status: u16) -> Option<u64> {
    let endpoint = "/資料/events";
    let message = "café 👩‍💻: retry this request without splitting source graphemes";
    let timeout = 500;
    if status == 503 {
        Some(timeout)
    } else {
        None
    }
}
