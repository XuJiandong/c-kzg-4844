#!/usr/bin/env python3
"""Pass test and benchmark inputs to CKB-VM through argv."""

import argparse
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parent.parent
VECTORS = ROOT / "tests"
INFINITY = b"\xc0" + bytes(47)
ZERO = bytes(32)
ONE = bytes(31) + b"\x01"


def vector(path, fields):
    # These upstream YAML files contain scalar hex strings, so no YAML package
    # is needed. Keep the expected output too, allowing negative reference cases.
    yaml = path.read_text()
    result = re.search(r"^output: (true|false|null)$", yaml, re.MULTILINE)
    if result is None:
        raise ValueError(f"{path}: missing output")
    values = []
    for field in fields:
        match = re.search(rf"^\s+{field}: '0x([0-9a-fA-F]*)'$", yaml, re.MULTILINE)
        if match is None:
            raise ValueError(f"{path}: missing {field}")
        value = bytes.fromhex(match[1])
        size = {"commitment": 48, "proof": 48, "z": 32, "y": 32, "blob": 131072}[field]
        if len(value) != size:
            raise ValueError(f"{path}: {field} must be {size} bytes for the fixed-size verifier API")
        values.append(value)
    expected = {"true": "true", "false": "false", "null": "bad-arguments"}[result[1]]
    return values, expected


def run(args, label, payload, *prefix):
    print(f"\n{label}", flush=True)
    # A blob's hex encoding exceeds Linux's per-argument size limit.
    hex_data = payload.hex()
    chunks = [hex_data[i:i + 32768] for i in range(0, len(hex_data), 32768)]
    result = subprocess.run(
        [args.debugger, "--bin", str(args.binary), "--max-cycles", str(args.max_cycles),
         "--", *prefix, *chunks],
    )
    if result.returncode:
        raise SystemExit(f"{label}: ckb-debugger exited with {result.returncode}")


def counted(values):
    return len(values).to_bytes(8, "big") + b"".join(values)


def kzg(args):
    key = (args.g2_key.read_bytes() if args.g2_key else
           (ROOT / "bindings/rust/src/ethereum_kzg_settings/g2_monomial_bytes.bin")
           .read_bytes()[96:192])
    if len(key) != 96:
        raise ValueError("G2 key must be exactly 96 bytes")

    def check(mode, label, expected, *values):
        run(args, label, key + b"".join(values), mode, expected)

    run(args, "invalid verification key", bytes(96), "key", "bad-arguments")
    if args.case in ("all", "point"):
        check("point", "zero point proof", "true", INFINITY, ZERO, ZERO, INFINITY)
        check("point", "incorrect point value", "false", INFINITY, ZERO, ONE, INFINITY)
        check("point", "malformed commitment", "bad-arguments", bytes(48), ZERO, ZERO, INFINITY)
        check("point", "malformed scalar", "bad-arguments", INFINITY, bytes([255]) * 32, ZERO, INFINITY)
        values, expected = vector(args.point_vector, ("commitment", "z", "y", "proof"))
        check("point", str(args.point_vector), expected, *values)
        if expected == "true":
            commitment, z, y, proof = values
            changed_y = ONE if y == ZERO else ZERO
            check("point", "changed reference value", "false", commitment, z, changed_y, proof)

    if args.case in ("all", "blob", "batch"):
        (blob, commitment, proof), expected = vector(args.blob_vector, ("blob", "commitment", "proof"))
        if args.case in ("all", "blob"):
            check("blob", "zero blob proof", "true", bytes(131072), INFINITY, INFINITY)
            check("blob", str(args.blob_vector), expected, blob, commitment, proof)
            if expected == "true":
                check("blob", "malformed blob commitment", "bad-arguments", blob, bytes(48), proof)
                if commitment != INFINITY:
                    check("blob", "incorrect blob commitment", "false", blob, INFINITY, proof)
        if args.case in ("all", "batch"):
            check("batch", "empty batch", "true", counted([]), counted([]), counted([]))
            blobs = counted([blob, blob])
            commitments = counted([commitment, commitment])
            proofs = counted([proof, proof])
            check("batch", "mismatched batch", "mismatch-length", blobs, counted([commitment]), counted([proof]))
            check("batch", "reference batch", expected, blobs, commitments, proofs)
            if expected == "true" and proof != INFINITY:
                check("batch", "incorrect batch proof", "false", blobs, commitments, counted([proof, INFINITY]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--debugger", default="ckb-debugger")
    parser.add_argument("--max-cycles", type=int, default=3500000000)
    commands = parser.add_subparsers(dest="command", required=True)
    kzg_parser = commands.add_parser("kzg")
    kzg_parser.add_argument("--binary", type=Path, required=True)
    kzg_parser.add_argument("--case", choices=("all", "point", "blob", "batch"), default="all")
    kzg_parser.add_argument("--g2-key", type=Path, help="raw 96-byte verification key")
    kzg_parser.add_argument("--point-vector", type=Path, default=VECTORS / "verify_kzg_proof/kzg-mainnet/verify_kzg_proof_case_correct_proof_3_2/data.yaml")
    kzg_parser.add_argument("--blob-vector", type=Path, default=VECTORS / "verify_blob_kzg_proof/kzg-mainnet/verify_blob_kzg_proof_case_correct_proof_3/data.yaml")
    tree_parser = commands.add_parser("blob-tree")
    tree_parser.add_argument("--binary", type=Path, required=True)
    tree_parser.add_argument("--benchmark", action="store_true",
                             help="measure valid proof verification without adversarial checks")
    tree_parser.add_argument("fixtures", type=Path, nargs="+")
    args = parser.parse_args()
    if args.command == "kzg":
        kzg(args)
    else:
        for fixture in args.fixtures:
            prefix = ("--bench",) if args.benchmark else ()
            run(args, str(fixture), fixture.read_bytes(), *prefix)


if __name__ == "__main__":
    main()
