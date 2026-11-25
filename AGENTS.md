# Agent guide — tool-compile-llvm

`handl-plugin-compile-llvm` — LLVM compiler for the Handl arrow
calculus (`tool-compile-*` family): lowers Handl `Term` arrow terms to
LLVM IR. The `inkwell` feature is required for real codegen. Rust 2024.

## Layout

| path | role |
| --- | --- |
| `src/lib.rs` | crate docs + public API |
| `src/compiler.rs`, `src/config.rs`, `src/target.rs`, `src/output.rs` | compiler driver, config, target triple, output |
| `src/lower/` | Term → LLVM lowering — `control`, `higher`, `primitive`, `proof` |
| `src/emit/` | object/debug-info emission (`object.rs`, `debug_info.rs`) |
| `src/rt/abi.rs` | runtime ABI helpers |
| `src/tests.rs`, `tests/` | unit + integration tests |
| `benches/`, `build.rs` | benchmarks + build script |
| `xtask/`, `workflows/gate.yaml`, `bupha.toml`, `.cargo/` | vendored repo gate + CI wiring |

## Contract

- Deps resolve from git (`handl-plugin-compile`, `hnl-engine`,
  `handl-language` → kernel/host/engine `branch = "main"`), plus
  `hnl-tool-typecheck` and `hnl-tool-codegen` (`handl-term` feature —
  `impl IrModule` for `handl_calculus::Term`).
- Without `inkwell`, the crate is a structural/verify-only path — real
  LLVM emission is feature-gated; keep tests honest about which path
  they exercise.

## Verification

```bash
cargo xtask gate   # fmt --check → clippy -D warnings → test → doc
```


<!-- myco:agent-awareness -->

## Agent awareness

- Session start / any work discussion: run `myco status --prompt`
  before proposing next steps — know the board first.
- User describes new work → offer to capture it (`myco todo add`,
  `issue new`, `proposal new`). Don't wait to be asked.
- Picking up a task: `myco resume <ID>` is the briefing.
- Wrapping up: `myco handoff <ID>` anything resumable; keep statuses
  honest (claimed → in-progress → blocked/done).

<!-- /myco:agent-awareness -->
