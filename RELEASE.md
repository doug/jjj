# Cutting a release

CI covers what a machine can check on every commit. This file holds what it
cannot: the timing-sensitive benchmark, the live GitHub run, the interactive
surface, and the decisions only a person makes.

Pushing a `v*` tag builds binaries for four targets, publishes a GitHub release
with checksums, and runs `cargo publish`. **A crates.io publish is permanent.**
Work through this list before pushing the tag.

---

## 1. Decide the version

- [ ] Choose the version and update `Cargo.toml`. The release workflow refuses a
      tag that disagrees with it.
- [ ] Write the `CHANGELOG.md` section. The workflow extracts the section whose
      heading starts `## <version>` and uses it verbatim as the release notes —
      it fails the release if there is no such section, so write it first.
- [ ] For a security fix, say plainly which versions are affected and what a user
      should do. Someone reading the notes six months from now needs that more
      than they need a list of commits.

## 2. Run the gates CI cannot

- [ ] **Benchmark at scale.** CI runs a 2K tripwire on a noisy shared runner. The
      real gate is 25K on a quiet machine — no other heavy processes, laptop on
      power:

      ```sh
      cargo build --release
      cd tools/bench
      python3 bench.py --count 25000 --json /tmp/release-25k.json \
          --check-against baseline-25k.json --tolerance 2.0
      ```

      Investigate anything above 2x. If a change is a deliberate trade, record the
      new numbers in `tools/bench/baseline-25k.json` and say why in the changelog.

- [ ] **Live GitHub run.** The everyday suite drives a stub `gh`. Confirm the stub
      still matches the real CLI:

      ```sh
      JJJ_LIVE_GITHUB=1 cargo test --test github_live_test
      ```

      A failure here usually means `gh` changed its output, not that jjj broke —
      fix `tests/fixtures/fake-gh/gh` to match, or the hermetic suite is now
      testing a CLI that no longer exists.

- [ ] **Semantic feature end to end.** CI only type-checks it:

      ```sh
      cargo test --features semantic
      ```

- [ ] **Drive the TUI by hand.** `tests/tui_test.rs` asserts on a `TestBackend`
      buffer, which cannot see colour, cursor position, resize behaviour, or the
      editor suspend/resume round trip. In a real terminal:

      ```sh
      cargo run --release -- ui
      ```

      Check: the tree renders and navigates; `Shift+K`/`Shift+J` reorder and a
      double-tap flings; `p` cycles the gap; `Ctrl+Z` undoes; `E` opens `$EDITOR`
      and the screen restores cleanly on exit; resizing the window redraws
      without artefacts; `q` leaves the terminal in a sane state.

- [ ] **Install from a clean machine.** After the release publishes, in a fresh
      container or a different user account:

      ```sh
      curl -fsSL https://raw.githubusercontent.com/doug/jjj/main/install.sh | bash
      jjj --version && jjj doctor
      ```

## 3. Check the upgrade path

- [ ] `jjj doctor` on a repository created by the **previous** release reports no
      problems.
- [ ] If the on-disk format changed at all, add a corpus for the outgoing version
      under `tests/fixtures/` — the recipe is in `tests/fixtures/README.md`. Do
      this *before* the release, while the binary is still easy to build.
- [ ] If the SQLite schema version changed, confirm an old cache migrates or
      rebuilds rather than erroring.

## 4. Ship it

- [ ] `git push origin main`
- [ ] `git tag -a vX.Y.Z -m "vX.Y.Z"` and `git push origin vX.Y.Z`
- [ ] Watch the Release workflow. It verifies the tag, builds four targets,
      publishes the GitHub release, then publishes to crates.io — in that order,
      so the irreversible step runs last.
- [ ] Confirm `cargo install jjj` fetches the new version.

## 5. After

- [ ] For a security release, make the advisory findable: it belongs in the
      release notes and the README's upgrade note, not only in a commit message.
- [ ] Bump `Cargo.toml` to the next patch version so `main` is never mistaken for
      the released build.

---

## Required repository secrets

| Secret | Used by | For |
|---|---|---|
| `CARGO_REGISTRY_TOKEN` | `release.yml` → `publish` | `cargo publish` |

`GITHUB_TOKEN` is provided automatically.
