# LocalFlow — Architecture & Build Spec

**What it is:** A free, fully local, cross-platform (macOS first, Windows next) voice dictation app that replicates Wispr Flow's core functionality — dictation, cleanup, snippets, commands, style formatting — with zero cloud dependency, zero accounts, and zero subscription cost.

**Core principle:** Ship a rock-solid MVP dictation loop first (matching Flow's accuracy/latency), then layer in secondary features, which are all cheap once the core loop and its supporting infrastructure exist.

---

## 1. Tech Stack

| Layer | Choice | Notes |
|---|---|---|
| App shell | **Tauri** | Lightweight vs. Electron, Rust backend, native OS API access |
| Frontend | **React + Tailwind + Bootstrap Icons** | |
| STT | **Whisper large-v3-turbo via whisper.cpp** | See §2 — NOT SenseVoice, NOT MLX |
| Cleanup/formatting LLM | **Qwen 2.5 3B-Instruct via llama.cpp** | See §3 |
| Local inference runtime | **whisper.cpp + llama.cpp**, both via Rust bindings (`whisper-rs`, `llama-cpp-rs`) | Cross-platform, no Python dependency, no MLX |
| Storage | **SQLite (`rusqlite`) for history/transcripts** + **JSON for settings/config** | See §5 |
| VAD | **Silero VAD** (or similar lightweight local VAD) | Dual purpose: end-of-utterance detection AND noise/silence trimming before STT |

### Explicitly rejected / avoid
- **MLX** — Apple-only, cannot port to Windows. Do not build the STT/LLM pipeline around it despite its speed edge on Mac.
- **SenseVoice-Small** — only supports 5 languages (Chinese, English, Cantonese, Japanese, Korean). Insufficient for a "dictate in any language" goal.
- **Pure regex/UserDefaults-style dictionary correction** — too brittle; use hybrid fuzzy-match + LLM prompt injection instead (§4).

---

## 2. Speech-to-Text (STT)

**Model:** Whisper large-v3-turbo (809M params, distilled from large-v3, decoder pruned from 32→4 layers)

- ~99 languages supported, auto-detect capable — solves the multilingual/name-accuracy problem SenseVoice-Small couldn't.
- Near large-v3 accuracy at ~4-6x the speed; strong fit for real-time dictation latency budgets.
- Runtime: **whisper.cpp**, not MLX — cross-platform (macOS + Windows + Linux), Rust bindings available (`whisper-rs`), supports Metal + Core ML acceleration on Apple Silicon (do not assume CPU-only performance — Core ML backend gets significant speedup on Mac).
- Expect whisper.cpp to be somewhat slower than MLX on raw benchmarks (~30-100% depending on config), but for dictation-length clips (seconds, not minutes) the absolute latency difference is small — validate on real hardware with real dictation-length clips, not long-form benchmarks.

**Noise robustness:**
- Whisper's encoder (unchanged in turbo) is inherently reasonably robust to background noise/accents, but WER still degrades ~5-15% in noisy conditions, and turbo's pruned decoder may hallucinate slightly more on short/noisy clips than full large-v3.
- **Mitigation: run VAD before STT.** Trim silence/noise, isolate actual speech segments. This VAD pass does double duty — it's the same mechanism needed for end-of-utterance/hotkey-release detection in the UX flow.

**RAM budget:** ~1.5-2GB for Whisper-turbo (quantized), comfortable on any 16GB+ unified-memory Mac. Combined with Qwen 3B (~2GB) and app overhead, treat **16GB as the realistic minimum spec** to advertise, even if lower works with careful quantization.

---

## 3. Cleanup / Formatting LLM

**Model:** Qwen 2.5 3B-Instruct, run via **llama.cpp** (`llama-cpp-rs` bindings) — cross-platform, no MLX/Python dependency.

**Strengths to lean on:**
- Strong at structured JSON output — use this to have a single call return `{"cleaned_text": "...", "detected_command": ..., "snippet_match": ...}` rather than running separate passes.
- Solid multilingual support (29+ languages) for cleanup/grammar tasks in non-English dictation.
- Good general instruction-following for straightforward cleanup/grammar/punctuation tasks — this part of the pipeline is low-risk.

**Known weak spot — verify empirically, don't assume:**
- Multi-step backtracking/self-correction resolution (e.g. "ship Friday — no wait, make it Tuesday" → "ship Tuesday") requires the model to track a temporal invalidation relationship across a sentence. This is a step up in reasoning complexity from plain cleanup, and general instruction-following benchmarks don't isolate this specific skill. **Test on real rambling speech before assuming it works.**
- **Mitigation if 3B underperforms on backtracking:** don't bundle a second larger model (avoid inflating app package size). Instead:
  1. Try better few-shot prompting first (3-5 concrete backtracking examples in the system prompt) — often closes most of the gap.
  2. If still weak, add a **deterministic keyword pre-pass**: detect backtracking markers ("no wait", "scratch that", "actually make it", "hold on") via simple string/regex matching, and split/discard the invalidated segment *before* it reaches the LLM — offloading the hard reasoning step to string matching rather than model inference.

**System prompt design — likely converges to ONE call per utterance handling multiple tasks:**
- Grammar/punctuation cleanup
- Backtracking resolution (with pre-pass fallback per above)
- App-context-aware formatting (inject a formatting instruction based on frontmost app — see §6)
- Snippet/command intent detection (inject the user's snippet library as context; ask model to detect if the utterance matches a snippet trigger and splice in the value — see §7)
- Dictionary-based name/term correction (inject user's custom dictionary as context — see §4)

This collapses what could have been several separate pipeline stages into one well-designed prompt + one Qwen call per utterance, since the infrastructure (one LLM call already running post-STT) is shared across all these features.

---

## 4. Dictionary (custom word/name correction)

**Priority: HIGH — build early, not a "dashboard nice-to-have."** This is a correction mechanism baked into the core loop, not a UI feature.

**Mechanism — hybrid, not pure regex:**
1. **Fast fuzzy-match pass** (Levenshtein distance or similar) against the user's dictionary list as a first, cheap correction layer.
2. **LLM prompt injection** — feed the dictionary list into the Qwen system prompt (e.g. "the user's name is Pratyush — correct any phonetic near-misses of this and these other terms: [...]"). Piggybacks on the cleanup call already running.
3. Do NOT rely on the LLM's background/pretrained "knowledge" of a name for accuracy — general familiarity from pretraining is not the same as reliable in-context correction. The dictionary injection is the actual reliability mechanism.

**UX for adding entries — better than Flow's dashboard-only approach:**
- After dictation, if the user manually edits the output (correcting a name/word), offer an inline "add this correction to your dictionary?" prompt right there. Turns the user's own edits into dictionary entries with zero dashboard trips required.

---

## 5. Storage

- **`config.json`** — settings/preferences, small, rarely changes, human-readable (nice trust signal: "here's literally the file on disk").
- **SQLite (`rusqlite`)** — history/transcript log. Chosen over pure JSON/JSONL because:
  - Avoids full-file rewrite on every save (JSON blob rewriting becomes painful at scale — tens of thousands of entries).
  - Enables real querying/search/indexing over history without loading everything into memory.
  - Safe incremental writes vs. risk of corrupting a giant JSON file on a crash mid-write.
  - **Weight concern is a non-issue**: SQLite adds <1MB to the binary and a few MB of RAM — negligible next to the ~2-4GB of model weights already being shipped/run. Not the "heavy" part of the app.
- **Atomic writes** for `config.json` (write to temp file, rename) to avoid corruption on crash.
- **Export flow:** macOS has no reliable "app is being deleted" hook — don't rely on one. Instead: (a) prompt for export periodically or on quit if it's been a while since last export, and (b) make manual export a one-click, always-available menu action.

---

## 6. App-Context Awareness (Style feature foundation)

- Detect the frontmost/focused application to adjust formatting tone (casual for chat apps, formal for docs/email, code-snippet style for editors).
- **Cross-platform requirement:** needs two backend implementations behind a shared Rust trait/interface:
  - macOS: `NSWorkspace` / frontmost app bundle ID.
  - Windows: process name / window handle via Win32 API.
- Feeds directly into the Qwen system prompt as a context modifier (e.g. "user is in a chat app — keep it casual and brief" vs. "user is in a formal document — use structured paragraphs").
- This is also the underlying mechanism for the **Style** feature (user-configurable formatting presets per app) — build the detection once, expose it as user-configurable later. Not new engineering once built.

---

## 7. Snippets

**Trigger mechanism — build the deterministic path first:**
- Real-world testing (against Flow itself) showed natural-language triggering is unreliable — plain phrasing sometimes fails to trigger, while an explicit delimiter (e.g., "my email**:** my email address") works reliably.
- **V1: explicit delimiter syntax** — deterministic, easy to build, testable, predictable for a power user.
- **Layer on top (not required for V1):** semantic/LLM-based intent detection — feed the snippet library (triggers + values) into the same Qwen cleanup call, ask it to detect if the utterance represents a snippet request and splice in the value, using structured JSON output. This is likely how Flow itself resolves ambiguous natural-language triggers (evidence: colon/delimiter syntax appears to be their deterministic fallback for cases the fuzzy/semantic layer doesn't confidently resolve).
- Keep the explicit delimiter as a strong hint *within* the LLM prompt even after adding semantic detection, rather than as a separate hardcoded parser — collapses into one call per utterance rather than two pipeline stages.

---

## 8. Command Mode (highlight + voice edit)

- User highlights text on screen, speaks an instruction ("make this more formal," "make this bullet points").
- Flow: simulate copy (Cmd+C / OS equivalent) → read clipboard → send `{highlighted_text, voice_instruction}` to Qwen with an instruction-execution prompt → paste result back over the selection.
- Rust implementation: `enigo` (simulated keypresses, cross-platform) + `arboard` (clipboard access, cross-platform).
- Not urgent for MVP — build after core dictation loop is solid.

---

## 9. Transforms

- User-defined custom prompts applied to dictated/selected text (e.g. "Polish," "Prompt Engineer" presets, or user-uploaded custom prompts).
- Reuses Command Mode infrastructure entirely — no new pipeline, just a prompt template library. Cheap once §8 exists.

---

## 10. Text Injection

- **Primary: Accessibility API** (macOS `AXUIElement`) — insert at cursor via `kAXSelectedTextAttribute`, preserves user's existing clipboard contents (unlike clipboard-based injection). Requires Accessibility permission (same one Flow itself requires).
- **Fallback: clipboard + simulated paste** — universal compatibility for apps that don't implement Accessibility API correctly (some Electron apps, canvas-based editors, some terminals). Save/restore clipboard contents around the paste to avoid clobbering user's copied data. Force plain-text paste to avoid rich-text formatting quirks.
- **Windows equivalent:** UI Automation (`IAutomation`) as primary, `SendInput` simulated keystrokes as fallback — same two-tier pattern as macOS.
- **Architecture requirement:** implement as a Rust trait with per-OS backend implementations, not hardcoded platform-specific logic — this is the one component that unavoidably needs real per-OS code.

---

## 11. UX Model

- **Global hotkey + floating overlay** (confirmed direction, Flow-style) — not a menu-bar-only app.
- Streaming vs. insert-on-release for text injection: still an open decision — affects whether AX updates happen mid-utterance or a single paste happens at the end. Resolve before deep injection implementation work.

---

## 12. Feature Priority (MVP scope)

**Build first (core loop):**
- Hotkey + overlay UX
- Audio capture → VAD → Whisper STT → Qwen cleanup → text injection pipeline, cross-platform from the start
- Dictionary (custom word/name correction) — treated as core, not secondary
- SQLite history + JSON config storage
- Export-before-delete flow

**Build after core loop is solid (cheap, low-effort additions):**
- Snippets (delimiter-based V1)
- Style (per-app formatting presets)
- Transforms (custom prompt templates)
- Command Mode

**Explicitly deprioritized / skip:**
- Insights tab / usage stats — considered low-value engagement tooling, not core utility.
- Full dashboard — keep minimal, functions mainly as a homepage; not a priority build target.
- Team/multi-user/invite features — not applicable, single-user local tool.

---

## 13. Branding

- **Name: LocalFlow.** Evaluated trademark risk against Wispr Flow — "Flow" alone is a weak/generic term used broadly across unrelated products (Microsoft Power Automate, Adobe, meditation/period-tracking apps), so LocalFlow doesn't meaningfully echo Wispr's actual distinctive mark ("Wispr Flow" as a combination). Direct name-drop of "Wispr Flow" in marketing copy/taglines has been removed — the actual differentiator (local, free, private) carries the positioning without invoking a competitor's name directly.
- If competitor comparison content is wanted later, prefer a dedicated factual "LocalFlow vs. Wispr Flow" comparison page over baking the competitor's name into primary hero copy.

---

## Open decisions to resolve during build

1. Streaming (incremental) vs. insert-on-release text injection timing.
2. Whether Qwen 3B needs a deterministic backtracking pre-pass, or handles it fine with good few-shot prompting alone — test empirically on real speech first.
3. Snippet trigger phrase UX — finalize the exact delimiter syntax (colon-based, or alternative).
4. Minimum supported RAM spec (16GB recommended baseline).
