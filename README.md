# compile-llvm

`handl-plugin-compile-llvm` — LLVM compiler for the Handl arrow calculus.
Part of the `compile-*` compiler-backend family: lowers Handl `Term` arrow
terms to LLVM IR.

## Development

```bash
cargo xtask gate        # fmt --check → clippy -D warnings → test → doc
```

`cargo xtask` wraps the repo's `xtask/` crate (`fmt`, `clippy`, `test`, `doc`,
`check`, `gate`); `ci.sh` is the gate entry. Deps resolve from git, so no
sibling checkouts are needed; a plain clone test-runs as-is.

## License

MIT.
