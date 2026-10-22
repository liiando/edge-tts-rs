//! Constants for the edge-tts package.

/// Base URL for the Microsoft Edge TTS service
pub const BASE_URL: &str = "speech.platform.bing.com/consumer/speech/synthesize/readaloud";

/// Trusted client token for authentication
pub const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";

/// Default voice name
pub const DEFAULT_VOICE: &str = "en-US-EmmaMultilingualNeural";

/// Chromium full version string
pub const CHROMIUM_FULL_VERSION: &str = "143.0.3650.75";

/// Chromium major version (parsed from full version)
pub const CHROMIUM_MAJOR_VERSION: &str = "143";

/// Security token version
pub const SEC_MS_GEC_VERSION: &str = "2-143.0.3650.75";

/// Generate the WebSocket URL for TTS
pub fn wss_url() -> String {
    format!(
        "wss://{}/edge/v1?TrustedClientToken={}",
        BASE_URL, TRUSTED_CLIENT_TOKEN
    )
}

/// Generate the voice list URL
pub fn voice_list_url() -> String {
    format!(
        "https://{}/voices/list?trustedclienttoken={}",
        BASE_URL, TRUSTED_CLIENT_TOKEN
    )
}

/// Get base headers for HTTP requests
pub fn base_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    
    headers.insert(
        reqwest::header::USER_AGENT,
        format!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/{}.0.0.0 Safari/537.36 \
             Edg/{}.0.0.0",
            CHROMIUM_MAJOR_VERSION, CHROMIUM_MAJOR_VERSION
        )
        .parse()
        .unwrap(),
    );
    
    headers.insert(
        reqwest::header::ACCEPT_ENCODING,
        "gzip, deflate, br, zstd".parse().unwrap(),
    );
    
    headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        "en-US,en;q=0.9".parse().unwrap(),
    );
    
    headers
}

/// Get headers for WebSocket connections
pub fn wss_headers() -> reqwest::header::HeaderMap {
    let mut headers = base_headers();
    
    headers.insert(
        reqwest::header::HeaderName::from_static("pragma"),
        "no-cache".parse().unwrap(),
    );
    
    headers.insert(
        reqwest::header::CACHE_CONTROL,
        "no-cache".parse().unwrap(),
    );
    
    headers.insert(
        reqwest::header::ORIGIN,
        "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold"
            .parse()
            .unwrap(),
    );
    
    headers.insert(
        reqwest::header::HeaderName::from_static("sec-websocket-version"),
        "13".parse().unwrap(),
    );
    
    headers
}

/// Get headers for voice list API requests
pub fn voice_headers() -> reqwest::header::HeaderMap {
    let mut headers = base_headers();
    
    headers.insert(
        reqwest::header::HeaderName::from_static("authority"),
        "speech.platform.bing.com".parse().unwrap(),
    );
    
    headers.insert(
        reqwest::header::HeaderName::from_static("sec-ch-ua"),
        format!(
            r#"" Not;A Brand";v="99", "Microsoft Edge";v="{}", "Chromium";v="{}""#,
            CHROMIUM_MAJOR_VERSION, CHROMIUM_MAJOR_VERSION
        )
        .parse()
        .unwrap(),
    );
    
    headers.insert(
        reqwest::header::HeaderName::from_static("sec-ch-ua-mobile"),
        "?0".parse().unwrap(),
    );
    
    headers.insert(
        reqwest::header::ACCEPT,
        "*/*".parse().unwrap(),
    );
    
    headers.insert(
        reqwest::header::HeaderName::from_static("sec-fetch-site"),
        "none".parse().unwrap(),
    );
    
    headers.insert(
        reqwest::header::HeaderName::from_static("sec-fetch-mode"),
        "cors".parse().unwrap(),
    );
    
    headers.insert(
        reqwest::header::HeaderName::from_static("sec-fetch-dest"),
        "empty".parse().unwrap(),
    );
    
    headers
}
