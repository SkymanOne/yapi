//! The capabilities a build declares: the crate features that enable the
//! requests needing a grant, and the `yapi:capabilities` custom section that
//! records them for `chest check`.

/// The feature each request or operation kind needs, for the kinds yapi
/// refuses without the matching grant whatever their payload. Kinds whose
/// grant depends on the payload, such as `tool.execute` and
/// `ui.editorSubmit`, are left to yapi.
#[doc(hidden)]
pub const KIND_FEATURES: &[(&str, &str)] = &[
    ("context.files", "fs"),
    ("fetch", "net"),
    ("fetch.stream", "net"),
    ("dns.lookup", "net"),
    ("ai.stream", "net"),
    ("ai.images", "net"),
    ("models.stream", "net"),
    ("models.auth", "net"),
    ("models.requestAuth", "net"),
    ("models.apiKey", "net"),
    ("models.classify", "net"),
    ("models.generateImages", "net"),
    ("env", "env"),
    ("session.sendMessage", "session"),
    ("session.sendUserMessage", "session"),
    ("session.appendEntry", "session"),
    ("session.setName", "session"),
    ("session.setLabel", "session"),
    ("session.write", "session"),
    ("session.new", "session"),
    ("session.fork", "session"),
    ("session.navigateTree", "session"),
    ("session.switch", "session"),
    ("session.reload", "session"),
    ("session.replaced", "session"),
    ("agent.compact", "session"),
    ("agent.setSystemPromptOptions", "session"),
    ("agent.abort", "session"),
    ("agent.shutdown", "session"),
    ("model.set", "session"),
    ("thinking.set", "session"),
    ("tools.setActive", "session"),
    ("exec.sync", "all"),
    ("process.spawn", "all"),
    ("providers.register", "all"),
    ("providers.unregister", "all"),
];

fn enabled(feature: &str) -> bool {
    match feature {
        "fs" => cfg!(feature = "fs"),
        "net" => cfg!(feature = "net"),
        "env" => cfg!(feature = "env"),
        "session" => cfg!(feature = "session"),
        _ => cfg!(feature = "all"),
    }
}

/// `Err` when this build leaves out the feature that `kind` needs.
pub(crate) fn check(kind: &str) -> Result<(), String> {
    match KIND_FEATURES.iter().find(|(name, _)| *name == kind) {
        Some((_, feature)) if !enabled(feature) => Err(format!(
            "request `{kind}` needs the `{feature}` feature of yapi-extension-api"
        )),
        _ => Ok(()),
    }
}

// The `yapi:capabilities` section: a marker line, so that a component without
// the section was built without this SDK, then a line per capability feature.
// The linker concatenates the statics.
#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = "yapi:capabilities")]
static MARKER: [u8; 19] = *b"yapi-extension-api\n";
#[cfg(all(target_arch = "wasm32", feature = "fs"))]
#[used]
#[unsafe(link_section = "yapi:capabilities")]
static FS: [u8; 3] = *b"fs\n";
#[cfg(all(target_arch = "wasm32", feature = "net"))]
#[used]
#[unsafe(link_section = "yapi:capabilities")]
static NET: [u8; 4] = *b"net\n";
#[cfg(all(target_arch = "wasm32", feature = "env"))]
#[used]
#[unsafe(link_section = "yapi:capabilities")]
static ENV: [u8; 4] = *b"env\n";
#[cfg(all(target_arch = "wasm32", feature = "session"))]
#[used]
#[unsafe(link_section = "yapi:capabilities")]
static SESSION: [u8; 8] = *b"session\n";
#[cfg(all(target_arch = "wasm32", feature = "all"))]
#[used]
#[unsafe(link_section = "yapi:capabilities")]
static ALL: [u8; 4] = *b"all\n";

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    use serde_json::json;

    /// Without the features, a kind that needs one fails before it reaches
    /// the host, which a native test build does not have.
    #[cfg(not(any(feature = "session", feature = "net", feature = "all")))]
    #[test]
    fn request_needs_feature() {
        assert_eq!(
            crate::request("session.appendEntry", &json!({})),
            Err(
                "request `session.appendEntry` needs the `session` feature of yapi-extension-api"
                    .into()
            )
        );
        assert_eq!(
            crate::request("exec.sync", &json!({"command": "true"})),
            Err("request `exec.sync` needs the `all` feature of yapi-extension-api".into())
        );
        let mut fetch = std::pin::pin!(crate::op("fetch", &json!({})));
        assert_eq!(
            fetch.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Err(
                "request `fetch` needs the `net` feature of yapi-extension-api".into()
            ))
        );
    }
}
