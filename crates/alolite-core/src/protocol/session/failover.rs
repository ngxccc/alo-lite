//! Multi-candidate host failover and registry-based API invocations.

use std::future::Future;

use anyhow::Result;
use serde_json::Value;

use super::status::SessionError;
use super::Session;

impl Session {
    /// True only when an error is a transport failure or HTTP 404 (#27).
    pub fn should_failover_candidate(err: &anyhow::Error) -> bool {
        if err.downcast_ref::<SessionError>().is_some() {
            return false;
        }
        if err.downcast_ref::<reqwest::Error>().is_some() {
            return true;
        }
        let msg = err.to_string();
        msg.starts_with("HTTP 404")
            || msg.contains("HTTP 404 ")
            || msg.contains("connection refused")
            || msg.contains("timed out")
            || msg.contains("dns error")
    }

    /// Try each candidate host for `endpoint` in priority order until one succeeds (#27).
    pub async fn request_with_candidates<F, Fut>(
        &self,
        endpoint: &str,
        path_and_query: &str,
        mut send_fn: F,
    ) -> Result<Value>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = Result<Value>>,
    {
        let candidates = self.domains.candidates(endpoint);
        let suffix = if path_and_query.starts_with('/') {
            path_and_query.to_string()
        } else {
            format!("/{path_and_query}")
        };
        let mut last_err: Option<anyhow::Error> = None;
        for host in candidates {
            let url = format!("{host}{suffix}");
            match send_fn(url).await {
                Ok(val) => {
                    self.domains.remember_winner(endpoint, &host);
                    return Ok(val);
                }
                Err(e) => {
                    if !Self::should_failover_candidate(&e) {
                        return Err(e);
                    }
                    crate::logger::log_error(
                        &format!("session::failover::{endpoint}"),
                        &format!("host {host} failed ({e}), trying next candidate"),
                    );
                    last_err = Some(e);
                }
            }
        }
        Err(last_err
            .unwrap_or_else(|| anyhow::anyhow!("no host candidates for endpoint {endpoint}")))
    }

    /// Perform a `GET` request against `endpoint` with multi-candidate host failover (#27).
    pub async fn get_endpoint(&self, endpoint: &str, path_and_query: &str) -> Result<Value> {
        self.request_with_candidates(endpoint, path_and_query, |url| async move {
            self.get(&url).await
        })
        .await
    }

    /// Perform a `POST` form request against `endpoint` with multi-candidate host failover (#27).
    pub async fn post_endpoint_form(
        &self,
        endpoint: &str,
        path_and_query: &str,
        body: String,
    ) -> Result<Value> {
        self.request_with_candidates(endpoint, path_and_query, |url| {
            let b = body.clone();
            async move { self.post_form(&url, b).await }
        })
        .await
    }

    /// Execute an authenticated `GET` against a centralized [`crate::protocol::registry::CommandSpec`] (`INV-1`).
    ///
    /// # Errors
    /// Returns an error if parameter encryption fails or all candidate hosts fail.
    pub async fn api_get(
        &self,
        spec: crate::protocol::registry::CommandSpec,
        params: Option<&Value>,
    ) -> Result<Value> {
        let pq = spec.query_path(self, params)?;
        let resp = self.get_endpoint(spec.domain.service_key(), &pq).await?;
        Ok(self.maybe_decrypt(resp, self.session_key_bytes()).await)
    }

    /// Execute an authenticated `POST` against a centralized [`crate::protocol::registry::CommandSpec`] (`INV-1`).
    ///
    /// # Errors
    /// Returns an error if parameter encryption fails or all candidate hosts fail.
    pub async fn api_post(
        &self,
        spec: crate::protocol::registry::CommandSpec,
        params: Option<&Value>,
    ) -> Result<Value> {
        let (pq, form) = spec.post_parts(self, params)?;
        let resp = self
            .post_endpoint_form(spec.domain.service_key(), &pq, form)
            .await?;
        Ok(self.maybe_decrypt(resp, self.session_key_bytes()).await)
    }

    /// Look up `command_id` in [`crate::protocol::registry::COMMAND_REGISTRY`] and execute [`Self::api_get`].
    ///
    /// # Errors
    /// Returns an error if `command_id` is unknown or the request fails.
    pub async fn api_get_id(&self, command_id: u32, params: Option<&Value>) -> Result<Value> {
        let spec = crate::protocol::registry::lookup_command(command_id)
            .ok_or_else(|| anyhow::anyhow!("unknown Zalo HTTP commandId: {command_id}"))?;
        self.api_get(spec, params).await
    }

    /// Look up `command_id` in [`crate::protocol::registry::COMMAND_REGISTRY`] and execute [`Self::api_post`].
    ///
    /// # Errors
    /// Returns an error if `command_id` is unknown or the request fails.
    pub async fn api_post_id(&self, command_id: u32, params: Option<&Value>) -> Result<Value> {
        let spec = crate::protocol::registry::lookup_command(command_id)
            .ok_or_else(|| anyhow::anyhow!("unknown Zalo HTTP commandId: {command_id}"))?;
        self.api_post(spec, params).await
    }
}
