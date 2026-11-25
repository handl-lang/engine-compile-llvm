# tool-compile-llvm

`handl-plugin-compile-llvm` — LLVM compiler for the Handl arrow calculus.
Part of the `tool-compile-*` compiler-backend family: lowers Handl `Term` arrow
terms to LLVM IR.

## `inkwell` feature (required for real codegen)

The `inkwell` cargo feature is **off by default** and is required for actual
LLVM compilation: it pulls in the `inkwell` crate, whose `build.rs` needs
`llvm-config` on `PATH` (i.e. a local LLVM installation).

```bash
cargo build --features inkwell   # real LLVM backend
cargo build                      # default: compile() fails loudly
```

Without the feature, `LlvmCompiler::compile()` and `DefaultObjectWriter::write()`
return an error naming the missing `inkwell` feature — they no longer emit a
placeholder object. Do not enable the feature in environments without LLVM.

## Development

```bash
cargo run --manifest-path xtask/Cargo.toml -- gate   # fmt → clippy → test → doc
```

`xtask/` provides the repo's `xtask/` crate (`fmt`, `clippy`, `test`, `doc`,
`check`, `gate`); `workflows/gate.yaml` is the gate entry. Deps resolve from git, so no
sibling checkouts are needed; a plain clone test-runs as-is.

## License

MIT.
