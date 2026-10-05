# LumenCAT — Agent Rules

## Optimized tooling

- New machine bootstrap BEFORE building: `cargo install kache --locked` (the project wraps rustc with kache; builds fail without it — verify with `kache doctor`) and `cargo install cargo-nextest --locked`. Stable Rust >= 1.99; nightly not required.
- Fast loop: `cargo fast-check` (incremental check), `cargo fast-test` (nextest), `cargo fast` (build+run), `cargo dbg` (full debuginfo). `cargo test` stays valid for doctests/special cases.
- Build state lives in `.cache/target`; never point builds at another `target/` or duplicate it. Worktrees get their own target and reuse deps through the kache store (hardlinks, 20GiB cap, auto GC).
- Never set global `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`; flags belong in repo config so builds and cache keys stay reproducible.
- Kache is the only cache wrapper (project-scoped `rustc-wrapper`). Never stack sccache or a second wrapper; use `kache stats --last-build` / `kache explain` for hit rates and misses.
- Release stays conservative: LLVM, `lto = "thin"`, `codegen-units = 1`. Never move release to an experimental backend (Cranelift) or drop safety flags.
- Experimental speedups (nightly, Cranelift, `-Zthreads`, linker swaps) are dev-only, need before/after benchmarks, and were already tried and rejected with evidence — see `docs/technical/TOOLING_20261005.md` before re-testing one.
- No antivirus/Defender exclusions or security changes without explicit user approval; report them as recommendations instead.
- Gate before claiming done: full `cargo clippy --locked --all-targets -- -D warnings`, `cargo fmt`, and the whole suite. `unwrap`/`expect` are denied, tests included.
- `#[ignore]` SDK tests need a fresh dir under `output/verification/` via env var; they never overwrite previous runs.

## E2E / GUI verification

- For E2E, UI bugs, and feature validation, read `.cursor/skills/verify-lumencat/SKILL.md` and its map `features/README.md` first.
- GPUI is the only UI. Use disposable instances and projects under `output/verification/`.
- The app is a queued resource (`output/verification/.app-lock.json`): drive it via the controller with `-WaitSeconds`; never launch the EXE yourself.
- Distinguish available guides from executed runs. Core tests prove neither GUI interaction nor Word/Trados compatibility.
- Keep evidence; clean up only processes you created; never take the user's foreground without authorization.
