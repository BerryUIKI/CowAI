//! URL helpers shared by the Anthropic and OpenAI Responses providers.

/// Append `suffix` (for example `/messages`) to a provider base URL.
///
/// Accepts a bare host, a host with `/v1`, or an endpoint that already ends
/// with the suffix, so a pasted full endpoint URL works unchanged.
pub(crate) fn endpoint_url(base_url: &str, suffix: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with(suffix) {
        base.to_string()
    } else if base.ends_with("/v1") {
        format!("{base}{suffix}")
    } else {
        format!("{base}/v1{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_the_version_prefix_when_missing() {
        assert_eq!(
            endpoint_url("https://api.anthropic.com", "/messages"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            endpoint_url("http://127.0.0.1:8788/", "/messages"),
            "http://127.0.0.1:8788/v1/messages"
        );
    }

    #[test]
    fn keeps_an_existing_version_prefix() {
        assert_eq!(
            endpoint_url("https://api.openai.com/v1", "/responses"),
            "https://api.openai.com/v1/responses"
        );
    }

    #[test]
    fn accepts_a_full_endpoint_url() {
        assert_eq!(
            endpoint_url("http://127.0.0.1:8788/v1/messages", "/messages"),
            "http://127.0.0.1:8788/v1/messages"
        );
        assert_eq!(
            endpoint_url("https://api.openai.com/v1/responses/", "/responses"),
            "https://api.openai.com/v1/responses"
        );
    }
}
