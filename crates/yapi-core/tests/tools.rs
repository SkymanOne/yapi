//! Built-in tools on a scratch directory.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use yapi_agent::UpdateSink;
use yapi_core::tools::{ToolEnv, builtin};
use yapi_types::event::ToolResult;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("yapi-tools-{name}-{}", std::process::id()));
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

fn env(cwd: &Path) -> ToolEnv {
    ToolEnv {
        cwd: cwd.to_path_buf(),
        runtime: Arc::default(),
        bin_dir: cwd.join("bin"),
    }
}

fn sink() -> (UpdateSink, Arc<Mutex<Vec<ToolResult>>>) {
    let updates = Arc::new(Mutex::new(Vec::new()));
    let store = updates.clone();
    (
        Arc::new(move |update| store.lock().unwrap().push(update)),
        updates,
    )
}

async fn call(cwd: &Path, name: &str, args: Value) -> Result<ToolResult, String> {
    let tool = builtin(name, &env(cwd)).unwrap().tool;
    let (updates, _) = sink();
    tool.execute("id".into(), args, CancellationToken::new(), updates)
        .await
}

fn text(result: &ToolResult) -> String {
    yapi_types::message::blocks_text(&result.content, "\n")
}

#[tokio::test]
async fn read_pages_through_files() {
    let dir = Scratch::new("read");
    let lines: Vec<String> = (1..=5).map(|n| format!("line {n}")).collect();
    std::fs::write(dir.0.join("a.txt"), lines.join("\n")).unwrap();

    let result = call(
        &dir.0,
        "read",
        json!({"path": "a.txt", "offset": 2, "limit": 2}),
    )
    .await
    .unwrap();
    assert_eq!(
        text(&result),
        "line 2\nline 3\n\n[2 more lines in file. Use offset=4 to continue.]"
    );
    let err = call(&dir.0, "read", json!({"path": "a.txt", "offset": 9}))
        .await
        .unwrap_err();
    assert_eq!(err, "Offset 9 is beyond end of file (5 lines total)");
    let err = call(&dir.0, "read", json!({"path": "missing.txt"}))
        .await
        .unwrap_err();
    assert!(err.starts_with("ENOENT: no such file or directory, access '"));

    let long: Vec<String> = (1..=2500).map(|n| n.to_string()).collect();
    std::fs::write(dir.0.join("long.txt"), long.join("\n")).unwrap();
    let result = call(&dir.0, "read", json!({"path": "@long.txt"}))
        .await
        .unwrap();
    assert!(
        text(&result).ends_with("\n\n[Showing lines 1-2000 of 2500. Use offset=2001 to continue.]")
    );
    assert_eq!(
        result.details.unwrap()["truncation"]["truncatedBy"],
        "lines"
    );
}

#[tokio::test]
async fn read_resizes_images() {
    use image::{DynamicImage, ImageOutputFormat, RgbaImage};
    let dir = Scratch::new("read-images");
    let write = |name: &str, width: u32, format: ImageOutputFormat| {
        let image = DynamicImage::ImageRgba8(RgbaImage::new(width, 4));
        image
            .write_to(
                &mut std::fs::File::create(dir.0.join(name)).unwrap(),
                format,
            )
            .unwrap();
    };
    write("small.bmp", 3, ImageOutputFormat::Bmp);
    write("wide.png", 2100, ImageOutputFormat::Png);
    let bmp = std::fs::read(dir.0.join("small.bmp")).unwrap();
    std::fs::write(dir.0.join("broken.bmp"), &bmp[..bmp.len() - 8]).unwrap();

    let read = |path: &'static str| {
        let dir = dir.0.clone();
        async move {
            let result = call(&dir, "read", json!({ "path": path })).await.unwrap();
            let images: Vec<String> = result
                .content
                .iter()
                .filter_map(|block| match block {
                    yapi_types::message::ContentBlock::Image(image) => {
                        Some(image.mime_type.clone())
                    }
                    _ => None,
                })
                .collect();
            (text(&result), images)
        }
    };
    assert_eq!(
        read("small.bmp").await,
        (
            "Read image file [image/png]\n[Image converted from image/bmp to image/png.]".into(),
            vec!["image/png".into()]
        )
    );
    assert_eq!(
        read("wide.png").await,
        (
            "Read image file [image/png]\n[Image: original 2100x4, displayed at 2000x4. Multiply coordinates by 1.05 to map to original image.]".into(),
            vec!["image/png".into()]
        )
    );
    assert_eq!(
        read("broken.bmp").await,
        (
            "Read image file [image/bmp]\n[Image omitted: could not be converted to a supported inline image format.]".into(),
            vec![]
        )
    );
}

#[tokio::test]
async fn bom_and_crlf_preserved() {
    let dir = Scratch::new("edit");
    let result = call(
        &dir.0,
        "write",
        json!({"path": "sub/f.txt", "content": "\u{feff}one\r\ntwo\r\n"}),
    )
    .await
    .unwrap();
    assert_eq!(text(&result), "Successfully wrote to sub/f.txt");

    let result = call(
        &dir.0,
        "edit",
        json!({"path": "sub/f.txt", "edits": [{"oldText": "two\n", "newText": "2\n"}]}),
    )
    .await
    .unwrap();
    assert_eq!(
        text(&result),
        "Successfully replaced 1 block(s) in sub/f.txt."
    );
    assert_eq!(
        std::fs::read_to_string(dir.0.join("sub/f.txt")).unwrap(),
        "\u{feff}one\r\n2\r\n"
    );
    let details = result.details.unwrap();
    assert_eq!(details["diff"], " 1 one\n-2 two\n+2 2");
    assert_eq!(details["firstChangedLine"], 2);

    let err = call(
        &dir.0,
        "edit",
        json!({"path": "nope.txt", "edits": [{"oldText": "a", "newText": "b"}]}),
    )
    .await
    .unwrap_err();
    assert_eq!(err, "Could not edit file: nope.txt. Error code: ENOENT.");

    // Node's messages for a directory: `read` names no path, `open` does.
    std::fs::create_dir(dir.0.join("adir")).unwrap();
    let err = call(&dir.0, "write", json!({"path": "adir", "content": "x"}))
        .await
        .unwrap_err();
    assert_eq!(
        err,
        format!(
            "EISDIR: illegal operation on a directory, open '{}'",
            dir.0.join("adir").display()
        )
    );
}

#[tokio::test]
async fn edit_accepts_legacy_arguments() {
    let dir = env(Path::new("/"));
    let tool = builtin("edit", &dir).unwrap().tool;
    let args = json!({"path": "f", "oldText": "a", "newText": "b"});
    let prepared = tool.prepare_arguments(args.as_object().unwrap().clone());
    assert_eq!(
        Value::Object(prepared),
        json!({"path": "f", "edits": [{"oldText": "a", "newText": "b"}]})
    );
}

#[tokio::test]
async fn bash_output_exit_codes() {
    let dir = Scratch::new("bash");
    let result = call(&dir.0, "bash", json!({"command": "echo hi; echo err >&2"}))
        .await
        .unwrap();
    // stdout and stderr are separate pipes, so their lines may arrive in either order.
    let mut lines: Vec<String> = text(&result).lines().map(str::to_owned).collect();
    lines.sort();
    assert_eq!(lines, ["err", "hi"]);
    assert_eq!(result.structured_content.as_ref().unwrap()["exit_code"], 0);

    let result = call(&dir.0, "bash", json!({"command": "exit 3"}))
        .await
        .unwrap();
    assert_eq!(text(&result), "(no output)\n\nCommand exited with code 3");
    assert_eq!(result.is_error, Some(true));

    let err = call(
        &dir.0,
        "bash",
        json!({"command": "echo start; sleep 5", "timeout": 0.2}),
    )
    .await
    .unwrap_err();
    assert_eq!(err, "start\n\n\nCommand timed out after 0.2 seconds");
}

#[tokio::test]
async fn bash_abort_and_updates() {
    let dir = Scratch::new("abort");
    let tool = builtin("bash", &env(&dir.0)).unwrap().tool;
    let (updates, seen) = sink();
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        trigger.cancel();
    });
    let err = tool
        .execute(
            "id".into(),
            json!({"command": "echo partial; sleep 10"}),
            cancel,
            updates,
        )
        .await
        .unwrap_err();
    assert_eq!(err, "partial\n\n\nCommand aborted");
    let seen = seen.lock().unwrap();
    assert!(seen.len() >= 2, "expected an initial and an output update");
    assert_eq!(text(&seen[1]), "partial\n");
}

#[tokio::test]
async fn read_huge_offset_limit() {
    let dir = Scratch::new("read-huge");
    std::fs::write(dir.0.join("a.txt"), "1\n2\n3").unwrap();
    let result = call(
        &dir.0,
        "read",
        json!({"path": "a.txt", "offset": 2, "limit": 1e20}),
    )
    .await
    .unwrap();
    assert_eq!(text(&result), "2\n3");
    let err = call(&dir.0, "read", json!({"path": "a.txt", "offset": 1e20}))
        .await
        .unwrap_err();
    assert_eq!(
        err,
        "Offset 100000000000000000000 is beyond end of file (3 lines total)"
    );
}

#[tokio::test]
async fn grep_huge_context() {
    let dir = Scratch::new("grep-context");
    std::fs::write(dir.0.join("a.txt"), "one\ntwo\nthree").unwrap();
    let result = call(&dir.0, "grep", json!({"pattern": "two", "context": 1e20}))
        .await
        .unwrap();
    assert_eq!(text(&result), "a.txt-1- one\na.txt:2: two\na.txt-3- three");
}

#[tokio::test]
async fn edit_refuses_non_utf8() {
    let dir = Scratch::new("edit-latin1");
    let original = b"caf\xe9\nx = 1\n";
    std::fs::write(dir.0.join("f.txt"), original).unwrap();
    let err = call(
        &dir.0,
        "edit",
        json!({"path": "f.txt", "edits": [{"oldText": "x = 1", "newText": "x = 2"}]}),
    )
    .await
    .unwrap_err();
    assert_eq!(err, "Could not edit file: f.txt. File is not valid UTF-8.");
    assert_eq!(std::fs::read(dir.0.join("f.txt")).unwrap(), original);
}

#[tokio::test]
async fn edit_keeps_line_endings() {
    let dir = Scratch::new("edit-endings");
    let file = dir.0.join("f.txt");
    for (before, old, new, after) in [
        ("a\nb\r\nc\r\n", "a", "A", "A\nb\r\nc\r\n"),
        ("s = \"x\ry\"\nz\n", "z", "Z", "s = \"x\ry\"\nZ\n"),
        // A fuzzy match: curly quotes and trailing spaces in the file.
        (
            "\u{feff}say \u{201C}hi\u{201D}  \r\nkeep \r\n",
            "say \"hi\"",
            "say \"bye\"",
            "\u{feff}say \"bye\"\r\nkeep \r\n",
        ),
    ] {
        std::fs::write(&file, before).unwrap();
        call(
            &dir.0,
            "edit",
            json!({"path": "f.txt", "edits": [{"oldText": old, "newText": new}]}),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), after, "{before:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn parallel_edits() {
    let dir = Scratch::new("edit-parallel");
    let file = dir.0.join("f.txt");
    let edit = |old: &str, new: &str| {
        call(
            &dir.0,
            "edit",
            json!({"path": "f.txt", "edits": [{"oldText": old, "newText": new}]}),
        )
    };
    for round in 0..20 {
        std::fs::write(&file, "top\nbottom\n").unwrap();
        let (top, bottom) = tokio::join!(edit("top", "TOP"), edit("bottom", "BOTTOM"));
        top.unwrap();
        bottom.unwrap();
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "TOP\nBOTTOM\n", "round {round}");
    }
}

#[tokio::test]
async fn bash_long_output() {
    let dir = Scratch::new("bash-long");
    let result = call(&dir.0, "bash", json!({"command": "seq 1 300000"}))
        .await
        .unwrap();
    let path = result.details.as_ref().unwrap()["fullOutputPath"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(text(&result).ends_with(&format!(
        "\n[Showing lines 298001-300000 of 300000. Full output: {path}]"
    )));
    let structured = result.structured_content.as_ref().unwrap();
    assert_eq!(structured["truncated"], true);
    assert_eq!(structured["full_output_path"], path);
    let full = structured["output"].as_str().unwrap();
    // `seq 1 300000` prints 1,988,895 bytes; the first and last 512 KiB stay.
    let omitted = 1_988_895 - 1024 * 1024;
    assert!(full.starts_with("1\n2\n3\n"));
    assert!(full.contains(&format!("\n\n[... {omitted} bytes omitted ...]\n\n")));
    assert!(full.ends_with("\n299999\n300000\n"));
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn bash_timeout_while_flooding() {
    let dir = Scratch::new("bash-flood");
    let run = call(&dir.0, "bash", json!({"command": "yes", "timeout": 0.5}));
    let err = tokio::time::timeout(std::time::Duration::from_secs(3), run)
        .await
        .expect("the timeout ends a flooding command")
        .unwrap_err();
    assert!(err.ends_with("\n\nCommand timed out after 0.5 seconds"));
    let log = err.split("Full output: ").nth(1).unwrap();
    std::fs::remove_file(log.split(']').next().unwrap()).unwrap();
}
