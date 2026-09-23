use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

static ORACLE: OnceLock<PathBuf> = OnceLock::new();

fn oracle_path() -> &'static Path {
    ORACLE.get_or_init(|| {
        let reference = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference");
        let build_dir = std::env::temp_dir().join(format!(
            "fastbase91-reference-{}-{}",
            env!("CARGO_PKG_VERSION"),
            std::process::id()
        ));
        std::fs::create_dir_all(&build_dir).expect("create C oracle build directory");
        let executable = build_dir.join("base91-reference");
        let status = Command::new("cc")
            .args(["-std=c99", "-Wall", "-Wextra", "-Werror"])
            .arg(reference.join("vendor/base91-0.6.0/base91.c"))
            .arg(reference.join("oracle.c"))
            .args(["-o"])
            .arg(&executable)
            .status()
            .expect("run host C compiler for reference oracle");
        assert!(status.success(), "C reference oracle compilation failed");
        executable
    })
}

fn run(operation: &str, input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(oracle_path())
        .arg(operation)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start C reference oracle");
    child
        .stdin
        .take()
        .expect("oracle stdin")
        .write_all(input)
        .expect("write bytes to C reference oracle");
    let result = child
        .wait_with_output()
        .expect("wait for C reference oracle");
    assert!(
        result.status.success(),
        "C reference oracle failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    result.stdout
}

pub fn encode(input: &[u8]) -> Vec<u8> {
    run("encode", input)
}

#[allow(dead_code)]
pub fn decode(input: &[u8]) -> Vec<u8> {
    run("decode", input)
}
