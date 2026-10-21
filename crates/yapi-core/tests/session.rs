//! Session files against pi's fixtures and pi's context output.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::path::{Path, PathBuf};

use serde_json::Value;
use yapi_core::session::{SessionManager, list};
use yapi_types::message::{Content, Message, UserMessage};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pi")
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("yapi-session-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The value as JavaScript sees it: numbers re-read from their JS text form.
fn js(value: &Value) -> Value {
    serde_json::from_str(&yapi_types::json::to_string(value).unwrap()).unwrap()
}

#[test]
fn contexts_match_pi() {
    let scratch = Scratch::new("contexts");
    for entry in std::fs::read_dir(fixtures().join("sessions")).unwrap() {
        let source = entry.unwrap().path();
        let name = source.file_stem().unwrap().to_string_lossy().into_owned();
        // Opening may append a final newline; work on a copy.
        let copy = scratch.0.join(source.file_name().unwrap());
        std::fs::copy(&source, &copy).unwrap();
        let session = SessionManager::open(&copy, None, Some(Path::new("/tmp"))).unwrap();
        let context = session.build_context();
        let expected: Value = serde_json::from_str(
            &std::fs::read_to_string(fixtures().join("contexts").join(format!("{name}.json")))
                .unwrap(),
        )
        .unwrap();
        let actual = serde_json::json!({
            "messages": serde_json::to_value(&context.messages).unwrap(),
            "thinkingLevel": context.thinking_level,
            "model": context.model.as_ref().map(|(provider, model_id)| serde_json::json!({"provider": provider, "modelId": model_id})),
        });
        assert_eq!(js(&actual), expected, "context of {name}");
        // Reading never rewrites a current-version file.
        assert_eq!(
            std::fs::read(&copy).unwrap(),
            std::fs::read(&source).unwrap(),
            "{name} changed on open"
        );
    }
}

#[test]
fn migrates_v1_files() {
    let scratch = Scratch::new("migrate");
    let copy = scratch.0.join("old.jsonl");
    std::fs::copy(fixtures().join("legacy/large-session.v1.jsonl"), &copy).unwrap();
    let session = SessionManager::open(&copy, None, None).unwrap();
    let text = std::fs::read_to_string(&copy).unwrap();
    let header: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(header["version"], 3);
    // Every entry got an id and a parent chain, in file order.
    let mut previous = Value::Null;
    for line in text.lines().skip(1) {
        let entry: Value = serde_json::from_str(line).unwrap();
        assert_eq!(entry["id"].as_str().unwrap().len(), 8);
        assert_eq!(entry["parentId"], previous);
        previous = entry["id"].clone();
    }
    // The migrated context matches pi's migration of the same excerpt.
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("contexts/legacy-large-session.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        js(&serde_json::to_value(&session.build_context().messages).unwrap()),
        expected["messages"]
    );
}

fn user(text: &str) -> Message {
    Message::User(UserMessage {
        content: Content::Text(text.into()),
        timestamp: 1,
    })
}

#[test]
fn lazy_file_and_branch() {
    let scratch = Scratch::new("lazy");
    let mut session = SessionManager::create(Path::new("/work"), &scratch.0, None).unwrap();
    let model = session.append_model_change("anthropic", "m").unwrap();
    let file = session.file().unwrap().to_path_buf();
    assert!(!file.exists(), "no file before a conversation");
    let first = session.append_message(user("one")).unwrap();
    assert!(file.exists());
    session.append_message(user("two")).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap().lines().count(), 4);

    session.branch(&first).unwrap();
    session.append_message(user("three")).unwrap();
    let texts: Vec<String> = session
        .build_context()
        .messages
        .iter()
        .map(|message| match message {
            Message::User(user) => user.content.text(""),
            _ => String::new(),
        })
        .collect();
    assert_eq!(texts, ["one", "three"]);

    // Listing reads a file whose last line lost its newline without
    // repairing it, as pi's listing does. Opening it repairs it.
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, text.trim_end()).unwrap();
    assert_eq!(list(&scratch.0, None)[0].first_message, "one");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text.trim_end());

    // A file written by yapi reopens with the same tree.
    let reopened = SessionManager::open(&file, None, None).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
    assert_eq!(reopened.build_context(), session.build_context());
    assert_eq!(reopened.id(), session.id());

    // Branching off into a new file keeps only the path, with its labels in
    // pi's order: a relabelled entry keeps its place.
    session.append_label(&first, Some("a".into())).unwrap();
    session.append_label(&model, Some("b".into())).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    session.append_label(&first, Some("c".into())).unwrap();
    let branched = session.create_branched_session(&first).unwrap().unwrap();
    let lines: Vec<Value> = std::fs::read_to_string(&branched)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let types: Vec<&str> = lines
        .iter()
        .map(|line| line["type"].as_str().unwrap())
        .collect();
    assert_eq!(
        types,
        ["session", "model_change", "message", "label", "label"]
    );
    let labels: Vec<(&str, &str)> = lines[3..]
        .iter()
        .map(|line| {
            (
                line["targetId"].as_str().unwrap(),
                line["label"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(labels, [(first.as_str(), "c"), (model.as_str(), "b")]);
}

fn user_texts(session: &SessionManager) -> Vec<String> {
    session
        .build_context()
        .messages
        .iter()
        .map(|message| match message {
            Message::User(user) => user.content.text(""),
            _ => String::new(),
        })
        .collect()
}

#[test]
fn failed_writes_keep_chain() {
    let scratch = Scratch::new("failed-write");
    let mut session = SessionManager::create(Path::new("/work"), &scratch.0, None).unwrap();
    session.append_message(user("one")).unwrap();
    let file = session.file().unwrap().to_path_buf();
    let saved = std::fs::read(&file).unwrap();

    // A directory in the file's place fails every write.
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    assert!(session.append_message(user("two")).is_err());
    assert!(session.append_message(user("three")).is_err());
    assert!(session.take_write_error().is_some());
    assert_eq!(session.take_write_error(), None, "reported once");
    assert_eq!(user_texts(&session), ["one", "two", "three"]);

    // The next write that succeeds appends the entries the failed ones left out.
    std::fs::remove_dir(&file).unwrap();
    std::fs::write(&file, saved).unwrap();
    session.append_message(user("four")).unwrap();
    let reopened = SessionManager::open(&file, None, None).unwrap();
    assert_eq!(user_texts(&reopened), ["one", "two", "three", "four"]);
    assert_eq!(reopened.build_context(), session.build_context());
}

/// pi writes a lone surrogate escape for text cut inside a surrogate pair.
#[test]
fn lone_surrogates_load() {
    let scratch = Scratch::new("surrogates");
    let mut session = SessionManager::create(Path::new("/work"), &scratch.0, None).unwrap();
    session.append_message(user("one")).unwrap();
    session.append_message(user("cut MARK")).unwrap();
    session.append_message(user("three")).unwrap();
    let file = session.file().unwrap().to_path_buf();
    let text = std::fs::read_to_string(&file).unwrap();
    // A lone high and a lone low surrogate, a pair, and an escaped backslash.
    let cut = r"cut \ud83d \ude00 😀 \\ud83d";
    std::fs::write(&file, text.replace("cut MARK", cut)).unwrap();

    let reopened = SessionManager::open(&file, None, None).unwrap();
    assert_eq!(
        user_texts(&reopened),
        ["one", "cut \u{fffd} \u{fffd} \u{1f600} \\ud83d", "three"]
    );
}
