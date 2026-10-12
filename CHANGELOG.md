# Changelog

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
