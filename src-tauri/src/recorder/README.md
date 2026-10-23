Meeting recorder (P3c): session.rs · mic.rs (cpal) · system_tap.rs (cidre Core Audio tap) · mixer.rs · vad.rs (silero) · worker.rs (live Whisper) · saver.rs (incremental WAV).
Rule: capture callbacks only push into BOUNDED channels; never block the UI thread.
