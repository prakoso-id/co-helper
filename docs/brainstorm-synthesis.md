# Brainstorm Synthesis — Spec v2.1 Corrections

## From 3 Independent Reviewers

### Reviewers
- **A** — Architecture, crate choices, CPAL pitfalls
- **B** — Product/UX, latency budget, alternative approaches  
- **C** — Stealth threat model, legal/ethical

---

### 🔴 MUST FIX (Critical)

| # | Issue | Source | Fix |
|---|---|---|---|
| 1 | **700ms silence too aggressive** — natural speech pauses 800-1200ms, mid-sentence cuts | A, B | Default 1000ms, configurable, max-segment 30s cap |
| 2 | **No STT queue** — dual source segments pile up, Whisper single-threaded | A | Segment queue with source priority (mic > system) |
| 3 | **Whisper model lifecycle undefined** — must be singleton, loaded once at startup | A | Load at app init, keep resident, never reload |
| 4 | **No AEC** — speakers leak system audio into mic = duplicate transcripts | A, B | Detect headphone vs speaker; warn if no headphones |
| 5 | **Whisper hallucination on silence/noise** — garbage text sent to LLM | A | Confidence/length filter post-STT, min chars threshold |
| 6 | **No crash recovery** — app crash mid-meeting = transcript gone | A | Append-only transcript log to disk per segment |
| 7 | **9router HTTP plaintext over LAN** — tcpdump readable | A, C | Default localhost; if remote, document TLS requirement |
| 8 | **No GPU detection + fallback** — CPU-only machine silently 4-7x slower | A, B | Auto-detect CUDA, fallback to smaller model or remote mode |
| 9 | **Threat model "Zoom cannot detect" = FALSE as absolute claim** | C | Soften to "highly unlikely via standard APIs"; document residual risks |
| 10 | **No legal/ethical section** — two-party consent jurisdictions | C | Add explicit legal disclaimer, optional recording indicator |

### 🟡 SHOULD FIX

| # | Issue | Source | Fix |
|---|---|---|---|
| 11 | **CPAL loopback config mismatch** — silent failure if sample rate mismatch | A | Probe device default config, adapt resampler |
| 12 | **Device hot-plug** — USB headset disconnect kills stream | A | Stream error callback -> rebuild on device change |
| 13 | **No volume normalization/AGC** — VAD threshold impossible to tune | A, B | RMS-based normalization pre-VAD |
| 14 | **whisper-rs v0.14 API changed from v0.13** | A | Pin v0.14, update code samples |
| 15 | **ort crate API churn** — Silero VAD pin both crates | A | Pin versions in Cargo.toml |
| 16 | **Silero VAD .onnx supply chain** — no hash pinning | A | Pin SHA256, download from official release |
| 17 | **Tauri IPC high-freq bottleneck** — VAD level 30-60/sec | A | VAD viz in Rust, only segment boundaries via IPC |
| 18 | **No transcript export** — missing format spec | A, B | JSON+txt with timestamps |
| 19 | **Language auto-detect unreliable** for short segments | A | Default configurable language, auto as option |
| 20 | **Meeting session lifecycle** — no session ID | A | Session abstraction with unique ID |
| 21 | **Device contention detectable** — Zoom could check WASAPI session count | C | Accept residual risk, document |
| 22 | **Performance signature** — GPU spike during Whisper detectable by EDR | C | Out of scope, document |
| 23 | **DLP/MDM** — no realistic bypass on corporate laptop | C | Document: personal laptop only |
| 24 | **Participant toggle insufficient consent** | C | Add disclaimer, user responsibility |

### 🟢 NICE TO HAVE (v2+)

| # | Item | Source |
|---|---|---|
| 25 | Speaker diarization on system audio | A |
| 26 | Audio device selection UI | A, B |
| 27 | Transcript at-rest encryption | A, C |
| 28 | Memory budgeting for low-end machines | A |
| 29 | faster-whisper sidecar for CPU-only (4x improvement) | A, B |
| 30 | Recording indicator option for legal protection | C |

---

### Verdict: Tauri + Rust = Correct Choice

All 3 reviewers agree: Tauri+Rust beats Electron/Python for this use case.

### Latency Budget (Revised)

| Stage | GPU | CPU |
|---|---|---|
| Audio buffer | 16-32ms | 16-32ms |
| VAD inference | <1ms | 2-5ms |
| Resampling | 5-15ms | 5-15ms |
| Silence wait | **1000ms** (revised) | **1000ms** |
| Whisper (5-15s seg) | 200-600ms | 3-8s |
| 9router TTFT | 200-500ms | 200-500ms |
| UI render | <50ms | <50ms |
| **Total** | **~1.5-2.2s** | **~5-9s** |

<2s target: GPU-only. CPU = 5-9s. Config default: mode=remote for 9router.
