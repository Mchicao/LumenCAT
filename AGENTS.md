# LumenCAT — Agent Rules

## Optimized tooling

- Build state lives in `.cache/target`; never point builds at another `target/` or duplicate it.
- Never set global `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`; flags belong in repo config so builds stay reproducible.
- Release stays conservative: LLVM, `lto = "thin"`, `codegen-units = 1`. Never move release to an experimental backend (Cranelift) or drop safety flags.
- Experimental speedups (nightly, Cranelift, `-Zthreads`, linker swap, Kache) are dev-only: benchmark before/after, keep only what helps, revert anything that breaks GPUI, tests, or the release build.
- One cache wrapper max (Kache when enabled). Never stack sccache on top or run two wrappers.
- Concurrent agents/worktrees share artifacts only through the content-addressed cache; never share one `target/` directly.
- No antivirus/Defender exclusions or security changes without explicit user approval; report them as recommendations instead.
- Prefer the fast loop (incremental check/test) for iteration; run full `cargo clippy --locked --all-targets -- -D warnings`, `cargo fmt`, and the whole suite before claiming done. `unwrap`/`expect` are denied, tests included.
- `#[ignore]` SDK tests need a fresh dir under `output/verification/` via env var; they never overwrite previous runs.

## E2E / GUI verification

- For E2E, UI bugs, and feature validation, read `.cursor/skills/verify-lumencat/SKILL.md` and its map `features/README.md` first.
- GPUI is the only UI. Use disposable instances and projects under `output/verification/`.
- The app is a queued resource (`output/verification/.app-lock.json`): drive it via the controller with `-WaitSeconds`; never launch the EXE yourself.
- Distinguish available guides from executed runs. Core tests prove neither GUI interaction nor Word/Trados compatibility.
- Keep evidence; clean up only processes you created; never take the user's foreground without authorization.
