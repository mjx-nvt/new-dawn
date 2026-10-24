# Bundled models

`ggml-silero-v5.1.2.bin` — Silero VAD v5.1.2 converted for whisper.cpp's built-in VAD.
- Source: https://huggingface.co/ggml-org/whisper-vad/resolve/9ffd54a1e1ee413ddf265af9913beaf518d1639b/ggml-silero-v5.1.2.bin
- Size: 885 098 bytes · SHA-256: 29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf (checked by a unit test in `kernel/ai/models.rs`)
- Licence: MIT (Silero VAD, https://github.com/snakers4/silero-vad)

Shipped inside the app as `Contents/Resources/models/` (see `tauri.conf.json` → `bundle.resources`), so users never download it. Whisper models are too large to bundle and are downloaded in Settings → Models (D-014).
