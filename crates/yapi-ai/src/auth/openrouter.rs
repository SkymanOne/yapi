//! OpenRouter OAuth: a PKCE sign-in that trades the authorization code for a
//! permanent, user-controlled API key. Port of `oauth/openrouter.ts`.

use std::time::Duration;

use serde_json::{Map, Value, json};
use tokio_util::sync::CancellationToken;
use url::Url;
use yapi_types::auth::OAuthCredential;

use super::callback::{Received, callback_or_manual, query, start_code_server};
use super::{
    AuthError, AuthEvent, BoxFuture, Interaction, LoginOptions, OAuthProvider, callback_host, form,
    pkce, send,
};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const TOKEN_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);
/// pi's credentials for OpenRouter never expire: `Number.MAX_SAFE_INTEGER`.
const NEVER: u64 = 9_007_199_254_740_991;

/// The OpenRouter sign-in and its endpoints.
#[derive(Clone, Debug)]
pub struct OpenRouterOAuth {
    /// Authorization page.
    pub authorize_url: String,
    /// Key exchange endpoint.
    pub token_url: String,
}

impl Default for OpenRouterOAuth {
    fn default() -> OpenRouterOAuth {
        OpenRouterOAuth {
            authorize_url: "https://openrouter.ai/auth".into(),
            token_url: "https://openrouter.ai/api/v1/auth/keys".into(),
        }
    }
}

/// The code in pasted input: a redirect URL, a `code=` query, or the code.
fn parse_input(input: &str) -> Option<String> {
    let value = input.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(url) = Url::parse(value) {
        return query(&url, "code");
    }
    if value.contains("code=") {
        return url::form_urlencoded::parse(value.as_bytes())
            .find(|(key, _)| key == "code")
            .map(|(_, code)| code.into_owned());
    }
    Some(value.to_owned())
}

fn error_detail(body: &Value) -> Option<String> {
    ["error_description", "message", "error"]
        .iter()
        .find_map(|key| body[*key].as_str().map(str::to_owned))
        .or_else(|| body["error"]["message"].as_str().map(str::to_owned))
}

impl OpenRouterOAuth {
    async fn exchange(
        &self,
        code: &str,
        verifier: &str,
        cancel: &CancellationToken,
    ) -> Result<OAuthCredential, AuthError> {
        let request = crate::http::client()
            .post(&self.token_url)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .body(yapi_types::json::stringify(&json!({
                "code": code,
                "code_verifier": verifier,
                "code_challenge_method": "S256",
            })));
        let response =
            match tokio::time::timeout(TOKEN_EXCHANGE_TIMEOUT, send(request, cancel)).await {
                Ok(response) => response?,
                Err(_) => {
                    return Err(AuthError::failed(
                        "OpenRouter OAuth token exchange timed out",
                    ));
                }
            };
        let status = response.status();
        let bytes = response.bytes().await.unwrap_or_default();
        let body = match serde_json::from_slice::<Value>(&bytes) {
            Ok(object @ Value::Object(_)) => object,
            Ok(_) => Value::Object(Map::new()),
            Err(_) if status.is_success() => {
                return Err(AuthError::failed("OpenRouter OAuth returned invalid JSON"));
            }
            Err(_) => Value::Object(Map::new()),
        };
        if !status.is_success() {
            let detail = error_detail(&body)
                .map(|detail| format!(": {detail}"))
                .unwrap_or_default();
            return Err(AuthError::Failed(format!(
                "OpenRouter OAuth key exchange failed (HTTP {}){detail}",
                status.as_u16()
            )));
        }
        let key = body["key"]
            .as_str()
            .filter(|key| !key.is_empty())
            .ok_or_else(|| AuthError::failed("OpenRouter OAuth response carries no \"key\""))?;
        Ok(OAuthCredential {
            access: key.to_owned(),
            refresh: String::new(),
            expires: NEVER as f64,
            extra: Map::new(),
        })
    }

    async fn login_openrouter(
        &self,
        interaction: &Interaction,
    ) -> Result<OAuthCredential, AuthError> {
        let pkce = pkce::generate()?;
        // OpenRouter sends no `state`; the random path keeps stray requests
        // from completing the sign-in.
        let uuid = yapi_types::time::try_uuid_v4().map_err(AuthError::random)?;
        let path = format!("/oauth/callback/{uuid}");
        let mut server = start_code_server(
            "OpenRouter",
            &callback_host(),
            0,
            &path,
            None,
            true,
            interaction.cancel(),
        )
        .await
        .map_err(|err| AuthError::Failed(err.to_string()))?;
        let redirect_uri = server.redirect_uri().to_owned();
        let query = form(&[
            ("callback_url", &redirect_uri),
            ("code_challenge", &pkce.challenge),
            ("code_challenge_method", "S256"),
        ]);
        interaction.notify(AuthEvent::Progress {
            message: format!("Listening for OpenRouter OAuth callback on {redirect_uri}"),
        });
        interaction.notify(AuthEvent::AuthUrl {
            url: format!("{}?{query}", self.authorize_url),
            instructions: Some("Complete sign-in in your browser. If the browser is on another machine, paste the final redirect URL here.".into()),
        });
        let received = callback_or_manual(
            interaction,
            Some(&mut server),
            "Complete sign-in in your browser, or paste the authorization code / redirect URL here:",
            &redirect_uri,
        );
        let received = tokio::time::timeout(LOGIN_TIMEOUT, received)
            .await
            .unwrap_or_else(|_| Err(AuthError::failed("OpenRouter sign-in timed out")))?;
        let (code, page) = match received {
            Received::Callback(callback) => callback,
            Received::Manual(input) => {
                let code = parse_input(&input)
                    .ok_or_else(|| AuthError::failed("Missing authorization code"))?;
                interaction.notify(AuthEvent::Progress {
                    message: "Exchanging authorization code for an API key...".into(),
                });
                (code, None)
            }
        };
        // pi exchanges the code before the browser's page, which then shows the result.
        let result = self
            .exchange(&code, &pkce.verifier, interaction.cancel())
            .await;
        if let Some(page) = page {
            page.finish("OpenRouter", &result).await;
        }
        result
    }
}

impl OAuthProvider for OpenRouterOAuth {
    fn name(&self) -> &str {
        "OpenRouter OAuth"
    }

    fn login_label(&self) -> Option<&str> {
        Some("Sign in with OpenRouter")
    }

    fn is_subscription(&self) -> bool {
        false
    }

    fn login<'a>(
        &'a self,
        interaction: &'a Interaction,
        _options: &'a LoginOptions,
    ) -> BoxFuture<'a, Result<OAuthCredential, AuthError>> {
        Box::pin(self.login_openrouter(interaction))
    }

    fn refresh<'a>(
        &'a self,
        credential: &'a OAuthCredential,
        _cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<OAuthCredential, AuthError>> {
        Box::pin(async move { Ok(credential.clone()) })
    }
}
