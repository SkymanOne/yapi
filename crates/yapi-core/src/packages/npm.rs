//! A built-in npm client, so packages install without Node: it resolves
//! versions from the registry, verifies and unpacks tarballs into
//! `node_modules` and installs dependencies, hoisting them where Node's
//! resolution allows. It runs no lifecycle scripts, so native addons stay
//! unbuilt, and an extension that loads one fails then. Installs return the
//! tree they unpacked as pins, which [`replay`] reproduces without the
//! registry.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use aws_lc_rs::digest::{SHA1_FOR_LEGACY_USE_ONLY as SHA1, SHA512, digest};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use futures_util::StreamExt as _;
use futures_util::future::BoxFuture;
use serde_json::{Map, Value, json};
use tokio_util::sync::CancellationToken;
use yapi_ai::auth::lock::{FileLock, LockError};

/// Packages yapi provides to extensions itself, so installing them is wasted work.
const HOST_PACKAGES: [&str; 10] = [
    "@earendil-works/pi-coding-agent",
    "@earendil-works/pi-ai",
    "@earendil-works/pi-agent-core",
    "@earendil-works/pi-tui",
    "@mariozechner/pi-coding-agent",
    "@mariozechner/pi-ai",
    "@mariozechner/pi-agent-core",
    "@mariozechner/pi-tui",
    "typebox",
    "@sinclair/typebox",
];

/// Why an npm operation failed. Its message escapes unprintable
/// characters, which names and URLs from a lock or a registry may hold.
#[derive(Debug, thiserror::Error)]
pub enum NpmError {
    /// The registry could not be reached or answered with an error.
    #[error("{}", escaped(.0))]
    Registry(String),
    /// No published version matches.
    #[error("No version of {} matches {}", escaped(.name), escaped(.range))]
    NoMatch {
        /// The package.
        name: String,
        /// The requested range.
        range: String,
    },
    /// The tarball's digest differs from the registry's.
    #[error("Integrity check failed for {}", escaped(.0))]
    Integrity(String),
    /// A package name or folder that would put files outside `node_modules`.
    #[error("{}", escaped(.0))]
    Refused(String),
    /// A file operation failed.
    #[error("{path}: {source}")]
    Io {
        /// Where.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> NpmError + '_ {
    move |source| NpmError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// `text` with its unprintable characters escaped, as `escape_debug` writes
/// them, so that it cannot drive the terminal an error is printed to.
pub(crate) fn escaped(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            // Printable, though `escape_debug` escapes them.
            '\\' | '\'' | '"' => escaped.push(c),
            _ => escaped.extend(c.escape_debug()),
        }
    }
    escaped
}

/// Fails unless `name` is an npm package name, `name` or `@scope/name`,
/// which keeps it inside the `node_modules` it is joined to. As npm, it
/// accepts only URL-safe characters and no leading `_`. No part may start
/// with `.`, so that none is `.git` or another hidden folder. The name is
/// ASCII, so folding its case needs no Unicode rules.
fn check_name(name: &str) -> Result<(), NpmError> {
    let segment = |part: &str| {
        !part.is_empty()
            && !part.starts_with('.')
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-._~!*'()".contains(&byte))
    };
    let valid = match name
        .strip_prefix('@')
        .and_then(|scoped| scoped.split_once('/'))
    {
        Some((scope, name)) => segment(scope) && segment(name),
        None => segment(name) && !name.starts_with('_'),
    };
    if valid {
        Ok(())
    } else {
        Err(NpmError::Refused(format!("Invalid package name: {name}")))
    }
}

/// One package of an installed npm tree, as a lock pins it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Pinned {
    /// The package's folder below the install root: `node_modules/<name>`,
    /// repeated for each level of nesting, with `/` separators.
    pub path: String,
    /// The name in the package's own manifest, which for an alias differs
    /// from its folder's.
    pub name: String,
    /// The installed version.
    pub version: String,
    /// The URL of its tarball.
    pub tarball: String,
    /// The `sha512-<base64>` digest of its tarball, as Subresource Integrity
    /// writes it.
    pub integrity: String,
}

impl Pinned {
    /// The pin at `path` of the package `manifest` describes, whose tarball
    /// is `bytes`. The digest is always sha512, whatever the registry sent.
    fn new(path: String, manifest: &Value, bytes: &[u8]) -> Pinned {
        Pinned {
            path,
            name: text(&manifest["name"]),
            version: text(&manifest["version"]),
            tarball: text(&manifest["dist"]["tarball"]),
            integrity: format!("sha512-{}", STANDARD.encode(digest(&SHA512, bytes))),
        }
    }
}

/// The package names along a pin's `path`, outermost first, which must be
/// `node_modules/<name>` repeated, each `<name>` a valid package name.
pub(crate) fn pin_names(path: &str) -> Result<Vec<String>, NpmError> {
    let refused = || NpmError::Refused(format!("Invalid package path: {path}"));
    let mut names = Vec::new();
    let mut parts = path.split('/');
    while let Some(part) = parts.next() {
        let name = match (part, parts.next()) {
            ("node_modules", Some(scope)) if scope.starts_with('@') => {
                format!("{scope}/{}", parts.next().ok_or_else(refused)?)
            }
            ("node_modules", Some(name)) => name.to_owned(),
            _ => return Err(refused()),
        };
        check_name(&name).map_err(|_| refused())?;
        names.push(name);
    }
    Ok(names)
}

/// The 64-byte digest of an integrity that is exactly one `sha512-` value.
fn sha512_digest(integrity: &str) -> Option<Vec<u8>> {
    let digest = STANDARD.decode(integrity.strip_prefix("sha512-")?).ok()?;
    (digest.len() == 64).then_some(digest)
}

/// Removes the file, link or folder at `path`, never following a link.
pub(crate) fn remove_path(path: &Path) -> std::io::Result<()> {
    match path.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn is_link(path: &Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
}

/// A new, empty folder beside `dir` in which to build its replacement.
/// Refuses when `dir` is a link, which would put the result elsewhere.
pub(crate) fn stage(dir: &Path) -> Result<PathBuf, NpmError> {
    if is_link(dir) {
        return Err(NpmError::Refused(format!(
            "Refusing to install through the link {}",
            dir.display()
        )));
    }
    let name = dir.file_name().unwrap_or_default().to_string_lossy();
    let staging = dir.with_file_name(format!(".{name}-staging"));
    let parent = dir.parent().unwrap_or(dir);
    std::fs::create_dir_all(parent).map_err(io(parent))?;
    // Left over by a stopped install. The root's lock keeps other installs out.
    remove_path(&staging).map_err(io(&staging))?;
    std::fs::create_dir(&staging).map_err(io(&staging))?;
    Ok(staging)
}

/// Puts `staging`, built by `built`, in place of `dir`, so that `dir` holds
/// either what it held or all of `staging`. A failed build only removes
/// `staging`.
pub(crate) fn swap<E: From<NpmError>>(
    staging: &Path,
    dir: &Path,
    built: Result<(), E>,
) -> Result<(), E> {
    if let Err(err) = built {
        let _ = remove_path(staging);
        return Err(err);
    }
    swap_with(staging, dir, |from, to| std::fs::rename(from, to)).map_err(E::from)
}

/// [`swap`], renaming with `rename`. The previous `dir` is deleted only
/// once the new one is in place: when putting it back fails, it stays
/// beside `dir`, and the error names where.
fn swap_with(
    staging: &Path,
    dir: &Path,
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), NpmError> {
    let name = dir.file_name().unwrap_or_default().to_string_lossy();
    let old = dir.with_file_name(format!(".{name}-old"));
    let replacing = dir.symlink_metadata().is_ok();
    let moved = remove_path(&old)
        .map_err(io(&old))
        .and_then(|()| match replacing {
            true => rename(dir, &old).map_err(io(dir)),
            false => Ok(()),
        });
    if let Err(err) = moved {
        let _ = remove_path(staging);
        return Err(err);
    }
    if let Err(err) = rename(staging, dir) {
        let _ = remove_path(staging);
        if replacing && let Err(back) = rename(&old, dir) {
            return Err(NpmError::Io {
                path: dir.to_path_buf(),
                source: std::io::Error::other(format!(
                    "{err}, and restoring it failed ({back}): the previous version is at {}",
                    old.display()
                )),
            });
        }
        return Err(io(dir)(err));
    }
    let _ = remove_path(&old);
    Ok(())
}

/// npm's configuration as the `npm install` pi runs reads it: the user config
/// file (`npm_config_userconfig`, else `~/.npmrc`) under `npm_config_*`
/// environment variables, with `${VAR}` references expanded.
pub fn config() -> HashMap<String, String> {
    let env = |name: &str| std::env::var(name).ok();
    let vars: HashMap<String, String> = std::env::vars_os()
        .filter_map(|(name, value)| {
            let key = env_key(name.to_str()?)?;
            let value = expand(value.to_str()?, env);
            (!value.is_empty()).then_some((key, value))
        })
        .collect();
    let file = vars.get("userconfig").map_or_else(
        || crate::tools::path::home_dir().join(".npmrc"),
        |path| PathBuf::from(crate::tools::path::expand_home(path)),
    );
    let mut config = parse_config(&std::fs::read_to_string(file).unwrap_or_default(), env);
    config.extend(vars);
    config
}

/// The setting environment variable `name` holds, when it is an
/// `npm_config_*` variable, named as npm names it.
fn env_key(name: &str) -> Option<String> {
    let key = name
        .get(..11)
        .filter(|prefix| prefix.eq_ignore_ascii_case("npm_config_"))
        .and(name.get(11..))?;
    // npm keeps `//host/:_authToken` keys as written.
    if key.starts_with("//") {
        return Some(key.to_owned());
    }
    let rest = key.get(1..)?.replace('_', "-");
    Some(format!("{}{rest}", &key[..1]).to_lowercase())
}

/// The top-level settings of an npmrc file, read as npm's `ini` parser reads
/// them, with `${VAR}` references in keys and values expanded from `env`.
fn parse_config(text: &str, env: impl Fn(&str) -> Option<String>) -> HashMap<String, String> {
    let mut config = HashMap::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            // What follows belongs to a section, which npm ignores.
            break;
        }
        if line.starts_with([';', '#']) {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        let quoted = value.len() >= 2
            && (value.starts_with('"') && value.ends_with('"')
                || value.starts_with('\'') && value.ends_with('\''));
        let value = if quoted {
            &value[1..value.len() - 1]
        } else {
            value.split([';', '#']).next().unwrap_or_default().trim()
        };
        let value = expand(value, &env);
        if !value.is_empty() {
            config.insert(expand(key.trim(), &env), value);
        }
    }
    config
}

/// `text` with npm's `${VAR}` references replaced by `env`'s values. An
/// undefined `${VAR}` stays as written and an undefined `${VAR?}` is empty.
/// npm's backslash escapes of references are not supported.
fn expand(text: &str, env: impl Fn(&str) -> Option<String>) -> String {
    let mut expanded = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        expanded.push_str(&rest[..start]);
        rest = &rest[start..];
        let reference = rest[2..].find('}').map(|end| &rest[2..end + 2]);
        let Some(reference) = reference.filter(|name| {
            let name = name.strip_suffix('?').unwrap_or(name);
            !name.is_empty() && !name.contains(['$', '{', '?'])
        }) else {
            expanded.push_str("${");
            rest = &rest[2..];
            continue;
        };
        let (name, optional) = match reference.strip_suffix('?') {
            Some(name) => (name, true),
            None => (reference, false),
        };
        match env(name) {
            Some(value) => expanded.push_str(&value),
            None if optional => {}
            None => expanded.push_str(&rest[..reference.len() + 3]),
        }
        rest = &rest[reference.len() + 3..];
    }
    expanded.push_str(rest);
    expanded
}

/// `token` without the `v` node-semver allows before a version, as in
/// `v1.2.3` or `^v1.2`.
fn without_v(token: &str) -> String {
    let version = token.trim_start_matches(['<', '>', '=', '~', '^']);
    let operator = &token[..token.len() - version.len()];
    match version.strip_prefix('v') {
        Some(version) => format!("{operator}{version}"),
        None => token.to_owned(),
    }
}

/// npm range syntax as `semver` requirements: any alternative may match.
fn requirements(range: &str) -> Option<Vec<semver::VersionReq>> {
    range
        .split("||")
        .map(|alternative| {
            let alternative = alternative.trim();
            let text = if let Some((low, high)) = alternative.split_once(" - ") {
                format!(">={}, <={}", without_v(low.trim()), without_v(high.trim()))
            } else {
                // npm separates comparators with spaces, and `semver` with commas.
                let mut parts: Vec<String> = Vec::new();
                for token in alternative.split_whitespace() {
                    let token = without_v(token);
                    match parts.last_mut() {
                        Some(last)
                            if matches!(
                                last.as_str(),
                                ">" | ">=" | "<" | "<=" | "=" | "~" | "^"
                            ) =>
                        {
                            last.push_str(&token);
                        }
                        // A bare version is exact in npm and a caret range in
                        // `semver`. Wildcards such as `1.x` mean the same in both.
                        _ if token.starts_with(|c: char| c.is_ascii_digit())
                            && !token.contains(['x', 'X', '*']) =>
                        {
                            parts.push(format!("={token}"));
                        }
                        _ => parts.push(token),
                    }
                }
                if parts.is_empty() {
                    "*".into()
                } else {
                    parts.join(", ")
                }
            };
            semver::VersionReq::parse(&text).ok()
        })
        .collect()
}

/// Whether `range` is a version range, not a dist-tag.
pub(crate) fn is_range(range: &str) -> bool {
    requirements(range).is_some()
}

/// Whether installed `version` satisfies `range`.
pub fn satisfies(version: &str, range: &str) -> bool {
    let Ok(version) = semver::Version::parse(version) else {
        return false;
    };
    requirements(range)
        .is_some_and(|alternatives| alternatives.iter().any(|req| req.matches(&version)))
}

/// Whether version `candidate` is later than `current`, as node-semver's
/// `gt` compares them, ignoring surrounding whitespace. `None` when either
/// is not a version, where `gt` throws.
pub fn newer(candidate: &str, current: &str) -> Option<bool> {
    let candidate = semver::Version::parse(candidate.trim()).ok()?;
    let current = semver::Version::parse(current.trim()).ok()?;
    Some(candidate > current)
}

/// The highest of `versions`, each with an item, that `range` matches, or of
/// all of them without a range.
fn highest<'a, T>(
    versions: impl IntoIterator<Item = (&'a str, T)>,
    range: Option<&str>,
) -> Option<(&'a str, T)> {
    let alternatives = match range {
        Some(range) => Some(requirements(range)?),
        None => None,
    };
    versions
        .into_iter()
        .filter_map(|(version, item)| Some((semver::Version::parse(version).ok()?, version, item)))
        .filter(|(parsed, ..)| {
            alternatives
                .as_ref()
                .is_none_or(|alternatives| alternatives.iter().any(|req| req.matches(parsed)))
        })
        .max_by(|(a, ..), (b, ..)| a.cmp(b))
        .map(|(_, version, item)| (version, item))
}

/// The newest version in `npm view <spec> version --json` output: the one
/// version, else the highest in the source's `range` (pi's `maxSatisfying`),
/// or of all without one.
pub(crate) fn viewed_version(output: &str, range: Option<&str>) -> Option<String> {
    match serde_json::from_str::<Value>(output.trim()).ok()? {
        Value::String(version) => Some(version),
        Value::Array(versions) => highest(
            versions
                .iter()
                .filter_map(Value::as_str)
                .filter(|version| !version.is_empty())
                .map(|version| (version, ())),
            range,
        )
        .map(|(version, ())| version.to_owned()),
        _ => None,
    }
}

/// What npm asks a registry for: the abbreviated packument, which holds only
/// what installs need. Full packuments of long-lived packages run to tens of
/// megabytes.
const PACKUMENT_ACCEPT: &str =
    "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*";

/// npm's default `fetch-timeout`.
const FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// How much one package may take, so that a registry or a lock can neither
/// fill memory with a huge download nor the disk with a small gzip bomb.
#[derive(Clone, Copy, Debug)]
struct Caps {
    /// The bytes of its packument, which are held in memory, with headroom
    /// over the largest full packuments on npm.
    packument: usize,
    /// The bytes of its tarball, which are held in memory.
    download: usize,
    /// The bytes it unpacks to, tar headers included.
    unpacked: u64,
    /// Its tar entries.
    entries: usize,
}

// shortcut: fixed caps far above any npm package yapi installs, raise them when a real package hits one.
const CAPS: Caps = Caps {
    packument: 256 << 20,
    download: 512 << 20,
    unpacked: 2 << 30,
    entries: 200_000,
};

/// `bytes` in the largest binary unit that divides them.
fn size(bytes: u64) -> String {
    match bytes {
        _ if bytes.is_multiple_of(1 << 30) => format!("{} GiB", bytes >> 30),
        _ if bytes.is_multiple_of(1 << 20) => format!("{} MiB", bytes >> 20),
        _ => format!("{bytes} bytes"),
    }
}

/// A registry client with a per-run packument cache.
#[derive(Clone)]
pub struct Npm {
    config: HashMap<String, String>,
    packuments: HashMap<String, Value>,
    caps: Caps,
}

impl Npm {
    /// A client with npm's configuration `config`, as [`config`] reads it.
    pub fn new(config: HashMap<String, String>) -> Npm {
        Npm {
            config,
            packuments: HashMap::new(),
            caps: CAPS,
        }
    }

    /// The registry of package `name`, ending in `/`: its scope's
    /// `@scope:registry`, else `registry`, else npm's default.
    fn registry(&self, name: &str) -> String {
        let scope = name.split_once('/').map(|(scope, _)| scope);
        let registry = scope
            .filter(|scope| scope.starts_with('@'))
            .and_then(|scope| self.config.get(&format!("{scope}:registry")))
            .or_else(|| self.config.get("registry"))
            .map_or("https://registry.npmjs.org/", String::as_str);
        format!("{}/", registry.strip_suffix('/').unwrap_or(registry))
    }

    /// The `Authorization` npm sends with a request for `url` from
    /// `registry`: the credentials of the longest `//host/path` key that
    /// covers `url`, else those of `registry` when it is on `url`'s host.
    fn authorization(&self, url: &str, registry: &str) -> Option<String> {
        let parsed = url::Url::parse(url).ok()?;
        let host =
            |url: &url::Url| url[url::Position::BeforeHost..url::Position::AfterPort].to_owned();
        let mut key = format!("//{}{}", host(&parsed), parsed.path());
        while key.len() > 2 {
            let field = |name: &str| self.config.get(&format!("{key}:{name}"));
            if let Some(token) = field("_authToken") {
                return Some(format!("Bearer {token}"));
            }
            if let Some(auth) = field("_auth") {
                return Some(format!("Basic {auth}"));
            }
            if let (Some(username), Some(password)) = (field("username"), field("_password")) {
                let mut credentials = format!("{username}:").into_bytes();
                credentials.extend(STANDARD.decode(password).ok()?);
                return Some(format!("Basic {}", STANDARD.encode(credentials)));
            }
            // Up one level: `//host/a/b/` to `//host/a/b`, then `//host/a/`.
            let parent = match key.strip_suffix('/') {
                Some(parent) => parent.len(),
                None => key.trim_end_matches(|c| c != '/').len(),
            };
            key.truncate(parent);
        }
        let same_host =
            url::Url::parse(registry).is_ok_and(|registry| host(&registry) == host(&parsed));
        if url != registry && same_host {
            self.authorization(registry, registry)
        } else {
            None
        }
    }

    /// GETs `url` of a package from `registry`, asking for the `accept`
    /// media types.
    async fn get(
        &self,
        url: &str,
        accept: &str,
        registry: &str,
    ) -> Result<reqwest::Response, NpmError> {
        let mut request = yapi_ai::http::client()
            .get(url)
            .header(reqwest::header::ACCEPT, accept);
        if let Some(authorization) = self.authorization(url, registry)
            && let Ok(mut value) = reqwest::header::HeaderValue::from_str(&authorization)
        {
            value.set_sensitive(true);
            request = request.header(reqwest::header::AUTHORIZATION, value);
        }
        // A request that never ends would hold the install, and the root's lock, for good.
        if let Some(timeout) = self.timeout() {
            request = request.timeout(timeout);
        }
        let response = request
            .send()
            .await
            .map_err(|err| request_error(url, &err))?;
        if !response.status().is_success() {
            return Err(NpmError::Registry(format!(
                "GET {url}: {}",
                response.status()
            )));
        }
        Ok(response)
    }

    async fn packument(&mut self, name: &str) -> Result<&Value, NpmError> {
        if !self.packuments.contains_key(name) {
            let registry = self.registry(name);
            let url = format!("{registry}{}", name.replace('/', "%2f"));
            let response = self.get(&url, PACKUMENT_ACCEPT, &registry).await?;
            let what = format!("The packument of {name}");
            let body = read_capped(response, &url, self.caps.packument, &what).await?;
            let packument: Value = serde_json::from_slice(&body)
                .map_err(|err| NpmError::Registry(format!("GET {url}: {err}")))?;
            self.packuments.insert(name.to_owned(), packument);
        }
        // Present: found or inserted above.
        Ok(&self.packuments[name])
    }

    /// The manifest of the version of `name` that `range` selects: a dist-tag,
    /// else the latest tag when it matches, else the highest match.
    async fn pick(&mut self, name: &str, range: &str) -> Result<Value, NpmError> {
        let packument = self.packument(name).await?;
        let no_versions = Map::new();
        let versions = packument["versions"].as_object().unwrap_or(&no_versions);
        let range = if range.is_empty() { "latest" } else { range };
        let no_match = || NpmError::NoMatch {
            name: name.to_owned(),
            range: range.to_owned(),
        };
        if let Some(tagged) = packument["dist-tags"][range].as_str() {
            return versions.get(tagged).cloned().ok_or_else(no_match);
        }
        if let Some(latest) = packument["dist-tags"]["latest"].as_str()
            && satisfies(latest, range)
            && let Some(manifest) = versions.get(latest)
        {
            return Ok(manifest.clone());
        }
        let versions = versions
            .iter()
            .map(|(version, manifest)| (version.as_str(), manifest));
        highest(versions, Some(range))
            .map(|(_, manifest)| manifest.clone())
            .ok_or_else(no_match)
    }

    /// The version pi's update checks compare with the installed one, as
    /// `npm view <name>@<range> version` reports it: the version a dist-tag
    /// names, else the highest version in the range.
    pub(crate) async fn latest(&mut self, name: &str, range: &str) -> Result<String, NpmError> {
        let packument = self.packument(name).await?;
        if let Some(tagged) = packument["dist-tags"][range].as_str() {
            return Ok(tagged.to_owned());
        }
        packument["versions"]
            .as_object()
            .and_then(|versions| {
                let versions = versions.keys().map(|version| (version.as_str(), ()));
                highest(versions, Some(range))
            })
            .map(|(version, ())| version.to_owned())
            .ok_or_else(|| NpmError::NoMatch {
                name: name.to_owned(),
                range: range.to_owned(),
            })
    }

    /// npm's `fetch-timeout`: how long a request may take, `None` (`0`) for
    /// no limit.
    fn timeout(&self) -> Option<std::time::Duration> {
        let millis = self.config.get("fetch-timeout");
        match millis.and_then(|millis| millis.trim().parse::<u64>().ok()) {
            Some(0) => None,
            Some(millis) => Some(std::time::Duration::from_millis(millis)),
            None => Some(FETCH_TIMEOUT),
        }
    }

    /// The tarball at `url` of package `name`, sent with the credentials of
    /// its registry. It is read in chunks, and the read stops at the cap.
    async fn download(&self, name: &str, url: &str) -> Result<Vec<u8>, NpmError> {
        let registry = self.registry(name);
        let response = self.get(url, "*/*", &registry).await?;
        let what = format!("The tarball of {name}");
        read_capped(response, url, self.caps.download, &what).await
    }

    async fn tarball(&self, manifest: &Value) -> Result<Vec<u8>, NpmError> {
        let id = format!("{}@{}", text(&manifest["name"]), text(&manifest["version"]));
        let url = manifest["dist"]["tarball"].as_str().unwrap_or_default();
        let bytes = self.download(&text(&manifest["name"]), url).await?;
        if verify(&bytes, &manifest["dist"]) {
            Ok(bytes)
        } else {
            Err(NpmError::Integrity(id))
        }
    }

    /// Fails unless `pin` is safe to download: a valid name, an integrity
    /// that is exactly one sha512 digest, and a tarball on the host and port
    /// of the registry configured for its name, over https unless that
    /// registry uses http. An edited lock can then neither send a download
    /// elsewhere nor credentials in cleartext. chest also checks each pin
    /// before it writes a lock, so that it never writes one replay refuses.
    pub(crate) fn check_pin(&self, pin: &Pinned) -> Result<(), NpmError> {
        check_name(&pin.name)?;
        let id = format!("{}@{}", pin.name, pin.version);
        if sha512_digest(&pin.integrity).is_none() {
            return Err(NpmError::Refused(format!(
                "Invalid integrity for {id}: expected one sha512 digest"
            )));
        }
        self.pinned_url(pin).map(drop)
    }

    /// Where to download `pin`'s tarball: its URL when that lies on the
    /// registry configured for its name, over https unless that registry
    /// uses http, at the standard `<name>/-/<unscoped name>-<version>.tgz`
    /// path for its name and version under the registry's own path. A
    /// tarball pinned on registry.npmjs.org
    /// comes from the configured registry when that differs, as npm's
    /// `replace-registry-host` does. Anything else is refused, so an edited
    /// lock can neither send a download elsewhere, nor credentials in
    /// cleartext, nor put another package's tarball in its place.
    fn pinned_url(&self, pin: &Pinned) -> Result<String, NpmError> {
        let id = format!("{}@{}", pin.name, pin.version);
        let refuse = |why: String| {
            NpmError::Refused(format!(
                "Refusing to download {id} from {}: {why}",
                pin.tarball
            ))
        };
        let registry = self.registry(&pin.name);
        let unscoped = pin.name.rsplit('/').next().unwrap_or(&pin.name);
        let file = format!("{}/-/{unscoped}-{}.tgz", pin.name, pin.version);
        let url = url::Url::parse(&pin.tarball).map_err(|_| refuse("not a URL".into()))?;
        let from =
            url::Url::parse(&registry).map_err(|_| refuse(format!("{registry} is not a URL")))?;
        let misplaced = |expected: String| {
            refuse(format!(
                "a pinned tarball must be at {expected}. Registries that keep tarballs elsewhere, such as GitHub Packages, cannot be replayed from a lock"
            ))
        };
        const NPMJS: Option<&str> = Some("registry.npmjs.org");
        if url.host_str() == NPMJS && from.host_str() != NPMJS {
            if url.path() != format!("/{file}") {
                return Err(misplaced(format!("/{file}")));
            }
            return Ok(format!("{registry}{file}"));
        }
        let scheme = url.scheme() == "https" || url.scheme() == "http" && from.scheme() == "http";
        let same = url.host_str() == from.host_str()
            && url.port_or_known_default() == from.port_or_known_default();
        if !scheme || !same {
            return Err(refuse(format!(
                "a pinned tarball must come from {registry}, the registry for {}, over https unless that registry uses http",
                pin.name
            )));
        }
        // Under the registry's own path, which may be a subpath.
        let expected = format!("{}{file}", from.path());
        if url.path() != expected {
            return Err(misplaced(expected));
        }
        Ok(pin.tarball.clone())
    }

    /// The tarball of `pin`, which [`Npm::check_pin`] passed, checked
    /// against its sha512 digest alone.
    async fn pinned_tarball(&self, pin: &Pinned) -> Result<Vec<u8>, NpmError> {
        let url = self.pinned_url(pin)?;
        let bytes = self.download(&pin.name, &url).await?;
        if sha512_digest(&pin.integrity).as_deref() == Some(digest(&SHA512, &bytes).as_ref()) {
            Ok(bytes)
        } else {
            Err(NpmError::Integrity(format!("{}@{}", pin.name, pin.version)))
        }
    }
}

/// The body of `response` for `url`, read in chunks and refused, as `what`,
/// once it passes `cap` bytes, which stops the read.
async fn read_capped(
    mut response: reqwest::Response,
    url: &str,
    cap: usize,
    what: &str,
) -> Result<Vec<u8>, NpmError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| request_error(url, &err))?
    {
        if bytes.len() + chunk.len() > cap {
            return Err(NpmError::Refused(format!(
                "{what} at {url} is over the download cap of {}",
                size(cap as u64)
            )));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

/// A failed request for `url`. A timeout is worded as npm words it, since
/// reqwest's message does not say so.
fn request_error(url: &str, err: &reqwest::Error) -> NpmError {
    if err.is_timeout() {
        NpmError::Registry(format!("network timeout at: {url}"))
    } else {
        NpmError::Registry(format!("GET {url}: {err}"))
    }
}

/// Checks `bytes` against the strongest algorithm of the registry's
/// `integrity` that yapi knows (sha512, then sha1), as npm's ssri does, and
/// against `shasum` when `integrity` has neither.
fn verify(bytes: &[u8], dist: &Value) -> bool {
    let integrity = dist["integrity"].as_str().unwrap_or_default();
    let hashes = |prefix: &str| -> Vec<&str> {
        integrity
            .split_whitespace()
            .filter_map(|entry| entry.strip_prefix(prefix))
            .collect()
    };
    let sha512 = hashes("sha512-");
    if !sha512.is_empty() {
        return sha512.contains(&STANDARD.encode(digest(&SHA512, bytes)).as_str());
    }
    let sha1 = digest(&SHA1, bytes);
    let sha1_hashes = hashes("sha1-");
    if !sha1_hashes.is_empty() {
        return sha1_hashes.contains(&STANDARD.encode(sha1).as_str());
    }
    dist["shasum"]
        .as_str()
        .is_some_and(|shasum| crate::time::hex(sha1.as_ref()).eq_ignore_ascii_case(shasum))
}

/// `inner`, failing once more than `left` bytes come out of it, which it
/// then records in `over`.
struct Capped<'a, R> {
    inner: R,
    left: u64,
    over: &'a std::cell::Cell<bool>,
}

impl<R: std::io::Read> std::io::Read for Capped<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        match self.left.checked_sub(read as u64) {
            Some(left) => {
                self.left = left;
                Ok(read)
            }
            None => {
                self.over.set(true);
                Err(std::io::Error::other("over the cap"))
            }
        }
    }
}

/// Unpacks the npm tarball of package `id` into `dir`, dropping each entry's
/// first path component (`package/`). Links and entries leaving `dir` are
/// skipped. Fails once the tarball unpacks to more than `caps` allow, with
/// what it unpacked left in `dir` for the caller to remove.
fn unpack(bytes: &[u8], dir: &Path, id: &str, caps: Caps) -> Result<(), NpmError> {
    let over = std::cell::Cell::new(false);
    let stream = Capped {
        inner: flate2::read::GzDecoder::new(bytes),
        left: caps.unpacked,
        over: &over,
    };
    // A read past the cap fails whatever step made it, long names included.
    unpack_entries(stream, dir, id, caps).map_err(|err| match over.get() {
        true => NpmError::Refused(format!(
            "The tarball of {id} unpacks to over the cap of {}",
            size(caps.unpacked)
        )),
        false => err,
    })
}

/// [`unpack`], from the tarball's decompressed `stream`.
fn unpack_entries(
    stream: impl std::io::Read,
    dir: &Path,
    id: &str,
    caps: Caps,
) -> Result<(), NpmError> {
    let mut archive = tar::Archive::new(stream);
    let entries = archive.entries().map_err(io(dir))?;
    for (count, entry) in entries.enumerate() {
        if count == caps.entries {
            return Err(NpmError::Refused(format!(
                "The tarball of {id} holds over the cap of {} entries",
                caps.entries
            )));
        }
        let mut entry = entry.map_err(io(dir))?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            continue;
        }
        let path = entry.path().map_err(io(dir))?.into_owned();
        let mut components = path.components();
        components.next();
        let relative: PathBuf = components.collect();
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            continue;
        }
        let target = dir.join(&relative);
        if kind.is_dir() {
            std::fs::create_dir_all(&target).map_err(io(&target))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(io(parent))?;
        }
        // Streamed: a small tarball can unpack to a file far larger than memory.
        let mut file = std::fs::File::create(&target).map_err(io(&target))?;
        std::io::copy(&mut entry, &mut file).map_err(io(&target))?;
        #[cfg(unix)]
        if let Ok(mode) = entry.header().mode() {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = std::fs::set_permissions(
                &target,
                std::fs::Permissions::from_mode(mode & 0o755 | 0o644),
            );
        }
    }
    Ok(())
}

/// The `package.json` of the package at `dir`, when it reads and parses. A
/// byte order mark is ignored, as npm ignores it.
fn read_manifest(dir: &Path) -> Option<Value> {
    yapi_types::json::parse(&std::fs::read_to_string(dir.join("package.json")).ok()?).ok()
}

/// The file in a package's folder while its install has not finished.
pub(crate) const INCOMPLETE: &str = ".yapi-incomplete";

/// The version in the `package.json` at `dir`.
fn manifest_version(dir: &Path) -> Option<String> {
    read_manifest(dir)?["version"].as_str().map(str::to_owned)
}

/// The version of the package installed at `dir`, if any. A package whose
/// install never finished, as when the process was stopped, is not installed.
pub(crate) fn installed_version(dir: &Path) -> Option<String> {
    if dir.join(INCOMPLETE).exists() {
        return None;
    }
    manifest_version(dir)
}

/// The `node_modules` folders in which Node looks for a dependency of the
/// package in `dependent`, nearest first, up to `root_modules`.
fn lookup_path<'a>(
    dependent: &'a Path,
    root_modules: &'a Path,
) -> impl Iterator<Item = PathBuf> + 'a {
    let ancestors = dependent.ancestors().filter(move |dir| {
        dir.file_name() == Some("node_modules".as_ref()) && dir.starts_with(root_modules)
    });
    std::iter::once(dependent.join("node_modules")).chain(ancestors.map(Path::to_path_buf))
}

/// The dependencies of `manifest` that [`Installer`] installs, in its order:
/// production ones not also optional, then optional ones, each with its
/// spec, `*` when that is not a string, and whether it is optional.
fn dependencies(manifest: &Value) -> impl Iterator<Item = (&str, &str, bool)> {
    fn listed<'a>(
        manifest: &'a Value,
        key: &'static str,
        optional: bool,
    ) -> impl Iterator<Item = (&'a str, &'a str, bool)> {
        let listed = manifest[key].as_object().into_iter().flatten();
        listed.map(move |(name, spec)| (name.as_str(), spec.as_str().unwrap_or("*"), optional))
    }
    let optional = &manifest["optionalDependencies"];
    listed(manifest, "dependencies", false)
        .filter(move |(name, ..)| optional.get(*name).is_none())
        .chain(listed(manifest, "optionalDependencies", true))
}

/// The package and range that dependency `name` with `spec` installs: the
/// target of an `npm:<package>@<range>` alias, which installs under `name`,
/// else `name` itself. `None` for what yapi never installs: host packages,
/// `file:` and `workspace:` specs.
fn wanted(name: &str, spec: &str) -> Option<(String, String)> {
    if HOST_PACKAGES.contains(&name) || spec.starts_with("file:") || spec.starts_with("workspace:")
    {
        return None;
    }
    Some(match spec.strip_prefix("npm:") {
        Some(spec) => {
            let (package, range) = super::source::split_npm_spec(spec);
            (package, range.unwrap_or_else(|| "*".into()))
        }
        None => (name.to_owned(), spec.to_owned()),
    })
}

/// Whether `version` of package `found` satisfies a dependency on `package`
/// at `range`. A dist-tag, which names no version, takes any version of the
/// package, as npm's arborist judges it, and an empty range is `latest`.
fn satisfied(found: &str, version: &str, package: &str, range: &str) -> bool {
    let tag = |range: &str| {
        range
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._".contains(&byte))
    };
    found == package
        && match range {
            "" => true,
            _ if is_range(range) => satisfies(version, range),
            _ => tag(range),
        }
}

/// Fails unless the tree `pins` records, unpacked in `staging` to stand for
/// `root/node_modules`, is what its dependents ask for, as npm keeps a tree.
/// Each package's own `package.json` must name its pin's name and version.
/// Each required dependency, of a pinned package or of `root/package.json`,
/// that reaches a pinned folder by Node's lookup must be satisfied by it,
/// and each pinned folder must be reached by a dependency it satisfies. An
/// optional dependency counts where it is satisfied and is otherwise
/// ignored, as yapi skips one that fails. The dependencies yapi never
/// installs are not checked: host packages, `file:` and `workspace:` specs,
/// and those for another platform, which reach no folder.
fn check_tree(root: &Path, staging: &Path, pins: &[&Pinned]) -> Result<(), NpmError> {
    let modules = Path::new("node_modules");
    // Pin paths start with `node_modules`, which `staging` stands for.
    let folder = |path: &Path| staging.join(path.strip_prefix(modules).unwrap_or(path));
    // Folded, as `replay` refused paths that differ by case alone, so that a
    // lookup a case-insensitive file system answers finds its pin.
    let index: HashMap<String, usize> = pins
        .iter()
        .enumerate()
        .map(|(index, pin)| (pin.path.to_ascii_lowercase(), index))
        .collect();
    let mut dependents = Vec::new();
    for pin in pins {
        let manifest = read_manifest(&folder(Path::new(&pin.path))).unwrap_or_default();
        let (name, version) = (text(&manifest["name"]), text(&manifest["version"]));
        // npm publishes a `v1.0.0` manifest as version `1.0.0`.
        if name != pin.name || version.trim().trim_start_matches(['=', 'v']) != pin.version {
            return Err(NpmError::Refused(format!(
                "The package.json in {} names {name}@{version}, not the pinned {}@{}",
                pin.path, pin.name, pin.version
            )));
        }
        dependents.push((PathBuf::from(&pin.path), manifest));
    }
    dependents.extend(read_manifest(root).map(|manifest| (PathBuf::new(), manifest)));
    let mut reached = vec![false; pins.len()];
    for (dependent, manifest) in &dependents {
        for (name, spec, optional) in dependencies(manifest) {
            // As `place` reads it. An invalid name installs nothing.
            let Some((package, range)) = check_name(name).ok().and_then(|()| wanted(name, spec))
            else {
                continue;
            };
            let Some(found) = lookup_path(dependent, modules)
                .map(|dir| dir.join(name))
                .find(|path| folder(path).is_dir())
            else {
                continue;
            };
            // A folder without a pin came in its parent's tarball.
            let found = found.to_string_lossy().replace('\\', "/");
            let Some(&index) = index.get(&found.to_ascii_lowercase()) else {
                continue;
            };
            let pin = pins[index];
            if satisfied(&pin.name, &pin.version, &package, &range) {
                reached[index] = true;
            } else if !optional {
                let by = match dependent.as_os_str().is_empty() {
                    true => "package.json".to_owned(),
                    false => dependent.to_string_lossy().replace('\\', "/"),
                };
                return Err(NpmError::Refused(format!(
                    "{} holds {}@{}, but {by} needs {package}@{range} there",
                    pin.path, pin.name, pin.version
                )));
            }
        }
    }
    if let Some((pin, _)) = pins.iter().zip(reached).find(|(_, reached)| !reached) {
        return Err(NpmError::Refused(format!(
            "{} holds {}@{}, which nothing depends on",
            pin.path, pin.name, pin.version
        )));
    }
    Ok(())
}

/// Installs into `root/node_modules`, hoisting dependencies there when Node's
/// resolution allows and nesting them otherwise.
struct Installer<'a> {
    npm: &'a mut Npm,
    root_modules: PathBuf,
    /// The packages whose dependencies are being installed, with their
    /// manifests. They count as installed so that cycles end.
    installing: Vec<(PathBuf, Value)>,
    /// A pin for each package installed so far.
    tree: Vec<Pinned>,
}

impl Installer<'_> {
    /// Unpacks `manifest`'s package into `dir`, replacing what is there, then
    /// installs its dependencies. When either fails, `dir` holds what it held
    /// before.
    fn unpack_package<'b>(
        &'b mut self,
        dir: PathBuf,
        manifest: Value,
    ) -> BoxFuture<'b, Result<(), NpmError>> {
        Box::pin(async move {
            let bytes = self.npm.tarball(&manifest).await?;
            let start = self.tree.len();
            // Every package folder is below `root_modules`: `install` and `place` put it there.
            let relative = dir.strip_prefix(&self.root_modules).unwrap_or(&dir);
            let path = format!(
                "node_modules/{}",
                relative.to_string_lossy().replace('\\', "/")
            );
            let pin = Pinned::new(path, &manifest, &bytes);
            // The old version waits beside `dir` while the new one installs in
            // place, where its dependencies find it and cycles end.
            let name = dir.file_name().unwrap_or_default().to_string_lossy();
            let old = dir.with_file_name(format!(".{name}-old"));
            let _ = std::fs::remove_dir_all(&old);
            let replacing = dir.exists();
            if replacing {
                std::fs::rename(&dir, &old).map_err(io(&dir))?;
            }
            let installed = async {
                std::fs::create_dir_all(&dir).map_err(io(&dir))?;
                // Marks the package until its dependencies are in, so an
                // install that never returns is redone.
                let marker = dir.join(INCOMPLETE);
                std::fs::write(&marker, "").map_err(io(&marker))?;
                let id = format!("{}@{}", text(&manifest["name"]), text(&manifest["version"]));
                unpack(&bytes, &dir, &id, self.npm.caps)?;
                self.installing.push((dir.clone(), manifest.clone()));
                let dependencies = self.dependencies(&dir, &manifest).await;
                self.installing.pop();
                dependencies?;
                std::fs::remove_file(&marker).map_err(io(&marker))
            }
            .await;
            if installed.is_ok() {
                let _ = std::fs::remove_dir_all(&old);
                self.tree.push(pin);
            } else {
                // So do the packages its install put elsewhere, such as at the
                // root, which only it needed, along with those nested in `dir`.
                for installed in self.tree.drain(start..) {
                    // Every pin's path starts so, as above.
                    if let Some(relative) = installed.path.strip_prefix("node_modules/") {
                        let _ = std::fs::remove_dir_all(self.root_modules.join(relative));
                    }
                }
                let _ = std::fs::remove_dir_all(&dir);
                if replacing {
                    let _ = std::fs::rename(&old, &dir);
                }
            }
            installed
        })
    }

    /// Installs the production and optional dependencies of the package in
    /// `dir`. Optional ones that fail or name a platform are skipped.
    async fn dependencies(&mut self, dir: &Path, manifest: &Value) -> Result<(), NpmError> {
        for (name, spec, optional) in dependencies(manifest) {
            let placed = self.place(dir, name, spec).await;
            if !optional {
                placed?;
            }
        }
        Ok(())
    }

    /// Makes dependency `name@range` of the package in `dependent` resolvable
    /// from it. The copy it finds first by Node's lookup is kept when it is
    /// the package at a version in the range, and a new copy is hoisted to
    /// the root when nothing is in the way, else nested in `dependent`. Either
    /// needs every package that would see the copy to accept it, so that none
    /// of them nests another over it later and shadows it for `dependent`.
    fn place<'b>(
        &'b mut self,
        dependent: &'b Path,
        name: &'b str,
        range: &'b str,
    ) -> BoxFuture<'b, Result<(), NpmError>> {
        Box::pin(async move {
            check_name(name)?;
            let Some((package, range)) = wanted(name, range) else {
                return Ok(());
            };
            let levels: Vec<PathBuf> = lookup_path(dependent, &self.root_modules).collect();
            let resolved = levels.iter().enumerate().find_map(|(level, modules)| {
                let dir = modules.join(name);
                let installing = self.installing.iter().any(|(path, _)| *path == dir);
                if !installing && dir.join(INCOMPLETE).exists() {
                    return None;
                }
                let manifest = read_manifest(&dir)?;
                let version = manifest["version"].as_str()?.to_owned();
                Some((level, text(&manifest["name"]), version))
            });
            if let Some((level, found, version)) = &resolved
                && satisfied(found, version, &package, &range)
                && self.accepted(&levels[..=*level], name, found, version)
            {
                return Ok(());
            }
            let manifest = self.npm.pick(&package, &range).await?;
            if manifest.get("os").is_some() || manifest.get("cpu").is_some() {
                // Platform packages carry prebuilt binaries, never JS.
                return Ok(());
            }
            let (found, version) = (text(&manifest["name"]), text(&manifest["version"]));
            let target = match resolved {
                None if self.accepted(&levels, name, &found, &version) => {
                    self.root_modules.join(name)
                }
                // Nested, it shadows the other version for `dependent` alone.
                _ => dependent.join("node_modules").join(name),
            };
            self.unpack_package(target, manifest).await
        })
    }

    /// Whether each package that owns one of the `node_modules` folders
    /// `levels` accepts `version` of package `found` as its dependency
    /// `name`, when it has one.
    fn accepted(&self, levels: &[PathBuf], name: &str, found: &str, version: &str) -> bool {
        levels.iter().all(|modules| {
            let Some(owner) = modules.parent() else {
                return true;
            };
            let read;
            let manifest = match self.installing.iter().find(|(dir, _)| dir == owner) {
                Some((_, manifest)) => manifest,
                None => match read_manifest(owner) {
                    Some(manifest) => {
                        read = manifest;
                        &read
                    }
                    None => return true,
                },
            };
            dependencies(manifest)
                .filter(|(dependency, ..)| *dependency == name)
                .all(|(dependency, spec, _)| {
                    wanted(dependency, spec)
                        .is_none_or(|(package, range)| satisfied(found, version, &package, &range))
                })
        })
    }
}

/// pi's `ensureGitIgnore`: creates `dir` with a `.gitignore` that keeps
/// everything else in it out of git.
pub(crate) fn ensure_git_ignore(dir: &Path) -> Result<(), NpmError> {
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, "*\n!.gitignore\n").map_err(io(&ignore))?;
    }
    Ok(())
}

/// The `package.json` of an npm root, created as pi creates it.
fn root_manifest(root: &Path) -> Result<Map<String, Value>, NpmError> {
    ensure_git_ignore(root)?;
    let manifest = read_manifest(root)
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_else(|| {
            let mut manifest = Map::new();
            manifest.insert("name".into(), json!("pi-extensions"));
            manifest.insert("private".into(), json!(true));
            manifest
        });
    Ok(manifest)
}

fn write_manifest(root: &Path, manifest: Map<String, Value>) -> Result<(), NpmError> {
    let path = root.join("package.json");
    let text =
        yapi_types::json::to_string_pretty(&Value::Object(manifest), "  ").unwrap_or_default();
    yapi_ai::auth::lock::write_atomic(&path, format!("{text}\n").as_bytes(), 0o666)
        .map_err(io(&path))
}

/// Locks the npm root `root`, creating it, against the installs and removals
/// of other processes until the lock is dropped. Waits as long as a live
/// holder keeps the lock, and takes over the lock of a holder that died.
pub(crate) async fn lock_root(root: &Path) -> Result<FileLock, NpmError> {
    std::fs::create_dir_all(root).map_err(io(root))?;
    lock(&root.join("package.json")).await
}

/// Locks `path`, whose folder exists, against other processes until the
/// lock is dropped, as [`lock_root`] does. The lock is a folder beside it.
pub(crate) async fn lock(path: &Path) -> Result<FileLock, NpmError> {
    loop {
        match FileLock::acquire(path, &CancellationToken::new()).await {
            // A lock still held after the stale period is fresh: its holder is alive.
            Err(LockError::Held) => continue,
            result => return result.map_err(|err| io(path)(std::io::Error::other(err))),
        }
    }
}

/// What [`install`] installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    /// The version of the requested package.
    pub version: String,
    /// A pin for each package the call unpacked, the requested one included,
    /// in no particular order. Packages it found already installed are not
    /// listed, so the tree is complete only when the root started empty: a
    /// tree recorded over a non-empty root must not be locked.
    pub tree: Vec<Pinned>,
}

/// Installs `name` (at `range`, else the latest version) into the npm root
/// `root`, and records it in the root's `package.json`. The caller holds the
/// root's `lock_root`.
pub async fn install(
    npm: &mut Npm,
    root: &Path,
    name: &str,
    range: Option<&str>,
) -> Result<Installed, NpmError> {
    check_name(name)?;
    let mut manifest_file = root_manifest(root)?;
    let manifest = npm.pick(name, range.unwrap_or("latest")).await?;
    let version = text(&manifest["version"]);
    let modules = root.join("node_modules");
    let mut installer = Installer {
        npm,
        root_modules: modules.clone(),
        installing: Vec::new(),
        tree: Vec::new(),
    };
    installer
        .unpack_package(modules.join(name), manifest)
        .await?;
    let tree = installer.tree;
    let dependencies = manifest_file
        .entry("dependencies")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(dependencies) = dependencies.as_object_mut() {
        let spec = range.map_or_else(|| format!("^{version}"), str::to_owned);
        dependencies.insert(name.to_owned(), Value::String(spec));
    }
    write_manifest(root, manifest_file)?;
    Ok(Installed { version, tree })
}

/// Installs the dependencies of the package in `dir` (a git checkout) into
/// its own `node_modules`, and returns a pin for each package it unpacked,
/// with paths below `dir`, as [`Installed::tree`] lists them. Fails when the
/// checkout links `node_modules` or a scope folder in it elsewhere.
pub async fn install_dependencies(npm: &mut Npm, dir: &Path) -> Result<Vec<Pinned>, NpmError> {
    let Some(manifest) = read_manifest(dir) else {
        return Ok(Vec::new());
    };
    let modules = dir.join("node_modules");
    let scopes = std::fs::read_dir(&modules)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with('@'))
        .map(|entry| entry.path());
    let is_link = |path: &PathBuf| {
        path.symlink_metadata()
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
    };
    if let Some(link) = std::iter::once(modules.clone()).chain(scopes).find(is_link) {
        return Err(NpmError::Refused(format!(
            "Refusing to install dependencies through the link {}",
            link.display()
        )));
    }
    let mut installer = Installer {
        npm,
        root_modules: modules,
        installing: Vec::new(),
        tree: Vec::new(),
    };
    installer.dependencies(dir, &manifest).await?;
    Ok(installer.tree)
}

/// Downloads `name` (at `range`, else the latest version), verifies its
/// tarball and unpacks it in place of `dir`, without its dependencies.
/// Returns its pin, whose `path` is empty: the package is `dir` itself.
pub async fn fetch(
    npm: &mut Npm,
    name: &str,
    range: Option<&str>,
    dir: &Path,
) -> Result<Pinned, NpmError> {
    check_name(name)?;
    let manifest = npm.pick(name, range.unwrap_or("latest")).await?;
    let bytes = npm.tarball(&manifest).await?;
    let staging = stage(dir)?;
    let id = format!("{name}@{}", text(&manifest["version"]));
    swap(&staging, dir, unpack(&bytes, &staging, &id, npm.caps))?;
    Ok(Pinned::new(String::new(), &manifest, &bytes))
}

/// Reinstalls the package `pin` records, as [`fetch`] returned it, in place
/// of `dir`, under the download rules of [`replay`]. Its `path` is ignored.
pub async fn fetch_pinned(npm: &Npm, pin: &Pinned, dir: &Path) -> Result<(), NpmError> {
    npm.check_pin(pin)?;
    let bytes = npm.pinned_tarball(pin).await?;
    let staging = stage(dir)?;
    let id = format!("{}@{}", pin.name, pin.version);
    swap(&staging, dir, unpack(&bytes, &staging, &id, npm.caps))
}

/// Fails unless the tree `pins` records, installed in `root/node_modules`
/// and declared by `root/package.json`, is one that [`replay`] accepts once
/// unpacked, so that a lock never pins a tree its replays refuse.
pub(crate) fn check_installed(root: &Path, pins: &[Pinned]) -> Result<(), NpmError> {
    let pins: Vec<&Pinned> = pins.iter().collect();
    check_tree(root, &root.join("node_modules"), &pins)
}

/// How many tarballs [`replay`] downloads at once.
const REPLAY_DOWNLOADS: usize = 8;

/// Replaces `root/node_modules` with exactly the tree `pins` records, as
/// [`Installed::tree`] lists it, without packuments or range resolution.
///
/// Before any request, it refuses a pin whose path is not
/// `node_modules/<name>` repeated, whose path collides with another's once
/// case is folded, whose parent package is not pinned, whose integrity is
/// not exactly one sha512 digest, or whose tarball URL is plain http while
/// its registry is not. It also refuses a `root` or `root/node_modules`
/// that is a link. Each tarball is downloaded with the credentials of its
/// name's registry and checked against its digest, eight at a time, and
/// unpacked only once it passes. Then each package's `package.json` must
/// name its pin, each required dependency, of the packages and of
/// `root/package.json`, that reaches a pinned folder must be satisfied by
/// it, and each pinned folder must be reached by a dependency it
/// satisfies, so that a lock can neither put one package in another's
/// place nor add one. A tree with a package at its top therefore needs a
/// `root/package.json` that depends on it. The tree is built beside
/// `root/node_modules` and swapped in only once complete, so a failure
/// leaves `root` as it was. The caller holds the root's `lock_root`.
pub async fn replay(npm: &Npm, root: &Path, pins: &[Pinned]) -> Result<(), NpmError> {
    let mut pins = pins
        .iter()
        .map(|pin| {
            npm.check_pin(pin)?;
            let names = pin_names(&pin.path)?;
            let parent = (names.len() > 1).then(|| {
                let outer = &names[..names.len() - 1];
                let parts: Vec<String> =
                    outer.iter().map(|n| format!("node_modules/{n}")).collect();
                parts.join("/")
            });
            Ok((pin.path.to_ascii_lowercase(), parent, pin))
        })
        .collect::<Result<Vec<_>, NpmError>>()?;
    // Parents first, so that nested packages unpack into their parent's folder.
    pins.sort_by(|a, b| a.0.cmp(&b.0));
    if let Some(pair) = pins.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(NpmError::Refused(format!(
            "Package paths collide: {} and {}",
            pair[0].2.path, pair[1].2.path
        )));
    }
    let paths: HashSet<&str> = pins.iter().map(|(_, _, pin)| pin.path.as_str()).collect();
    if let Some((_, Some(parent), pin)) = pins.iter().find(|(_, parent, _)| {
        parent
            .as_deref()
            .is_some_and(|parent| !paths.contains(parent))
    }) {
        return Err(NpmError::Refused(format!(
            "Package path {} has no pinned parent {parent}",
            pin.path
        )));
    }
    if is_link(root) {
        return Err(NpmError::Refused(format!(
            "Refusing to install through the link {}",
            root.display()
        )));
    }
    let modules = root.join("node_modules");
    let staging = stage(&modules)?;
    let built = async {
        // Downloads overlap, and each tarball unpacks in order once checked.
        // The futures are built first: a closure the stream held across
        // these awaits would keep the caller's future from being `Send`.
        let downloads: Vec<_> = pins
            .iter()
            .map(|(_, _, pin)| {
                let pin: &Pinned = pin;
                async move { (pin, npm.pinned_tarball(pin).await) }
            })
            .collect();
        let mut downloads = futures_util::stream::iter(downloads).buffered(REPLAY_DOWNLOADS);
        while let Some((pin, bytes)) = downloads.next().await {
            let bytes = bytes?;
            // `pin_names` checked that the path starts so, and `staging` stands for it.
            let relative = pin.path.strip_prefix("node_modules/").unwrap_or(&pin.path);
            let dir = staging.join(relative);
            // A bundled copy in the parent's tarball gives way to the pinned one.
            remove_path(&dir).map_err(io(&dir))?;
            std::fs::create_dir_all(&dir).map_err(io(&dir))?;
            let id = format!("{}@{}", pin.name, pin.version);
            unpack(&bytes, &dir, &id, npm.caps)?;
        }
        let pins: Vec<&Pinned> = pins.iter().map(|(_, _, pin)| *pin).collect();
        check_tree(root, &staging, &pins)
    }
    .await;
    swap(&staging, &modules, built)
}

/// Removes `name` from the npm root `root`, then the packages nothing
/// depends on any more, `name` included unless another package needs it.
pub async fn uninstall(root: &Path, name: &str) -> Result<(), NpmError> {
    if !root.exists() {
        return Ok(());
    }
    let _lock = lock_root(root).await?;
    let mut manifest = root_manifest(root)?;
    if let Some(dependencies) = manifest
        .get_mut("dependencies")
        .and_then(Value::as_object_mut)
    {
        dependencies.shift_remove(name);
    }
    let roots: Vec<String> = manifest
        .get("dependencies")
        .and_then(Value::as_object)
        .map(|dependencies| dependencies.keys().cloned().collect())
        .unwrap_or_default();
    write_manifest(root, manifest)?;
    prune(&root.join("node_modules"), &roots);
    Ok(())
}

/// Removes top-level packages in `modules` that no package in `roots`
/// reaches through Node's resolution.
fn prune(modules: &Path, roots: &[String]) {
    let mut reached: Vec<PathBuf> = Vec::new();
    let mut queue: Vec<PathBuf> = roots.iter().map(|name| modules.join(name)).collect();
    while let Some(dir) = queue.pop() {
        if reached.contains(&dir) || !dir.exists() {
            continue;
        }
        reached.push(dir.clone());
        let manifest = read_manifest(&dir).unwrap_or(Value::Null);
        for key in ["dependencies", "optionalDependencies"] {
            for name in manifest[key]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(name, _)| name)
            {
                let resolved = lookup_path(&dir, modules)
                    .map(|modules| modules.join(name))
                    .find(|path| path.exists());
                queue.extend(resolved);
            }
        }
    }
    let Ok(entries) = std::fs::read_dir(modules) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let candidates: Vec<PathBuf> = if name.starts_with('@') {
            std::fs::read_dir(&path)
                .map(|scoped| scoped.flatten().map(|entry| entry.path()).collect())
                .unwrap_or_default()
        } else if name.starts_with('.') {
            Vec::new()
        } else {
            vec![path]
        };
        for candidate in candidates {
            if !reached.contains(&candidate) {
                let _ = std::fs::remove_dir_all(&candidate);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_like_semver() {
        assert_eq!(newer("1.2.0", "1.0.0"), Some(true));
        assert_eq!(newer(" 1.0.0\n", "1.0.0"), Some(false));
        assert_eq!(newer("1.0.0-rc.1", "1.0.0"), Some(false));
        // `gt` throws, and pi's update goes ahead while its check reports none.
        assert_eq!(newer("1.0.0", "next"), None);
    }

    #[test]
    fn reads_npm_view_output() {
        assert_eq!(
            viewed_version("\"2.0.0\"\n", Some("^1")).as_deref(),
            Some("2.0.0")
        );
        let several = r#"["1.0.0", "1.5.0", "2.0.0", ""]"#;
        assert_eq!(
            viewed_version(several, Some("^1")).as_deref(),
            Some("1.5.0")
        );
        assert_eq!(viewed_version(several, None).as_deref(), Some("2.0.0"));
        assert_eq!(viewed_version(several, Some("^3")), None);
        assert_eq!(viewed_version("", None), None);
    }

    #[test]
    fn matches_npm_ranges() {
        assert!(satisfies("1.4.0", "^1.2.3"));
        assert!(!satisfies("2.0.0", "^1.2.3"));
        assert!(satisfies("1.2.9", "~1.2.0"));
        assert!(satisfies("1.5.0", ">=1.2 <2"));
        assert!(satisfies("3.0.0", "^1 || ^3"));
        assert!(satisfies("1.9.9", "1.x"));
        assert!(satisfies("0.1.0", "*"));
        assert!(satisfies("1.3.0", "1.2.0 - 1.4.0"));
        assert!(!satisfies("1.0.0-beta.1", "^1.0.0"));
        // A bare version is exact, and a partial one fixes what it names.
        assert!(satisfies("0.35.0", "0.35.0"));
        assert!(!satisfies("0.35.1", "0.35.0"));
        assert!(!satisfies("1.3.0", "1.2.3"));
        assert!(satisfies("1.2.9", "1.2"));
        assert!(!satisfies("1.3.0", "1.2"));
        assert!(satisfies("1.9.0", "1"));
        assert!(!satisfies("2.0.0", "1"));
        assert!(satisfies("1.2.3", "1.2.3 || 2"));
        assert!(!satisfies("1.2.4", "1.2.3 || 2"));
        assert!(satisfies("1.2.9", "1.2.x"));
        assert!(!satisfies("1.3.0", "1.2.x"));
    }

    #[test]
    fn npmrc_environment_references() {
        let env = |name: &str| (name == "TOKEN").then(|| "secret".to_owned());
        let config = parse_config(
            "; comment\n\
             # comment\n\
             registry = https://registry.example/ \n\
             @acme:registry=\"https://npm.acme.test/\"\n\
             //npm.acme.test/:_authToken=${TOKEN}\n\
             //other.test/:_authToken=${MISSING}\n\
             //optional.test/:_authToken=${MISSING?}\n\
             //token.test/:_auth=dXNlcjpwYXNz ; comment\n\
             [section]\n\
             ignored=1\n",
            env,
        );
        let mut entries: Vec<_> = config.iter().collect();
        entries.sort();
        assert_eq!(
            entries,
            [
                (
                    &"//npm.acme.test/:_authToken".to_owned(),
                    &"secret".to_owned()
                ),
                (&"//other.test/:_authToken".into(), &"${MISSING}".into()),
                (&"//token.test/:_auth".into(), &"dXNlcjpwYXNz".into()),
                (&"@acme:registry".into(), &"https://npm.acme.test/".into()),
                (&"registry".into(), &"https://registry.example/".into()),
            ]
        );
        assert_eq!(
            expand("a${TOKEN}b${${TOKEN}}${}", env),
            "asecretb${secret}${}"
        );
    }

    #[test]
    fn credentials_scoped_to_url() {
        let npm = Npm::new(HashMap::from(
            [
                ("@acme:registry", "https://npm.acme.test"),
                ("//npm.acme.test/:_authToken", "acme"),
                ("//npm.acme.test/private/:_authToken", "private"),
                ("//basic.test/:username", "user"),
                ("//basic.test/:_password", "cGFzcw=="),
                ("//legacy.test:8080/:_auth", "dXNlcjpwYXNz"),
                ("//gitlab.test/api/v4/packages/npm/:_authToken", "gitlab"),
            ]
            .map(|(key, value)| (key.to_owned(), value.to_owned())),
        ));
        assert_eq!(npm.registry("@acme/pkg"), "https://npm.acme.test/");
        assert_eq!(npm.registry("pkg"), "https://registry.npmjs.org/");
        let auth = |url: &str, registry: &str| npm.authorization(url, registry);
        let acme = "https://npm.acme.test/";
        assert_eq!(
            auth("https://npm.acme.test/@acme%2fpkg", acme).unwrap(),
            "Bearer acme"
        );
        assert_eq!(
            auth("https://npm.acme.test/private/pkg.tgz", acme).unwrap(),
            "Bearer private"
        );
        assert_eq!(
            auth("https://npm.acme.test/privateer/pkg.tgz", acme).unwrap(),
            "Bearer acme"
        );
        assert_eq!(
            auth("https://basic.test/pkg", "https://basic.test/").unwrap(),
            format!("Basic {}", STANDARD.encode("user:pass"))
        );
        assert_eq!(
            auth("http://legacy.test:8080/pkg", acme).unwrap(),
            "Basic dXNlcjpwYXNz"
        );
        // A tarball elsewhere on the registry's host gets the registry's.
        let gitlab = "https://gitlab.test/api/v4/packages/npm/";
        assert_eq!(
            auth(
                "https://gitlab.test/api/v4/projects/7/packages/npm/pkg.tgz",
                gitlab
            )
            .unwrap(),
            "Bearer gitlab"
        );
        // Other hosts and ports get nothing, tarballs included.
        assert_eq!(auth("http://legacy.test/pkg", acme), None);
        assert_eq!(auth("https://npm.acme.test.evil/pkg", acme), None);
        assert_eq!(auth("https://cdn.acme.test/pkg.tgz", acme), None);
        assert_eq!(
            auth(
                "https://registry.npmjs.org/pkg",
                "https://registry.npmjs.org/"
            ),
            None
        );
    }

    #[tokio::test]
    async fn registry_timeout() {
        assert_eq!(Npm::new(HashMap::new()).timeout(), Some(FETCH_TIMEOUT));
        let unlimited = HashMap::from([("fetch-timeout".to_owned(), "0".to_owned())]);
        assert_eq!(Npm::new(unlimited).timeout(), None);
        // A registry that takes the connection and never answers.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let registry = format!("http://{}/", listener.local_addr().unwrap());
        let mut npm = Npm::new(HashMap::from([
            ("registry".to_owned(), registry.clone()),
            ("fetch-timeout".to_owned(), "200".to_owned()),
        ]));
        let pick = npm.pick("pkg", "latest");
        let error = tokio::time::timeout(std::time::Duration::from_secs(10), pick)
            .await
            .expect("the request never timed out")
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("network timeout at: {registry}pkg")
        );
    }

    #[test]
    fn reads_manifests_with_bom() {
        let dir = std::env::temp_dir().join(format!("yapi-npm-bom-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package.json"), "\u{feff}{\"version\": \"1.2.3\"}").unwrap();
        assert_eq!(installed_version(&dir).as_deref(), Some("1.2.3"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Entries that climb out of the package, absolute paths and links stay
    /// out of the folder, and nothing is written beside it. Files keep their
    /// bytes and executable bits, however large.
    #[test]
    fn unpack_stays_inside() {
        use tar::EntryType;
        let mut builder = tar::Builder::new(Vec::new());
        let mut add = |name: &str, kind: EntryType, link: &str, data: &[u8], mode: u32| {
            // Raw names: `set_path` refuses `..`.
            let mut header = tar::Header::new_gnu();
            let raw = header.as_old_mut();
            raw.name[..name.len()].copy_from_slice(name.as_bytes());
            raw.linkname[..link.len()].copy_from_slice(link.as_bytes());
            header.set_entry_type(kind);
            header.set_size(data.len() as u64);
            header.set_mode(mode);
            header.set_cksum();
            builder.append(&header, data).unwrap();
        };
        // Larger than the buffers a streamed copy goes through.
        let large: Vec<u8> = (0..300_000u32).map(|n| (n % 251) as u8).collect();
        add("package/index.js", EntryType::Regular, "", b"ok", 0o644);
        add("package/run", EntryType::Regular, "", &large, 0o755);
        add("package/../evil", EntryType::Regular, "", b"evil", 0o644);
        add("/abs/evil", EntryType::Regular, "", b"abs", 0o644);
        add("package/link", EntryType::Symlink, "/etc", b"", 0o644);
        add(
            "package/hard",
            EntryType::Link,
            "package/index.js",
            b"",
            0o644,
        );
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut gz, &builder.into_inner().unwrap()).unwrap();
        let bytes = gz.finish().unwrap();

        let parent = std::env::temp_dir().join(format!("yapi-npm-unpack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        let dir = parent.join("package");
        std::fs::create_dir_all(&dir).unwrap();
        unpack(&bytes, &dir, "pkg@1.0.0", CAPS).unwrap();
        let names = |dir: &Path| {
            let mut names: Vec<String> = std::fs::read_dir(dir)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        };
        assert_eq!(names(&parent), ["package"]);
        // An absolute path loses its root with the first component.
        assert_eq!(names(&dir), ["abs", "index.js", "run"]);
        assert_eq!(std::fs::read(dir.join("index.js")).unwrap(), b"ok");
        assert_eq!(std::fs::read(dir.join("run")).unwrap(), large);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = |name: &str| {
                let metadata = std::fs::metadata(dir.join(name)).unwrap();
                metadata.permissions().mode() & 0o777
            };
            assert_eq!((mode("index.js"), mode("run")), (0o644, 0o755));
        }
        std::fs::remove_dir_all(&parent).unwrap();
    }

    /// A tarball over any of the caps fails its replay, which then leaves
    /// the root as it was: one too large to download, one that unpacks to
    /// too much, and one with too many entries. A packument too large to
    /// download fails before it is parsed.
    #[tokio::test(flavor = "multi_thread")]
    async fn capped_tarballs_leave_nothing() {
        // Noise, so that the gzip stream stays as large as what it holds.
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let noise: Vec<u8> = (0..2 << 20)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state.to_le_bytes()[0]
            })
            .collect();
        let mut builder = tar::Builder::new(Vec::new());
        for (name, data) in [
            (
                "package/package.json",
                &br#"{"name": "app", "version": "1.0.0"}"#[..],
            ),
            ("package/noise", &noise),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, name, data).unwrap();
        }
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gz, &builder.into_inner().unwrap()).unwrap();
        let body = gz.finish().unwrap();
        assert!(body.len() > 1 << 20);
        let sha512 = STANDARD.encode(digest(&SHA512, &body));
        // Serves `body` at every path.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            use std::io::{BufRead as _, Write as _};
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap_or(0) > 2 {
                    line.clear();
                }
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        let pin = Pinned {
            path: "node_modules/app".into(),
            name: "app".into(),
            version: "1.0.0".into(),
            tarball: format!("{base}/app/-/app-1.0.0.tgz"),
            integrity: format!("sha512-{sha512}"),
        };
        let root = std::env::temp_dir().join(format!("yapi-npm-caps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("node_modules/kept")).unwrap();
        let npm = Npm::new(HashMap::from([("registry".to_owned(), format!("{base}/"))]));
        let lowered = [
            (
                Caps {
                    download: 1 << 20,
                    ..CAPS
                },
                format!(
                    "The tarball of app at {} is over the download cap of 1 MiB",
                    pin.tarball
                ),
            ),
            (
                Caps {
                    unpacked: 1 << 20,
                    ..CAPS
                },
                "The tarball of app@1.0.0 unpacks to over the cap of 1 MiB".to_owned(),
            ),
            (
                Caps { entries: 1, ..CAPS },
                "The tarball of app@1.0.0 holds over the cap of 1 entries".to_owned(),
            ),
        ];
        for (caps, message) in lowered {
            let npm = Npm {
                caps,
                ..npm.clone()
            };
            let error = replay(&npm, &root, std::slice::from_ref(&pin))
                .await
                .unwrap_err();
            assert_eq!(error.to_string(), message);
            let names: Vec<_> = std::fs::read_dir(&root)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            assert_eq!(names, ["node_modules"]);
            assert!(root.join("node_modules/kept").exists());
        }
        let mut npm = Npm {
            caps: Caps {
                packument: 1 << 20,
                ..CAPS
            },
            ..npm
        };
        let error = npm.pick("app", "latest").await.unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("The packument of app at {base}/app is over the download cap of 1 MiB")
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// When the new tree cannot take its place and the old one cannot go
    /// back, the old one stays where the error says.
    #[test]
    fn swap_keeps_old_tree() {
        let parent = std::env::temp_dir().join(format!("yapi-npm-swap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        let (dir, staging, old) = (
            parent.join("node_modules"),
            parent.join(".node_modules-staging"),
            parent.join(".node_modules-old"),
        );
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::create_dir_all(staging.join("b")).unwrap();
        let rename = |from: &Path, to: &Path| {
            if from == staging || from == old {
                Err(std::io::Error::other("refused"))
            } else {
                std::fs::rename(from, to)
            }
        };
        let error = swap_with(&staging, &dir, rename).unwrap_err();
        assert!(
            error
                .to_string()
                .ends_with(&format!("the previous version is at {}", old.display())),
            "{error}"
        );
        assert!(old.join("a").exists());
        // A plain failure puts the old tree back.
        std::fs::rename(&old, &dir).unwrap();
        std::fs::create_dir_all(staging.join("b")).unwrap();
        let fail_new = |from: &Path, to: &Path| {
            if from == staging {
                Err(std::io::Error::other("refused"))
            } else {
                std::fs::rename(from, to)
            }
        };
        assert!(swap_with(&staging, &dir, fail_new).is_err());
        assert!(dir.join("a").exists() && !old.exists() && !staging.exists());
        std::fs::remove_dir_all(&parent).unwrap();
    }

    #[test]
    fn ranges_allow_v_prefix() {
        assert!(satisfies("1.2.3", "v1.2.3"));
        assert!(!satisfies("1.2.4", "v1.2.3"));
        assert!(satisfies("1.4.0", "^v1.2.3"));
        assert!(satisfies("1.2.9", "~v1.2.0"));
        assert!(satisfies("1.5.0", ">=v1.2 <v2"));
        assert!(satisfies("1.5.0", ">= v1.2 < v2"));
        assert!(satisfies("1.3.0", "v1.2.0 - v1.4.0"));
        assert!(satisfies("2.0.0", "v1 || v2.x"));
    }

    #[test]
    fn verifies_strongest_integrity() {
        let bytes = b"tarball";
        let sha512 = format!("sha512-{}", STANDARD.encode(digest(&SHA512, bytes)));
        let sha1 = format!("sha1-{}", STANDARD.encode(digest(&SHA1, bytes)));
        let shasum = crate::time::hex(digest(&SHA1, bytes).as_ref());
        let wrong512 = format!("sha512-{}", STANDARD.encode(digest(&SHA512, b"other")));
        let check = |dist: Value| verify(bytes, &dist);
        assert!(check(json!({"integrity": sha512})));
        assert!(check(json!({"integrity": sha1})));
        assert!(!check(json!({"integrity": "sha1-AAAA"})));
        // sha512 decides when present, as the strongest algorithm.
        assert!(!check(json!({"integrity": format!("{wrong512} {sha1}")})));
        assert!(check(json!({"integrity": format!("{wrong512} {sha512}")})));
        // An integrity without a known algorithm falls back to `shasum`.
        assert!(check(json!({"integrity": "sha384-AAAA", "shasum": shasum})));
        assert!(check(json!({"shasum": shasum.to_uppercase()})));
        assert!(!check(json!({"integrity": "sha384-AAAA"})));
        assert!(!check(json!({})));
    }
}
