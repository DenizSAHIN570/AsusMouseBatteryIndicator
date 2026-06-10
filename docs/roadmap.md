# Roadmap

## Independent versioning for daemon and extension

Currently the release workflow stamps a single tag version (e.g. `v0.2.2`) into
both `daemon/Cargo.toml` and `gnome-extension/metadata.json`. This means every
release bumps both components together.

**Goal:** allow the daemon and the GNOME extension to be released at different
version numbers when one component has no meaningful changes.

### Proposed approach

1. Add `workflow_dispatch` inputs to `release.yml`:
   ```yaml
   workflow_dispatch:
     inputs:
       daemon_version:
         description: 'Daemon version (X.Y.Z), leave blank to use tag'
         required: false
       extension_version:
         description: 'Extension version (X.Y.Z), leave blank to use tag'
         required: false
   ```

2. In the stamp step, resolve each version independently:
   - If the input is provided, use it; otherwise fall back to the tag.
   - Validate both with the same `^[0-9]+\.[0-9]+\.[0-9]+$` check.

3. Keep the tag-push trigger working as-is for the common case where both
   components share the same version.

### Considerations

- The GNOME extension `version` field in `metadata.json` is a GNOME-required
  integer (incremented per GNOME Extensions submission). It is separate from
  `version-name` and must be bumped manually when submitting to extensions.gnome.org.
- If components diverge, `update.sh` will need to distinguish between the
  daemon release asset and the extension release asset (currently both come
  from the same GitHub release).
