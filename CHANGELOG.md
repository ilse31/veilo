# Changelog

## v1.5.6

### Added
- **Auto-update.** The app checks GitHub Releases on launch and offers to install newer versions in place.

### Changed
- **New app icon.** Regenerated ghost-V logo across all platforms.
- Updated `@tauri-apps/api` to match the Tauri 2.12.7 runtime.

## v1.5.5

### Fixed
- Copilot code blocks now wrap long lines instead of requiring horizontal scrolling in narrow windows.

## v1.5.4

### Improved
- **Faster Copilot answers.** The delay between the other party finishing a sentence and the AI reply starting is roughly halved: Deepgram endpointing is 0.5s instead of 1s, the orchestrator debounce is 0.4s instead of 1.2s, and the HTTP connection to the AI endpoint is kept warm between suggestions.

## v1.5.3

### Added
- **Draggable Copilot card.** Drag the floating Copilot window by its header to reposition it. The position is remembered across sessions and restored on the next launch.
- **Endpoint errors shown in the card.** If the AI endpoint fails (HTTP error or unreachable), the Copilot card now shows a visible warning instead of silently producing no answer.

### Fixed
- Speech utterances were never finalized while the other party went quiet — WASAPI loopback delivers no audio callbacks during silence, so the STT backend never heard the trailing silence it needs. A watchdog now feeds real-time silence during capture gaps.
- AI endpoint URLs ending in `/v1` (or not) now resolve correctly for Copilot suggestions, translation, and Whisper transcription.

## v1.5.2

### Added
- Choose EN for English or JP for Japanese in romaji on the Copilot card. The choice applies to upcoming transcript suggestions. Click the active button again to return to automatic language selection.

## v1.5.1

### Fixed
- The Copilot card now appears when starting a session in the installed app.

## v1.5.0

### Added
- **Ask AI in Copilot.** Type a question directly in the Copilot card. The question field is separate from persistent instructions such as a preferred programming language.
- **Screen preview.** Choose a monitor, inspect its screenshot, optionally add a question, and confirm before sending it to the AI. Canceling discards the preview.
- **Image delivery test.** Settings sends a known image through the selected model and reports whether the provider route actually passed it through. A successful test marks the model as image-capable after saving.

### Improved
- The model list reads 9router-style `capabilities.vision` metadata when available.
- OpenAI-compatible endpoint URLs work whether the configured base ends in `/v1` or not.
- The Copilot card can be closed after a standalone question or screen reading without an audio session.

## v1.4.0

### Added
- **Read my screen.** Use Ctrl+Alt+S to capture the monitor under the cursor, or Ctrl+Alt+M to choose a monitor. A vision-capable model reads the screenshot and streams its answer into the Copilot card.
- **Vision model selection.** The model picker shows image support reported by the provider or inferred from the model name, with a manual override for models whose capability is unknown.
- **Screen context.** Optionally include the latest Copilot transcript with a screenshot. Custom instructions entered in the Copilot card, including a preferred programming language, also apply to the next screen capture.

### Fixed
- The Copilot instruction input requests keyboard focus when selected and the card stays open while it is being edited.
- Empty image responses and interrupted streams show a visible message instead of leaving an unfinished answer.

### Notes
- Screen reading requires a provider route that passes image inputs to the selected model. On 9router, Antigravity Claude routes may report vision support yet omit the image; an Antigravity Gemini vision route worked in local testing.
- macOS requires Screen Recording permission before capture.

## v1.3.0

### Added
- **Deepgram streaming STT.** New speech-to-text backend over WebSocket (nova-3) with live partial results and automatic language detection. It has its own API key field, separate from the AI chat key.
- **Microphone input.** Choose a microphone or input device as the audio source in Settings → Copilot.
- **Combined audio mode.** Capture system audio and your microphone together. The other party's speech drives the AI; your own voice is transcribed for display only (labelled *Saya*), so you can check how you sound.
- **Chat-style Q&A.** Every spoken question gets its own answer in a scrolling thread instead of one merged reply.
- **Live transcript panel.** See what the speech recognizer heard as you speak, with *Lawan* / *Saya* labels. Toggle it with the 🎙 button.
- **Always-Indonesian toggle (ID).** Translates transcripts and forces answers into Indonesian whatever language the other party speaks. Can be switched live from the Copilot card.
- **Markdown session summaries.** Summaries in the session history now render headings, lists, code and tables.

### Fixed
- macOS system-audio loopback dropped two thirds of the audio when a Bluetooth headset switched to hands-free (16 kHz) mode, which made speech unrecognizable. The real sample rate is now measured and followed.
- Starting a Copilot session with a non-HTTP AI endpoint (for example a pasted `wss://` URL) now fails immediately with a clear message instead of silently producing no answers.
- Deepgram no longer re-triggers the AI mid-sentence: phrases are joined into one utterance before answering.

### Notes
- Deepgram `language=multi` covers about ten languages (English, Japanese, Spanish, French, German, Hindi, Russian, Portuguese, Italian, Dutch). For Indonesian, Korean or Chinese pick that language explicitly, or use the Whisper backend.
- Combined mode and the Indonesian toggle make extra requests (a second STT stream, one translation call per utterance), which increases API usage.
- macOS asks for microphone and system-audio recording permission on first use. When running from a terminal in development, grant it to the terminal or IDE.
- Windows Speech Recognition is not supported in combined audio mode.

## v1.0.0

First release under the Veilo name: new branding, automatic import of existing noscreen settings and data, and automated multi-platform release builds.
