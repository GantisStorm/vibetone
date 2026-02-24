# Vibetone

> hear yourself vibe

Real-time sidetone app for vibe coders. Hear your own voice through your headphones so you don't slur your words while you're in the zone.

## What it does

Routes your microphone to your headphones with ultra-low latency (~1.3ms at default settings). Includes a noise gate to silence background noise between words and a voice filter that cleans up rumble and hiss.

## Install

Requires [Rust](https://rustup.rs/).

**macOS / Linux:**

```bash
git clone git@github.com:GantisStorm/vibetone.git
cd vibetone
./release.sh
```

**Windows (PowerShell):**

```powershell
git clone git@github.com:GantisStorm/vibetone.git
cd vibetone
.\release.ps1
```

Then run `vibetone` from anywhere.

## Dev

```bash
./dev.sh
```

Builds debug and runs immediately.

## Features

- Device selection (input/output)
- Configurable buffer size and sample rate
- Volume control
- Noise gate with adjustable threshold
- Voice filter (100Hz high-pass + 8kHz low-pass)
- Cyberpunk UI
