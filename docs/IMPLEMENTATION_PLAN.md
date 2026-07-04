# CO-Helper Implementation Plan

> Voice-to-Chat Desktop Agent — Tauri 2 + Rust + React
> Derived from PRD v2 + Brainstorm Synthesis (3-reviewer)

---

## Milestones (Revised from Spec)

| Phase | Scope | Est |
|---|---|---|
| **M1** | Tauri skeleton + chat UI + 9router SSE (text input, no audio) | 1 day |
| **M2** | Mic capture → VAD → segment → debug WAV output | 1 day |
| **M3** | System loopback (WASAPI/PulseAudio monitor) + dual pipeline | 1-2 days |
| **M4** | Whisper-rs STT integration + transcript in UI (labeled) | 1 day |
| **M5** | Full voice→LLM loop + interrupt handling | 1 day |
| **M6** | Desktop polish: tray, hotkeys, panic hide, config, always-on-top | 1-2 days |
| **M7** | Stealth audit + legal disclaimer + crash recovery | 0.5 day |
| **M8** | Packaging (.msi/.deb/.AppImage) | 0.5 day |

Total: ~7-9 days

---

## M1 — Tauri Skeleton + Chat UI + 9router

### Tasks

1. **Scaffold Tauri 2 project**
   - `pnpm create tauri-app co-helper` (React+TS template)
   - Vite + React 18 + TypeScript
   - Install: tailwindcss, zustand, shadcn/ui
   
2. **Frontend: Chat UI skeleton**
   - `ChatWindow.tsx` — message list + input box
   - `MessageBubble.tsx` — role-colored bubbles (user/assistant/meeting)
   - `VadIndicator.tsx` — status pill (idle/listening/processing) [stub]
   - `SourceSelector.tsx` — Mic/System/Both toggle [stub]
   - `store/chat.ts` — Zustand store: messages[], isStreaming, status
   - Wait for Tauri events: `transcript`, `llm_token`, `vad_status`

3. **Backend: 9router SSE client**
   - `commands.rs` — `start_capture` (stub), `stop_capture` (stub), `send_to_llm`, `abort_llm`
   - `llm/router_client.rs` — reqwest POST + SSE stream parser (manual line-based, no SSE crate)
   - `state.rs` — `Arc<Mutex<AppState>>` with message history, abort flag
   - Emit `llm_token` events to frontend via Tauri IPC
   - Config: read `config.toml` for 9router URL + model

4. **Config system**
   - `config.rs` — serde struct, read from `~/.config/co-helper/config.toml`
   - Default config generated on first run
   - Model: `ocg/deepseek-v4` (verified on 9router)

5. **Verify**
   - `pnpm tauri dev` compiles + launches
   - Text input → 9router SSE → streaming tokens in chat UI
   - Abort works (new message mid-stream)

### Deliverable
Working chat app talking to 9router. No audio yet. Screenshot-able.

---

## M2 — Mic Capture + VAD

### Tasks

1. **CPAL mic stream**
   - `audio/capture.rs` — enum AudioSource, AudioFrame struct
   - Open default input device via CPAL
   - Push frames to tokio MPSC channel
   - Stream error callback → log + emit `error` event

2. **Resampler**
   - `audio/resample.rs` — rubato wrapper
   - Device sample rate → 16kHz mono f32
   - Handle 44.1k/48k input

3. **VAD pipeline**
   - `audio/vad.rs` — Silero VAD via `voice_activity_detector` crate
   - chunk_size = 1024 (64ms @ 16kHz)
   - Threshold: positive 0.8, negative 0.35, silence 1000ms (revised from 700ms)
   - On segment cut → emit `vad_status` event + save debug WAV

4. **Verify**
   - Speak into mic → see VAD status change in UI
   - Silence → segment cut → debug WAV file written
   - Hot-plug: unplug USB headset → error logged, no crash

---

## M3 — System Loopback + Dual Pipeline

### Tasks

1. **WASAPI loopback (Windows)**
   - `capture.rs` — open default output as input via CPAL
   - Probe device default config, match sample rate
   
2. **PulseAudio monitor (Linux)**
   - Enum input devices, find `.monitor` suffix
   - PipeWire: PulseAudio compat layer (test both)
   
3. **Dual-source pipeline**
   - Separate VAD instances per source
   - Segment queue with priority: mic > system
   - Label segments: `AudioSource::Mic` → "You", `AudioSource::System` → "Meeting"

4. **AEC detection**
   - Detect if default output = speakers (not headphones)
   - Warn user: "System capture + speakers = echo. Use headphones."
   - No AEC algorithm v1 — just warning

5. **Verify**
   - Play audio on system → VAD detects on system channel
   - Both mic + system → separate segments labeled correctly
   - No cross-contamination

---

## M4 — Whisper-rs STT

### Tasks

1. **whisper-rs integration**
   - `stt/whisper.rs` — WhisperContext singleton, loaded at startup
   - Model: `ggml-large-v3-turbo-q5_0.bin` (GPU) or `ggml-base-q5_0.bin` (CPU fallback)
   - Pin whisper-rs v0.14
   - CUDA feature flag: `whisper-rs = { version = "0.14", features = ["cuda"] }`

2. **GPU auto-detection**
   - Try CUDA context init → if fail, fall back to CPU
   - Config: `stt.gpu = "auto"` (true/false/auto)

3. **STT queue**
   - Segment queue consumer → whisper inference → transcript text
   - Priority: mic segments before system
   - Overlapping segments: queue, don't drop

4. **Post-STT filtering**
   - Min length: 3 chars (filter "The.", "Um.")
   - Whisper hallucination guard: reject repeated phrases
   - Emit `transcript` event with source label

5. **Verify**
   - Speak → transcript appears in chat labeled "You"
   - System audio → transcript labeled "Meeting"
   - Silence/garbage → no hallucination text
   - GPU vs CPU mode switchable via config

---

## M5 — Full Voice→LLM Loop

### Tasks

1. **Wire transcript → 9router**
   - On `transcript` event → build message context (last N messages)
   - System prompt: "You are a helpful assistant listening in on a meeting. Keep answers concise. Meeting is in Indonesian and English."
   - Send to 9router, stream tokens to UI

2. **Interrupt handling**
   - New speech segment while LLM streaming → abort 9router request
   - Drop old token stream, start fresh
   - `abort_llm` command kills reqwest future

3. **Context management**
   - Sliding window: last 10 messages (configurable)
   - System prompt + context + new transcript
   - Label meeting audio vs user audio in context

4. **Verify**
   - Full loop: speak → 9router responds → tokens stream in chat
   - Interrupt: speak again mid-response → old response aborts, new starts
   - Context preserved across segments

---

## M6 — Desktop Polish

### Tasks

1. **Tray icon** (built-in Tauri 2, no plugin)
   - Idle (gray) / listening (blue) / processing (yellow)
   - Click → toggle window
   - Right-click → menu: Start/Stop, Settings, Quit

2. **Global hotkeys** (`tauri-plugin-global-shortcut`)
   - `Ctrl+Shift+Space` → toggle window
   - `Ctrl+Shift+L` → toggle listening
   - `Escape` → panic hide + stop listening
   - Configurable, warn on conflict

3. **Window stealth**
   - `hide_from_taskbar = true` → `WS_EX_TOOLWINDOW` (Win), `_NET_WM_STATE_SKIP_TASKBAR` (Linux)
   - Always-on-top toggle
   - Compact: 400×600, position config

4. **Settings dialog**
   - Model selection (dropdown from 9router models)
   - Audio device selection
   - VAD threshold + silence duration sliders
   - Source toggle (mic/system/both)
   - Hotkey config

5. **Verify**
   - Tray works, hotkeys fire, panic hide instant
   - Settings persist to config.toml
   - App hidden from alt-tab when configured

---

## M7 — Stealth Audit + Legal + Crash Recovery

### Tasks

1. **Stealth audit**
   - Test with actual Zoom + GMeet + Teams
   - Verify: no recording indicator, no participant-visible clue
   - Document findings in `docs/stealth-audit.md`

2. **Legal disclaimer**
   - First-run dialog: "This app records audio. You are responsible for complying with local recording consent laws."
   - Settings: optional recording indicator (visible badge)
   - Document in README

3. **Crash recovery**
   - Append-only transcript log: `~/.config/co-helper/sessions/<session-id>.jsonl`
   - Per segment: `{"ts": "...", "source": "mic", "text": "...", "llm_response": "..."}`
   - On restart: detect unsaved session, offer recovery

4. **Transcript export**
   - Export current session: txt + JSON with timestamps
   - Optional: export to Obsidian vault (config path)

5. **Verify**
   - Kill app mid-meeting → restart → session recovered
   - Export → readable txt/json
   - Legal dialog shown on first run

---

## M8 — Packaging

### Tasks

1. **Production build**
   - `pnpm tauri build`
   - Windows: .msi (WebView2 bundling)
   - Linux: .AppImage + .deb

2. **Binary signing** (Windows)
   - Self-signed cert for dev, document code signing for production

3. **Installer test**
   - Clean machine install test
   - Model download on first run (not bundled in installer)

4. **README**
   - Setup steps, platform requirements, config guide
   - Legal disclaimer prominent

---

## Crate Versions (Pinned)

| Crate | Version | Notes |
|---|---|---|
| `cpal` | 0.15 | Cross-platform audio |
| `rubato` | 0.16 | Resampling |
| `voice_activity_detector` | 0.1 | Silero VAD (pin + pin ort) |
| `ort` | 0.6 | ONNX runtime (pinned for API stability) |
| `whisper-rs` | 0.14 | whisper.cpp binding |
| `reqwest` | 0.12 | HTTP + streaming |
| `tokio` | 1 | Async runtime |
| `serde` | 1 | Serialization |
| `tauri` | 2 | Desktop shell |
| `tauri-plugin-global-shortcut` | 2 | Hotkeys |

## Config Defaults (Revised)

```toml
[llm]
url = "http://localhost:20128"
model = "ocg/deepseek-v4"
max_context_messages = 10
system_prompt = "You are a helpful assistant listening in on a meeting. Keep answers concise. The meeting is in Indonesian and English."

[stt]
mode = "in_process"  # "in_process" | "remote"
model_path = "models/ggml-large-v3-turbo-q5_0.bin"
remote_url = ""
language = "auto"
gpu = "auto"  # true | false | auto

[audio]
mic_device = "default"
loopback_device = "default"
capture_mic = true
capture_system = true

[vad]
positive_threshold = 0.8
negative_threshold = 0.35
chunk_size = 1024
silence_ms = 1000  # revised from 700ms
max_segment_ms = 30000

[hotkeys]
toggle_window = "Ctrl+Shift+Space"
toggle_listening = "Ctrl+Shift+L"
panic_hide = "Escape"

[ui]
always_on_top = false
hide_from_taskbar = true
window_position = "top-right"
show_recording_indicator = false  # legal protection option
```

## 9router Model Mapping

| Use Case | Model | Reason |
|---|---|---|
| Default meeting assistant | `ocg/deepseek-v4` | Good technical reasoning, fast |
| Fast simple responses | `ocg/mimo` | Lightweight |
| Complex analysis | `ocg/kimi` | Strong context |
| Fallback | `combos1` |_COMBO unreliable for tools, fine for chat_ |