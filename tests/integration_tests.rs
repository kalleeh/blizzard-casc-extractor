//! CLI-level integration tests that assert observable behaviour.
//!
//! These deliberately avoid the previous "accept any output" shape. That version
//! passed with no mock archive present at all — it only checked that clap had not
//! rejected the arguments, then asserted the existence of a directory the test
//! itself had created. Every assertion here fails if the behaviour it describes
//! breaks.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_casc-extractor"))
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("failed to execute casc-extractor")
}

/// A directory that is not a CASC installation.
fn empty_install() -> TempDir {
    TempDir::new().expect("tempdir")
}

/// The real archive, when one is available. Tests that need the extraction path
/// to actually run must gate on this — with an unopenable archive the tool exits
/// before most logic is reached.
fn archive_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("STARCRAFT_PATH") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    let default = PathBuf::from("/Applications/StarCraft");
    default.exists().then_some(default)
}

#[test]
fn missing_archive_fails_with_a_nonzero_exit_and_an_explanation() {
    let install = empty_install();
    let out = TempDir::new().expect("tempdir");
    let output = run(&[
        "--install-path",
        install.path().to_str().unwrap(),
        "--output",
        out.path().to_str().unwrap(),
        "extract",
        "anim",
        "--ids",
        "0",
    ]);

    assert!(
        !output.status.success(),
        "pointing --install-path at a non-archive must fail, got success"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let lower = combined.to_lowercase();
    assert!(
        lower.contains("casc") || lower.contains("archive") || lower.contains("starcraft"),
        "error should name what it could not open, got: {combined}"
    );
}

#[test]
fn unknown_subcommand_and_flag_are_usage_errors() {
    // clap exits 2 for usage problems; anything else means an argument was
    // silently accepted, or the process died some other way.
    for args in [
        vec!["extract", "nonexistent-subcommand"],
        vec!["extract", "anim", "--definitely-not-a-flag"],
    ] {
        let output = run(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{:?} should be a usage error (exit 2)",
            args
        );
    }
}

#[test]
fn cols_and_tile_size_reject_out_of_range_values() {
    // These used to panic with a divide-by-zero or a multiply overflow.
    for args in [
        vec!["extract", "tileset-atlas", "--cols", "0"],
        vec!["extract", "tileset-atlas", "--tile-size", "0"],
        vec!["extract", "tileset-atlas", "--cols", "100000"],
    ] {
        let output = run(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{:?} should be rejected by the value parser",
            args
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panicked"),
            "{:?} must not panic, stderr: {}",
            args,
            stderr
        );
    }
}

#[test]
fn name_map_rejects_paths_that_escape_the_output_directory() {
    // Needs a real archive: without one the open fails before the name map is
    // ever consulted, which would make this pass for the wrong reason.
    let Some(install) = archive_path() else {
        eprintln!("skipping: no StarCraft archive (set STARCRAFT_PATH to enable)");
        return;
    };
    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    std::fs::create_dir_all(&out).expect("create out");

    let name_map = tmp.path().join("names.json");
    std::fs::write(&name_map, r#"{"0":"../escaped/sprite"}"#).expect("write name map");

    let output = run(&[
        "--install-path",
        install.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
        "extract",
        "anim",
        "--ids",
        "0",
        "--convert-to-png",
        "--name-map",
        name_map.to_str().unwrap(),
    ]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("refusing unsafe output name"),
        "a `..` mapped name must be rejected explicitly, got: {combined}"
    );
    assert!(
        !tmp.path().join("escaped").exists(),
        "a `..` name must not create a directory outside --output"
    );

    // A legitimate nested name must still work, so the guard is not just
    // rejecting everything.
    std::fs::write(&name_map, r#"{"0":"terran/marine"}"#).expect("write name map");
    let ok = run(&[
        "--install-path",
        install.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
        "extract",
        "anim",
        "--ids",
        "0",
        "--name-map",
        name_map.to_str().unwrap(),
    ]);
    assert!(
        ok.status.success(),
        "nested mapped names must still be allowed"
    );
    assert!(
        out.join("terran").join("marine.anim").exists(),
        "expected {}",
        out.join("terran/marine.anim").display()
    );
}

#[test]
fn config_init_writes_a_config_that_the_tool_can_read_back() {
    let tmp = TempDir::new().expect("tempdir");
    let cfg = tmp.path().join("cfg.json");

    let output = run(&["config", "init", "--output", cfg.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "config init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(cfg.exists(), "config init did not write {}", cfg.display());

    let text = std::fs::read_to_string(&cfg).expect("read config");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("config must be valid JSON");
    assert!(
        parsed.get("quality_settings").is_some(),
        "generated config is missing quality_settings: {text}"
    );

    // The tool must accept its own output rather than warning about it.
    let install = empty_install();
    let reread = run(&[
        "-c",
        cfg.to_str().unwrap(),
        "--install-path",
        install.path().to_str().unwrap(),
        "inspect",
        "archive",
    ]);
    let stderr = String::from_utf8_lossy(&reread.stderr);
    assert!(
        !stderr.contains("could not parse config"),
        "tool rejected its own generated config: {stderr}"
    );
}

#[test]
fn validate_run_reports_failure_with_a_nonzero_exit() {
    let tmp = TempDir::new().expect("tempdir");
    let file = tmp.path().join("artifact.bin");
    let suite = tmp.path().join("suite.json");
    std::fs::write(&file, b"original").expect("write artifact");

    let reg = run(&[
        "validate",
        "register",
        file.to_str().unwrap(),
        "--suite",
        suite.to_str().unwrap(),
    ]);
    assert!(reg.status.success(), "register failed");

    let pass = run(&[
        "validate",
        "run",
        tmp.path().to_str().unwrap(),
        "--suite",
        suite.to_str().unwrap(),
    ]);
    assert!(pass.status.success(), "unmodified artifact should pass");

    // A changed artifact must fail loudly — this silently exited 0 before.
    std::fs::write(&file, b"tampered").expect("modify artifact");
    let fail = run(&[
        "validate",
        "run",
        tmp.path().to_str().unwrap(),
        "--suite",
        suite.to_str().unwrap(),
    ]);
    assert!(
        !fail.status.success(),
        "a failing regression check must exit non-zero"
    );
}

#[test]
fn every_documented_subcommand_resolves() {
    // Catches a subcommand wired to a missing handler, and any --help that panics.
    let commands: &[&[&str]] = &[
        &["extract", "anim"],
        &["extract", "tileset"],
        &["extract", "effect"],
        &["extract", "tileset-atlas"],
        &["extract", "tileset-megatile"],
        &["extract", "map-terrain"],
        &["extract", "map-terrain-dual"],
        &["extract", "mainmenu"],
        &["extract", "dat"],
        &["extract", "cmdicons"],
        &["extract", "cursors"],
        &["extract", "raw"],
        &["sounds", "extract"],
        &["sounds", "list"],
        &["sounds", "probe"],
        &["sounds", "export-targets"],
        &["inspect", "sprites"],
        &["inspect", "archive"],
        &["inspect", "files"],
        &["config", "init"],
        &["validate", "register"],
        &["validate", "run"],
    ];
    for cmd in commands {
        let mut args = cmd.to_vec();
        args.push("--help");
        let output = run(&args);
        assert!(
            output.status.success(),
            "`{}` --help failed: {}",
            cmd.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn version_and_help_work_without_an_archive() {
    let v = run(&["--version"]);
    assert!(v.status.success(), "--version must not require an archive");
    let stdout = String::from_utf8_lossy(&v.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "--version should print the crate version, got: {stdout}"
    );

    let h = run(&["--help"]);
    assert!(h.status.success(), "--help failed");
}
