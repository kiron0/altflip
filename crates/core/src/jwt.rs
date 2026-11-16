use base64::Engine;
use serde_json::Value;

pub fn claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn email(token: &str) -> Option<String> {
    claims(token)?.get("email")?.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_unverified_claims() {
        let body = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"email":"a@b.c","n":1}"#);
        let token = format!("x.{body}.sig");
        assert_eq!(email(&token).as_deref(), Some("a@b.c"));
        assert!(claims("garbage").is_none());
    }
}
