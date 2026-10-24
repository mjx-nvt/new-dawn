Meeting recorder (P2.2): session.rs · mic.rs (cpal) · system_tap.rs (cidre Core Audio tap) · mixer.rs (rubato) · saver.rs (incremental 16 kHz mono WAV).
No live transcription in v1 — transcription is a batch job after Stop (modules/meetings/transcribe.rs).
Rule: capture callbacks only push into BOUNDED channels; never block the UI thread.
