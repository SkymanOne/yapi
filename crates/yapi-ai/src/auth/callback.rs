//! The loopback server that receives an OAuth redirect, and the race between
//! it and a pasted code. Port of `oauth/callback-server.ts` and
//! `utils/oauth-page.ts`.

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::{AuthError, AuthPrompt, Interaction};

const PAGE: &str = include_str!("oauth-page.html");

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_page(title: &str, message: &str, details: Option<&str>) -> String {
    let details = details.map_or_else(String::new, |details| {
        format!("<div class=\"details\">{}</div>", escape_html(details))
    });
    PAGE.replace("__TITLE__", &escape_html(title))
        .replace("__HEADING__", &escape_html(title))
        .replace("__MESSAGE__", &escape_html(message))
        .replace("__DETAILS__", &details)
}

/// pi's success page.
pub fn success_page(message: &str) -> String {
    render_page("Authentication successful", message, None)
}

/// pi's failure page.
pub fn error_page(message: &str, details: Option<&str>) -> String {
    render_page("Authentication failed", message, details)
}

/// The reply to one request, and what it means for the sign-in.
pub struct Reply<T> {
    /// HTTP status.
    pub status: u16,
    /// The page.
    pub html: String,
    /// Ends the wait with a value or an error.
    pub outcome: Option<Result<T, AuthError>>,
}

impl<T> Reply<T> {
    /// A page that does not end the wait.
    pub fn page(status: u16, html: String) -> Reply<T> {
        Reply {
            status,
            html,
            outcome: None,
        }
    }
}

/// Answers one request, given its method and URL.
pub type Handler<T> = Box<dyn FnMut(&str, &Url) -> Reply<T> + Send>;

/// The browser's page for a callback whose sign-in finishes first.
pub struct PendingPage {
    page: oneshot::Sender<(u16, String)>,
    sent: oneshot::Receiver<()>,
}

impl PendingPage {
    /// Answers with pi's page for `result`, signed in or 502 with the error,
    /// and waits until it is sent.
    pub async fn finish<U>(self, provider: &str, result: &Result<U, AuthError>) {
        let page = match result {
            Ok(_) => (
                200,
                success_page(&format!(
                    "Signed in to {provider}. You may now close this page."
                )),
            ),
            Err(err) => (
                502,
                error_page(
                    &format!("{provider} sign-in failed."),
                    Some(&err.to_string()),
                ),
            ),
        };
        if self.page.send(page).is_ok() {
            let _ = self.sent.await;
        }
    }
}

type Outcome<T> = (Result<T, AuthError>, Option<PendingPage>);

/// A running loopback server. Dropping it closes the listener and every open
/// connection.
pub struct Server<T> {
    port: u16,
    redirect_uri: String,
    outcome: oneshot::Receiver<Outcome<T>>,
    cancel: CancellationToken,
    tasks: tokio::task::JoinHandle<()>,
}

impl<T: Send + 'static> Server<T> {
    /// Listens on `host:port` (0 picks a port) and answers each request with
    /// `handler`, until a reply carries an outcome.
    pub async fn start(
        host: &str,
        port: u16,
        path: &str,
        cancel: &CancellationToken,
        handler: Handler<T>,
    ) -> std::io::Result<Server<T>> {
        Self::listen(host, port, path, cancel, handler, false).await
    }

    /// [`Server::start`] for a sign-in that finishes before the browser's
    /// page, as pi's `complete` option: the reply that yields a value waits
    /// for the [`PendingPage`] from [`Server::wait_pending`].
    pub async fn start_pending(
        host: &str,
        port: u16,
        path: &str,
        cancel: &CancellationToken,
        handler: Handler<T>,
    ) -> std::io::Result<Server<T>> {
        Self::listen(host, port, path, cancel, handler, true).await
    }

    async fn listen(
        host: &str,
        port: u16,
        path: &str,
        cancel: &CancellationToken,
        handler: Handler<T>,
        pending: bool,
    ) -> std::io::Result<Server<T>> {
        let listener = TcpListener::bind((host, port)).await?;
        let port = listener.local_addr()?.port();
        let redirect_uri = format!("http://{host}:{port}{path}");
        let (sender, outcome) = oneshot::channel();
        let state = Arc::new(Mutex::new((handler, Some(sender), pending)));
        let tasks = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            while let Ok((stream, _)) = listener.accept().await {
                connections.spawn(serve_connection(stream, Arc::clone(&state)));
            }
        });
        Ok(Server {
            port,
            redirect_uri,
            outcome,
            cancel: cancel.clone(),
            tasks,
        })
    }

    /// `http://<host>:<port><path>`, with the port the listener bound.
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// The port the listener bound.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The outcome of the first conclusive request.
    pub async fn wait(&mut self) -> Result<T, AuthError> {
        self.wait_pending().await.map(|(value, _)| value)
    }

    /// [`Server::wait`], with the page a server from
    /// [`Server::start_pending`] holds until the sign-in finishes.
    pub async fn wait_pending(&mut self) -> Result<(T, Option<PendingPage>), AuthError> {
        match self.cancel.run_until_cancelled(&mut self.outcome).await {
            Some(Ok((outcome, page))) => outcome.map(|value| (value, page)),
            Some(Err(_)) => Err(AuthError::failed("OAuth callback server closed")),
            None => Err(AuthError::Cancelled),
        }
    }
}

impl<T> Drop for Server<T> {
    fn drop(&mut self) {
        self.tasks.abort();
    }
}

/// The handler, the sender of the outcome, and whether a value's page waits.
type Shared<T> = Arc<Mutex<(Handler<T>, Option<oneshot::Sender<Outcome<T>>>, bool)>>;

async fn serve_connection<T>(mut stream: TcpStream, state: Shared<T>) {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 2048];
    while !buffer.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
        }
        if buffer.len() > 16 * 1024 {
            return;
        }
    }
    let head = String::from_utf8_lossy(&buffer);
    let mut parts = head.lines().next().unwrap_or_default().split(' ');
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or("/");
    let url = Url::parse("http://localhost").and_then(|base| base.join(target));
    let (mut status, mut html, outcome, pending) = match url {
        Ok(url) => {
            let mut guard = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let (handler, sender, pending) = &mut *guard;
            let reply = handler(&method, &url);
            let outcome = reply
                .outcome
                .and_then(|outcome| sender.take().map(|sender| (sender, outcome)));
            (reply.status, reply.html, outcome, *pending)
        }
        Err(_) => (
            404,
            error_page("Callback route not found.", None),
            None,
            false,
        ),
    };
    let mut sent = None;
    let outcome = match outcome {
        Some((sender, Ok(value))) if pending => {
            let (page, chosen) = oneshot::channel();
            let (done, waiting) = oneshot::channel();
            sent = Some(done);
            let page = PendingPage {
                page,
                sent: waiting,
            };
            let _ = sender.send((Ok(value), Some(page)));
            // A sign-in that ends without choosing keeps the handler's page.
            if let Ok(page) = chosen.await {
                (status, html) = page;
            }
            None
        }
        outcome => outcome,
    };
    let reason = reqwest::StatusCode::from_u16(status)
        .ok()
        .and_then(|code| code.canonical_reason())
        .unwrap_or("Error");
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: text/html; charset=utf-8\r\ncache-control: no-store\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{html}",
        html.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
    if let Some(sent) = sent {
        let _ = sent.send(());
    }
    if let Some((sender, outcome)) = outcome {
        let _ = sender.send((outcome, None));
    }
}

/// The first value of the query parameter `name`.
pub fn query(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

/// A server for pi's shared callback: GET `path` with `code` (and `state` when
/// expected) yields the code.
pub(crate) async fn start_code_server(
    provider_name: &str,
    host: &str,
    port: u16,
    path: &str,
    state: Option<String>,
    cancel: &CancellationToken,
) -> std::io::Result<Server<String>> {
    let provider = provider_name.to_owned();
    let expected_path = path.to_owned();
    let mut claimed = false;
    let handler = move |method: &str, url: &Url| -> Reply<String> {
        if method != "GET" || url.path() != expected_path {
            return Reply::page(404, error_page("Callback route not found.", None));
        }
        if let Some(state) = &state
            && query(url, "state").as_deref() != Some(state)
        {
            return Reply::page(400, error_page("State mismatch.", None));
        }
        if claimed {
            return Reply::page(
                409,
                error_page("This sign-in has already been handled.", None),
            );
        }
        if let Some(error) = query(url, "error") {
            let description = query(url, "error_description").unwrap_or(error);
            claimed = true;
            return Reply {
                status: 400,
                html: error_page(
                    &format!("{provider} authorization failed."),
                    Some(&description),
                ),
                outcome: Some(Err(AuthError::Failed(format!(
                    "{provider} authorization failed: {description}"
                )))),
            };
        }
        let Some(code) = query(url, "code") else {
            return Reply::page(400, error_page("Missing authorization code.", None));
        };
        claimed = true;
        Reply {
            status: 200,
            html: success_page(&format!(
                "Signed in to {provider}. You may now close this page."
            )),
            outcome: Some(Ok(code)),
        }
    };
    Server::start(host, port, path, cancel, Box::new(handler)).await
}

/// Where the authorization arrived from.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Received<T> {
    /// The browser reached the loopback server.
    Callback(T),
    /// The user pasted input.
    Manual(String),
}

/// Waits for the browser callback or for pasted input, whichever comes first.
/// Without a server only the prompt is used. Port of
/// `waitForCallbackOrManualInput`.
pub(crate) async fn callback_or_manual<T: Send + 'static>(
    interaction: &Interaction,
    server: Option<&mut Server<T>>,
    message: &str,
    placeholder: &str,
) -> Result<Received<T>, AuthError> {
    let withdraw = interaction.cancel().child_token();
    let _withdraw_on_return = withdraw.clone().drop_guard();
    let manual = interaction.prompt_until(
        AuthPrompt::ManualCode {
            message: message.to_owned(),
            placeholder: Some(placeholder.to_owned()),
        },
        withdraw,
    );
    let Some(server) = server else {
        return manual.await.map(Received::Manual);
    };
    tokio::select! {
        value = server.wait() => value.map(Received::Callback),
        input = manual => input.map(Received::Manual),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthRequest;

    async fn get(url: &str) -> (u16, String) {
        let response = reqwest::Client::new().get(url).send().await.unwrap();
        (response.status().as_u16(), response.text().await.unwrap())
    }

    #[tokio::test]
    async fn answers_and_yields_code() {
        let cancel = CancellationToken::new();
        let mut server = start_code_server(
            "Example",
            "127.0.0.1",
            0,
            "/callback",
            Some("s1".into()),
            &cancel,
        )
        .await
        .unwrap();
        let base = server.redirect_uri().to_owned();
        assert!(base.starts_with("http://127.0.0.1:") && base.ends_with("/callback"));
        let root = base.trim_end_matches("/callback");
        assert_eq!(get(&format!("{root}/other")).await.0, 404);
        assert_eq!(get(&format!("{base}?code=c&state=bad")).await.0, 400);
        assert_eq!(get(&format!("{base}?state=s1")).await.0, 400);
        let (status, body) = get(&format!("{base}?code=abc&state=s1")).await;
        assert_eq!(status, 200);
        assert!(body.contains("Signed in to Example. You may now close this page."));
        assert_eq!(server.wait().await, Ok("abc".to_owned()));
    }

    #[tokio::test]
    async fn pasted_input_wins() {
        let cancel = CancellationToken::new();
        let (interaction, mut requests) = Interaction::new(cancel.clone());
        let mut server = start_code_server("Example", "127.0.0.1", 0, "/cb", None, &cancel)
            .await
            .unwrap();
        let answer = tokio::spawn(async move {
            let Some(AuthRequest::Prompt { reply, .. }) = requests.recv().await else {
                panic!("expected a prompt");
            };
            reply.send("pasted".into()).unwrap();
        });
        let received = callback_or_manual(&interaction, Some(&mut server), "Paste", "x")
            .await
            .unwrap();
        answer.await.unwrap();
        assert_eq!(received, Received::Manual("pasted".into()));
    }
}
