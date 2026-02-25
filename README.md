<p align="center">
  <img src="assets/icon.png" width="200" alt="vibetone" />
</p>

<h3 align="center">hear yourself vibe</h3>

---

### `> why_`

Closed-back headphones and IEMs block your own voice — the **occlusion effect**. Without natural air and bone conduction feedback, your brain loses track of how you sound. You unconsciously speak louder, mumble, or lose control of your vocal tone. This is especially noticeable during long sessions with noise-isolating headphones.

**Sidetone** fixes this by routing your mic back to your headphones with near-zero latency, restoring the feedback loop your brain expects. Many gaming headsets have this built in — Vibetone brings it to any mic + headphone combo.

```
sub-millisecond latency  //  noise gate  //  voice filter  //  cyberpunk ui
```

Latency depends on your buffer size and sample rate — tune it to your hardware:

| Buffer | 44.1 kHz | 48 kHz | 96 kHz |
|--------|----------|--------|--------|
| 16     | 0.4ms    | 0.3ms  | 0.2ms  |
| 32     | 0.7ms    | 0.7ms  | 0.3ms  |
| 64     | 1.5ms    | 1.3ms  | 0.7ms  |
| 128    | 2.9ms    | 2.7ms  | 1.3ms  |
| 256    | 5.8ms    | 5.3ms  | 2.7ms  |
| 512    | 11.6ms   | 10.7ms | 5.3ms  |
| 1024   | 23.2ms   | 21.3ms | 10.7ms |

Smaller buffer = lower latency but more CPU demand. Anything under ~10ms is imperceptible.

---

### `> install_`

Requires [Rust](https://rustup.rs/).

**macOS / Linux:**

```bash
git clone git@github.com:GantisStorm/vibetone.git
cd vibetone
./release.sh
```

- **macOS** — installs `Vibetone.app` to `/Applications`. Open from Spotlight or Launchpad.
- **Linux** — installs binary to `~/.local/bin` + adds app launcher entry.

**Windows (PowerShell):**

```powershell
git clone git@github.com:GantisStorm/vibetone.git
cd vibetone
.\release.ps1
```

Installs to `%LOCALAPPDATA%\Vibetone` + adds Start Menu shortcut.

---

### `> dev_`

```bash
./dev.sh
```

Builds debug and runs immediately. Pass args with `./dev.sh --help`.

---

### `> features_`

```
[x] device selection (input/output)
[x] configurable buffer size (16–1024) + sample rate (44.1/48/96 kHz)
[x] real-time validation — warns if your device doesn't support the selected combo
[x] volume control
[x] noise gate w/ adjustable threshold
[x] voice filter (100Hz HPF / 8kHz LPF)
[x] cyberpunk terminal ui
```

---

```
built with rust  //  powered by cpal + egui  //  vibe coded with claude
```
