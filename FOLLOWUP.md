# Follow-up tasks

Tracked work items left open after the developer-experience overhaul
(PRs #1–#3). Kept in-repo so it survives a move to another dev machine.

## Done (merged to `main`)

- **#1** Cargo workspace, committed lockfiles, fmt/clippy/test + shellcheck CI,
  75 unit tests, GUI-crate CI, cargo-deny policy, `Makefile`, lint configs,
  monorepo CONTRIBUTING.
- **#2** Post-merge fixes: ISO binary paths (shared workspace `target/`),
  `findmnt`→`util-linux`, cargo-deny tuning (`unmaintained = "workspace"` +
  3 triaged webpki vulns), dropped redundant `cargo audit`.
- **#3** CI ISO build now compiles the whole OS: `pacman-key --init`,
  `protobuf`/`cmake`/`clang`, Wayland deps for `indos-shell`, cargo caching.

## Open

### 1. CI ISO build needs a CachyOS host environment  (deferred)
`Build IndOS ISO` now gets through `[2/5]` compile, `[3/5]` binary install, and
`[4/5]` configs, then fails at `[5/5] mkarchiso`:

```
error: config file /etc/pacman.d/cachyos-v3-mirrorlist could not be read
error parsing 'indos-iso/pacman.conf'
ERROR: mkarchiso failed with exit code 1
```

**Cause:** `mkarchiso` pacstraps against CachyOS repos/mirrorlists. CI runs in a
bare `archlinux:latest` container, which is not a CachyOS host.

**Options:**
1. *(recommended)* Self-hosted or CachyOS-based runner, or a prebuilt CachyOS
   container image — the build expects a CachyOS host.
2. Bootstrap CachyOS on the host in-workflow: copy
   `indos-iso/airootfs/etc/pacman.d/cachyos-*mirrorlist` → `/etc/pacman.d/`, add
   the CachyOS repos to the host `pacman.conf`, import the keyring, then
   pacstrap. Expect a continued chain of blockers; each end-to-end build is
   ~54 min (uncached cargo build of the lancedb tree is ~47 min — caching now
   added).

### 2. Optional hardening (low priority)
- Upgrade `lancedb` 0.15 → 0.30+ to drop the old `rustls 0.21`/`webpki 0.101`
  chain and remove the 3 ignored advisories from `deny.toml`.
- Flip the cargo-deny **licenses** check from advisory (`continue-on-error`) to
  blocking once the transitive license set is curated.
- Trim `tokio` features in `indos-shell`/`indos-canvas` (still `["full"]`).
