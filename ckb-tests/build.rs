// Extract a small selection of the repository's upstream reference vectors.
// Their YAML consists of scalar hex strings; no YAML dependency is needed.
use std::{env, fs, path::PathBuf};

fn extract(case: &str, fields: &[&str], name: &str) {
    let path = PathBuf::from("../tests").join(case).join("data.yaml");
    println!("cargo:rerun-if-changed={}", path.display());
    let yaml = fs::read_to_string(path).unwrap();
    assert!(yaml.lines().any(|line| line == "output: true"));
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    for field in fields {
        let prefix = format!("{field}: '0x");
        let hex = yaml
            .lines()
            .find_map(|line| line.trim().strip_prefix(&prefix))
            .unwrap()
            .strip_suffix('\'')
            .unwrap();
        let bytes: Vec<_> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        fs::write(out.join(format!("{name}_{field}.bin")), bytes).unwrap();
    }
}

fn main() {
    extract(
        "verify_kzg_proof/kzg-mainnet/verify_kzg_proof_case_correct_proof_3_2",
        &["commitment", "z", "y", "proof"],
        "point",
    );
    extract(
        "verify_blob_kzg_proof/kzg-mainnet/verify_blob_kzg_proof_case_correct_proof_3",
        &["blob", "commitment", "proof"],
        "blob",
    );
}
