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

---

## Battery history, smarter time estimates, and health tracking

Three related features that share a common foundation: a persistent store of
battery readings on disk.

### 1. Historical data store

The daemon currently holds all state in memory; it is lost on restart. A
persistent store is the prerequisite for everything below.

**Proposed approach:**
- Write readings to a SQLite database (e.g. `~/.local/share/mouse-battery/history.db`)
  using a simple schema: `(timestamp, percentage, voltage_mv, status)`.
- Append a row on every poll tick. Prune records older than a configurable
  retention window (e.g. 90 days) to keep the file small.
- Expose the path (or a DBus method to query it) so the extension can read it
  directly via GLib; avoid streaming large datasets over DBus.

### 2. Battery stats graph in the popup

A charge/discharge graph in the GNOME popup, similar to what smartphones show
for daily battery usage.

**Proposed approach:**
- Draw the graph as an SVG or using a Cairo `DrawingArea` in the extension.
- X-axis: time (last 8–24 hours, configurable). Y-axis: percentage (0–100).
- Colour-code segments by status (green = charging, grey = discharging).
- Keep the graph lightweight — read from the local SQLite file directly rather
  than pulling data through DBus.

**Open question:** GNOME Shell extensions run in the compositor process, so
heavy I/O or computation on the main thread can cause visible jank. If the
history file grows large, reading should be done asynchronously (GTask /
`Gio.File.read_async`).

### 3. History-based time-to-empty / time-to-full

The current predictor (`predictor.rs`) uses only the readings from the current
daemon session. A cold-started daemon has no data and reports "Calculating…"
until enough samples accumulate.

**Proposed approach:**
- On startup, seed the predictor with the most recent N readings from the
  history database so estimates are available immediately.
- For longer-horizon estimates, fit a discharge curve over the last few cycles
  rather than just the current session's linear regression.

### 4. Battery health inference

Estimate battery health (capacity relative to design capacity) from cycle
history, similar to smartphone battery health metrics.

**Proposed approach:**
- Track full discharge cycles: detect when the battery goes from ≥95% to ≤5%
  without interruption and record the voltage curve across that cycle.
- Over time, compare the voltage-at-percentage curve against a reference
  baseline captured during the first recorded full cycle. Degradation shows
  as lower voltage at the same percentage.
- Expose a `BatteryHealth` DBus property (0–100%) and display it in the popup.

**Caveat:** the mouse firmware does not expose rated capacity or cycle count
directly, so health can only be inferred indirectly from voltage behaviour.
The estimate will be approximate, especially on mice that report coarse
percentage steps (e.g. increments of 5%).
