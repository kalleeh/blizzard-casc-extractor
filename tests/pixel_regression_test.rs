//! End-to-end pixel regression against committed SHA256 baselines.
//!
//! `tests/baselines/validation-suite.json` pins the exact bytes of representative
//! outputs from every decode path: HD diffuse (BC3) and team-colour mask (BC1
//! punchthrough) at three sizes, an SD sprite (v0x0101 container), a tileset atlas
//! (VR4 stride and geometry) and a map terrain render (ERA table, WPE palette,
//! CV5 indirection). Those are precisely the paths where past bugs produced
//! plausible-looking but wrong images, which no unit test noticed.
//!
//! Requires the real archive, so it no-ops when one is not present — that keeps
//! CI green while still failing loudly for anyone who has the game installed.

use std::path::{Path, PathBuf};
use std::process::Command;

fn archive_available() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("STARCRAFT_PATH") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    let default = PathBuf::from("/Applications/StarCraft");
    default.exists().then_some(default)
}

fn binary() -> PathBuf {
    // Same profile as the test binary: target/<profile>/deps/<test> -> ../
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("casc-extractor")
}

fn run(bin: &Path, args: &[&str]) -> bool {
    Command::new(bin)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn extraction_output_matches_committed_baselines() {
    let Some(install) = archive_available() else {
        eprintln!("skipping: no StarCraft archive (set STARCRAFT_PATH to enable)");
        return;
    };
    let bin = binary();
    if !bin.exists() {
        eprintln!("skipping: {} not built", bin.display());
        return;
    }

    let suite = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/baselines/validation-suite.json");
    assert!(
        suite.exists(),
        "baseline suite missing: {}",
        suite.display()
    );

    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path();
    let install = install.to_str().expect("install path is UTF-8");
    let map = format!("{}/Maps/(8)Char Magma.scm", install);

    // Regenerate every baseline artefact from the archive.
    let hd = out.join("hd");
    let sd = out.join("sd");
    let ta = out.join("ta");
    let mt = out.join("mt");
    let steps: Vec<Vec<&str>> = vec![
        vec![
            "--install-path",
            install,
            "extract",
            "anim",
            "--ids",
            "0,25,250",
            "--quality",
            "hd4",
            "--convert-to-png",
            "--team-color-mask",
            "--raw",
            "--output",
            hd.to_str().unwrap(),
        ],
        vec![
            "--install-path",
            install,
            "extract",
            "anim",
            "--quality",
            "sd",
            "--ids",
            "0",
            "--convert-to-png",
            "--output",
            sd.to_str().unwrap(),
        ],
        vec![
            "--install-path",
            install,
            "extract",
            "tileset-atlas",
            "--tileset",
            "jungle",
            "--max-tiles",
            "16",
            "--cols",
            "4",
            "--output",
            ta.to_str().unwrap(),
        ],
        vec![
            "--install-path",
            install,
            "extract",
            "map-terrain",
            "--scm",
            &map,
            "--output",
            mt.to_str().unwrap(),
        ],
    ];
    for args in &steps {
        assert!(
            run(&bin, args),
            "extraction step failed: {}",
            args.join(" ")
        );
    }

    // Collect the generated files into one flat directory, which is what
    // `validate run` compares against (it matches entries by file name).
    let flat = out.join("flat");
    std::fs::create_dir_all(&flat).expect("create flat dir");
    let mut copied = 0usize;
    for dir in [&hd, &sd, &ta, &mt] {
        for entry in std::fs::read_dir(dir).expect("read output dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                let name = path.file_name().expect("file name");
                std::fs::copy(&path, flat.join(name)).expect("copy output");
                copied += 1;
            }
        }
    }
    assert!(copied > 0, "no PNGs were produced");

    // Non-zero exit means at least one hash diverged.
    let status = Command::new(&bin)
        .args([
            "validate",
            "run",
            flat.to_str().unwrap(),
            "--suite",
            suite.to_str().unwrap(),
        ])
        .status()
        .expect("run validate");

    assert!(
        status.success(),
        "extraction output diverged from tests/baselines/validation-suite.json. \
         If the change is intended, re-register the affected files with \
         `casc-extractor validate register <file> --suite tests/baselines/validation-suite.json`."
    );
}
