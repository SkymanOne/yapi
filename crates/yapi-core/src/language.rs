//! pi's `getLanguageFromPath` in `modes/interactive/theme/theme.ts` of pi
//! `v1.0.0`: the language name pi gives a file's code, for code renderers.

/// The language of the file at `path`, by its extension, as pi names it.
pub fn from_path(path: &str) -> Option<&'static str> {
    let extension = path.rsplit('.').next()?.to_ascii_lowercase();
    Some(match extension.as_str() {
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" => "python",
        "rb" => "ruby",
        "rs" => "rust",
        "go" => "go",
        "java" => "java",
        "kt" => "kotlin",
        "swift" => "swift",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "cs" => "csharp",
        "php" => "php",
        "sh" | "bash" | "zsh" => "bash",
        "fish" => "fish",
        "ps1" => "powershell",
        "sql" => "sql",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "less",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "md" | "markdown" => "markdown",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        "cmake" => "cmake",
        "lua" => "lua",
        "perl" => "perl",
        "r" => "r",
        "scala" => "scala",
        "clj" => "clojure",
        "ex" | "exs" => "elixir",
        "erl" => "erlang",
        "hs" => "haskell",
        "ml" => "ocaml",
        "vim" => "vim",
        "graphql" => "graphql",
        "proto" => "protobuf",
        "tf" | "hcl" => "hcl",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::from_path;

    #[test]
    fn names_languages_by_extension_as_pi() {
        assert_eq!(from_path("src/main.rs"), Some("rust"));
        assert_eq!(from_path("a/b.test.TSX"), Some("typescript"));
        assert_eq!(from_path("Dockerfile"), Some("dockerfile"));
        assert_eq!(from_path("notes.txt"), None);
        assert_eq!(from_path("Makefile.am"), None);
        assert_eq!(from_path("trailing."), None);
    }
}
