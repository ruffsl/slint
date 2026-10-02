// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0

//! The paths the Rust generator passes to `include_bytes!` for embedded files.
//! With a destination file they're relative to it, so the generated code doesn't depend on where the sources are.

#![cfg(feature = "rust")]

use i_slint_compiler::diagnostics::BuildDiagnostics;
use i_slint_compiler::generator::{self, OutputFormat};
use i_slint_compiler::parser::parse;
use i_slint_compiler::{CompilerConfiguration, EmbedResourcesKind, compile_syntax_node};
use std::path::{Path, PathBuf};

/// A fresh directory with `res/icon.svg` in it, unique to `name`.
fn fixture(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rust_embedded_files").join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("res")).unwrap();
    std::fs::write(root.join("res/icon.svg"), "<svg/>").unwrap();
    root
}

/// Generate the Rust code of a window that embeds `res/icon.svg` from `root`,
/// as written to `destination`, and return the argument of its `include_bytes!`.
fn embedded_path(root: &Path, destination: Option<&Path>) -> String {
    let source =
        r#"export component Test inherits Window { Image { source: @image-url("icon.svg"); } }"#;
    let mut diagnostics = BuildDiagnostics::default();
    let syntax_node = parse(source.into(), Some(&root.join("res/test.slint")), &mut diagnostics);
    let mut config = CompilerConfiguration::new(OutputFormat::Rust);
    config.embed_resources = EmbedResourcesKind::EmbedAllResources;
    let (doc, diagnostics, loader) =
        spin_on::spin_on(compile_syntax_node(syntax_node, diagnostics, config));
    assert!(!diagnostics.has_errors(), "{:?}", diagnostics.to_string_vec());

    let mut output = Vec::new();
    generator::generate(
        OutputFormat::Rust,
        &mut output,
        destination,
        &doc,
        &loader.compiler_config,
    )
    .unwrap();
    let output = String::from_utf8(output).unwrap();
    let include = regex::Regex::new(r#"include_bytes\s*!\s*\(\s*"([^"]*)"\s*\)"#).unwrap();
    let paths: Vec<_> = include.captures_iter(&output).map(|c| c[1].to_string()).collect();
    assert_eq!(paths.len(), 1, "{output}");
    paths.into_iter().next().unwrap()
}

#[test]
fn relative_to_the_destination() {
    let root = fixture("relative_to_the_destination");
    std::fs::create_dir_all(root.join("out")).unwrap();
    assert_eq!(embedded_path(&root, Some(&root.join("out/generated.rs"))), "../res/icon.svg");
}

#[test]
fn absolute_without_a_destination() {
    let root = fixture("absolute_without_a_destination");
    let path = embedded_path(&root, None);
    assert!(Path::new(&path).is_absolute(), "{path}");
}

#[cfg(unix)]
#[test]
fn through_a_symbolic_link() {
    // `link/..` is `deep`, not the fixture root.
    let root = fixture("through_a_symbolic_link");
    std::fs::create_dir_all(root.join("deep/out")).unwrap();
    std::os::unix::fs::symlink(root.join("deep/out"), root.join("link")).unwrap();
    let path = embedded_path(&root, Some(&root.join("link/generated.rs")));
    assert_eq!(path, "../../res/icon.svg");
    assert_eq!(
        std::fs::canonicalize(root.join("link").join(&path)).unwrap(),
        std::fs::canonicalize(root.join("res/icon.svg")).unwrap()
    );
}
