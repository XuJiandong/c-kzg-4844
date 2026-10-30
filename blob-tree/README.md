# Blob Tree

A Rust membership proofs over two levels of EIP-4844 blobs, with capacity for up to 4096² elements.

- [Tutorial](tutorial/README.md): a tutorial, read it first
- [Design](DESIGN.md): the detailed design

## Usage

A minimal example of generating and verifying a proof:

```Rust
    let values = (0..WIDTH + 2)
        .map(|i| format!("element-{i}").into_bytes())
        .collect::<Vec<_>>();
    let tree = Tree::new(&values)?;
    let (values, proof) = tree.prove(&[0, 7, WIDTH, WIDTH + 1])?;
    let bytes = proof.to_bytes();
    let received = Proof::from_bytes(tree.root(), &bytes)?;
    assert!(verify(tree.root(), &values, &received)?);
```

## Verification on CKB-VM

Use `verification-only` with default features disabled:

```toml
blob-tree = { path = "../blob-tree", default-features = false, features = ["verification-only"] }
```

See more examples in [ckb-tests](../ckb-tests).
