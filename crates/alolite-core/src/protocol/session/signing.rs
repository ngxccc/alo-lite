//! Parameter encryption, common query parameters, and AES response decryption.

use anyhow::Result;
use serde_json::Value;

use super::privacy::{truncate, urlencode};
use super::{Session, ZPW_TYPE, ZPW_VER};
use crate::crypto;

impl Session {
    /// `_constructUrlParams(se)` → `zpw_ver=695&zpw_type=23`.
    pub fn common_params(&self) -> String {
        format!("zpw_ver={ZPW_VER}&zpw_type={ZPW_TYPE}")
    }

    /// The raw session AES key as bytes, for response decryption.
    pub fn session_key_bytes(&self) -> &[u8] {
        self.secret_key.as_deref().unwrap_or(&[])
    }

    /// `getEncodedParams(obj)` → url-encoded `encodeAES(json)` (`INV-1`).
    ///
    /// # Errors
    /// Returns an error if the session lacks an AES secret key or AES-CBC encryption fails.
    pub fn encoded_params(&self, v: &Value) -> Result<String> {
        let key = self
            .secret_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no session key yet (pre-login)"))?;
        let b64 = crypto::encode_aes(key, &v.to_string())?;
        Ok(urlencode(&b64))
    }

    /// Plain `params=<url-encoded json>` for pre-login endpoints.
    pub fn plain_params(v: &Value) -> String {
        format!("params={}", urlencode(&v.to_string()))
    }

    /// `decryptResp` — when the request was param-encrypted, `data` comes back as base64 AES.
    pub async fn maybe_decrypt(&self, v: Value, key: &[u8]) -> Value {
        let Some(data) = v.get("data") else {
            return v;
        };
        let Some(b64) = data.as_str() else {
            return v;
        };
        if b64.is_empty() || b64.starts_with('{') || key.is_empty() {
            return v;
        }
        match crate::crypto::decode_aes(key, b64) {
            Ok(pt) => match serde_json::from_str::<Value>(&pt) {
                Ok(inner) => {
                    if self.verbose {
                        eprintln!("  data decrypted ({} bytes)", pt.len());
                    }
                    inner
                }
                Err(e) => {
                    if self.verbose {
                        eprintln!(
                            "  data decrypted but not JSON: {e} / {}",
                            truncate(&pt, 200)
                        );
                    }
                    v
                }
            },
            Err(e) => {
                if self.verbose {
                    eprintln!("  data decrypt failed: {e}");
                }
                v
            }
        }
    }
}
