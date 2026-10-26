Meeting recorder (P2.2): session.rs · mic.rs (cpal) · system_tap.rs (cidre Core Audio tap) · mixer.rs (rubato) · saver.rs (incremental 16 kHz mono WAV).
Transcription is a batch job after Stop (modules/meetings/transcribe.rs) — except a Live session (D-041): the mixer offers each 600 ms window to kernel/ai/live.rs over a bounded channel (try_send; a full channel drops the window), which writes Vietnamese lines and their English as they arrive.
Rule: capture callbacks only push into BOUNDED channels; never block the UI thread.
