use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn bin_path() -> PathBuf {
    // Prefer Cargo-provided env var for the built binary
    if let Ok(p) = env::var("CARGO_BIN_EXE_reasonable") {
        return PathBuf::from(p);
    }
    // Fallback: assume workspace target dir
    let target_dir = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "../target".to_string());
    let mut p = PathBuf::from(target_dir);
    p.push("debug");
    p.push("reasonable");
    if !p.exists() {
        panic!("could not locate CLI binary at {}", p.display());
    }
    p
}

#[test]
fn help_prints_usage() {
    let output = Command::new(bin_path())
        .arg("--help")
        .output()
        .expect("failed to execute binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reasonable"));
}

#[test]
fn run_on_example_writes_output() {
    let bin = bin_path();
    // Create a minimal TTL input
    let mut input = env::temp_dir();
    input.push("reasonable_cli_minimal.ttl");
    let mut f = File::create(&input).expect("create temp ttl");
    writeln!(
        f,
        "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix ex: <urn:> .\nex:a rdf:type owl:Thing .\n"
    )
    .unwrap();

    let mut out_path = env::temp_dir();
    out_path.push("reasonable_cli_test_out.ttl");
    let status = Command::new(&bin)
        .args([input.to_str().unwrap(), "-o", out_path.to_str().unwrap()])
        .status()
        .expect("failed to run CLI");
    assert!(status.success());
    let meta = fs::metadata(&out_path).expect("missing output file");
    assert!(meta.len() > 0);
}

#[test]
fn fail_on_cax_dw_sets_exit_code() {
    let bin = bin_path();
    // Build a temporary TTL that triggers cax-dw: A disjointWith B; i a A and i a B
    let mut input = env::temp_dir();
    input.push("reasonable_cli_caxdw.ttl");
    let mut f = File::create(&input).expect("create temp ttl");
    writeln!(
        f,
        "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix ex: <urn:> .\nex:A a owl:Class .\nex:B a owl:Class .\nex:A owl:disjointWith ex:B .\nex:i rdf:type ex:A .\nex:i rdf:type ex:B .\n"
    )
    .unwrap();

    let mut out_path = env::temp_dir();
    out_path.push("reasonable_cli_caxdw_out.ttl");
    let status = Command::new(&bin)
        .args([
            input.to_str().unwrap(),
            "--fail-on",
            "cax-dw",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .status()
        .expect("run CLI");
    assert_eq!(status.code(), Some(2));
}

#[test]
fn json_error_format_outputs_json() {
    let bin = bin_path();
    // Reuse triggering file
    let mut input = env::temp_dir();
    input.push("reasonable_cli_caxdw2.ttl");
    let mut f = File::create(&input).expect("create temp ttl");
    writeln!(
        f,
        "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix ex: <urn:> .\nex:A a owl:Class .\nex:B a owl:Class .\nex:A owl:disjointWith ex:B .\nex:i rdf:type ex:A .\nex:i rdf:type ex:B .\n"
    )
    .unwrap();

    let mut out_path = env::temp_dir();
    out_path.push("reasonable_cli_json_out.ttl");
    let output = Command::new(&bin)
        .args([
            input.to_str().unwrap(),
            "--error-format",
            "json",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .expect("run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.trim_start().starts_with('['));
    assert!(stdout.contains("OWLRL.CAX_DW") || stdout.contains("cax-dw"));
}

#[test]
fn fail_on_prp_irp_sets_exit_code_and_outputs() {
    let bin = bin_path();
    let mut input = env::temp_dir();
    input.push("reasonable_cli_prpirp.ttl");
    let mut f = File::create(&input).expect("create temp ttl");
    writeln!(
        f,
        "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix ex: <urn:> .\nex:p a owl:IrreflexiveProperty .\nex:x ex:p ex:x .\n"
    )
    .unwrap();

    let mut out_path = env::temp_dir();
    out_path.push("reasonable_cli_prpirp_out.ttl");
    let output = Command::new(&bin)
        .args([
            input.to_str().unwrap(),
            "--error-format",
            "ndjson",
            "--fail-on",
            "prp-irp",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .expect("run CLI");
    // Should fail with exit code 2
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OWLRL.PRP_IRP") || stdout.contains("prp-irp"));
}
