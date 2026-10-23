# Open-weight model families compared for New Dawn (checked 2026-10-23; Nemotron added the same day)

_The question: which family should power the text side of New Dawn on a 16 GB M1 Pro, 100 % local, English-first with Vietnamese, with meetings and knowledge work as the jobs — and which families bring useful specialist siblings (speech, embeddings, vision/OCR, reranking, safety). Facts below come from the vendors' pages and October-2026 local-LLM guides; where a source was silent I say so. Nothing here changes the gateway design — swapping a model is a config change in `kernel/ai/models.rs`._

## 1. The scoreboard

| Family | Best model that fits 16 GB | Quality at that size | Vietnamese | Vision | Specialist siblings (open weights) | Licence | Ollama / llama.cpp | Verdict |
|---|---|---|---|---|---|---|---|---|
| **Qwen** (Alibaba) | Qwen3.5-9B (6.6 GB) · 4B (3.4 GB) | Top of the 8–10B class in Oct-2026 guides | Yes — 100+ languages claimed, Vietnamese verified in practice | Native | ASR 0.6B/1.7B, TTS, Embedding 0.6B–8B, Reranker, VL-Embedding, Guard, Coder, Omni, Image | Apache 2.0 | Day-one, mature | **Primary** |
| **Gemma 4** (Google) | E4B (~4.5–5 GB) · 4B · 2B; 12B is ~7–9 GB and tight | #2 in the same guides; "compressed 8–9B" | Strong multilingual (140 languages in Gemma 3; assume similar) | Yes; 12B also takes audio input | EmbeddingGemma, ShieldGemma (safety), MedGemma — plus audio-in on 12B | Apache 2.0 (new for Gemma 4) | Day-one | **Runner-up / second opinion** |
| **Mistral** (France) | Ministral 3 — 3B / 8B / 14B (8B ≈ 5 GB) | Solid; European-language leaning | Not highlighted; "several Asian languages" unnamed | Only in Small 4 (119B MoE — too big) | Voxtral Transcribe (speech, "mixed" licence), Voxtral TTS (**non-commercial**), OCR 4 / Embed / Codestral are **API-only** | Apache 2.0 for Ministral/Magistral/Devstral | Within days of release | Worth a benchmark for the quality tier; specialists mostly not usable offline |
| **Meta Muse Glimmer** | None — 30B dense, "under 20 GB" at 4-bit, Meta targets 24–32 GB | Vendor numbers close to Qwen3.6-27B (ahead on tool-use, behind on OSWorld/Terminal-Bench) | 100+ languages trained, "not evaluated on all" | Yes | Muse Spark (teacher, not open), Muse Image; Llama Guard family | Apache 2.0 | Yes (GGUF) | Not on this Mac; candidate if you upgrade to 32 GB |
| **DeepSeek** | R1-Distill 7B / 14B (≈4.5 / 9 GB) — these are Qwen/Llama bases fine-tuned by DeepSeek in early 2025 | Reasoning-tuned, verbose `<think>`; dated | Weak at small sizes | No | DeepSeek-OCR (document OCR, Python/vLLM-oriented) | MIT | Yes | Flagships (V3/V4, 671B–1.6T) are server-only; distills are superseded by Qwen3.5 itself |
| **Kimi** (Moonshot) | None — K2.x is 1 T total / 32B active | Frontier-class agentic model | — | K2.6 native multimodal | Kimi-VL / Kimi-Audio were 2025 research releases; no current small line | Modified MIT | K2 only on big rigs | Not a local option |
| **gpt-oss-20b** (OpenAI) | 21B / 3.6B active, MXFP4 "runs within 16 GB" (~13 GB) | Good English reasoning | English-centric | No | None | Apache 2.0 | Yes | Fills the whole memory budget — no room for Whisper or the app |
| **Nemotron** (NVIDIA) | `nemotron-3-nano:4b` (2.9 GB); the real Nano is 30B / 3.5B-active hybrid Mamba-MoE at 18–24 GB | Nano 30B is strong for agents (1M context); the 4B is a small helper | **No** — official languages are English, German, Spanish, French, Italian, Japanese | Only Nano **Omni** 30B (text, image, video, audio) | Parakeet-TDT 0.6B and Canary 1B ASR (25 European languages, no Vietnamese), NV-Embed, Nemotron Safety Guard, Cosmos (vision) | NVIDIA Open Model License (commercial allowed with conditions — read it) | Yes; Mamba hybrids need recent llama.cpp | Not for Vietnamese; its speech models are English/European-first |
| Others | Llama 4 8B (Meta licence — review for commercial), Phi-4 14B/mini (MIT), Granite 4 small (IBM, Apache), GLM-4.1V-9B | Respectable | Mixed | Some | Phi-4-multimodal (audio in), Granite speech | Mixed | Yes | Nothing beats the top two for this use |

## 2. Why Qwen stays primary

1. **It is the only family with a current, well-regarded 9B that fits with room to spare for Whisper.** The others either stop at 27–30B (Qwen3.6/3.8, Muse, Mistral Small 4, Magistral) or are older distills.
2. **Vietnamese is a first-class language**, and the 9B reads images, so scanned Vietnamese documents work in Phase 5 without an extra OCR model.
3. **The specialist line-up is the widest *and* the most usable offline**: every Qwen specialist is open-weight under Apache 2.0, most already have GGUF/ONNX builds, so they can run from Rust. Mistral's specialists are the closest rival, but Voxtral TTS is non-commercial and OCR/Embed/Codestral are API-only — which is exactly what New Dawn cannot use.
4. **Nemotron is the agent-builder's family, not the multilingual one**: the 30B-A3.5B Nano is fast and has a 1M-token context, but it is too big for 16 GB and lists no Vietnamese; its speech models (Parakeet, Canary) are the ones Meetily uses and are English/European-first.
5. **Cadence**: Qwen3.5 (Feb/Mar) → 3.6 (Apr) → 3.8 (Aug) in one year. Whatever you pick now will be replaced within months; Qwen's small sizes have shipped with every generation so far, which is the real guarantee you want.

## 3. What "many specialists" actually buys New Dawn

The count is less important than three questions per specialist: does it run from Rust (GGUF / ONNX / Core ML), is the licence commercial-safe, does it handle Vietnamese?

| Job in New Dawn | Current choice | Qwen alternative | Other alternative | Plan |
|---|---|---|---|---|
| Speech → text | Whisper large-v3-turbo via `whisper-rs` | Qwen3-ASR 1.7B (Jan 2026) — strong, but no mature Rust/GGUF path yet | Mistral Voxtral Transcribe (licence "mixed"), NVIDIA Parakeet-TDT / Canary (25 European languages, no Vietnamese) | Keep Whisper; re-check Qwen3-ASR when llama.cpp supports it |
| Summaries, actions, drafts, Q&A | Qwen3.5-9B (thinking on) / 4B | — | Gemma 4 12B / E4B, Ministral 3 14B | Bake-off in P3a (accuracy over speed); keep the winner, keep the gateway |
| Embeddings | `bge-m3` | Qwen3-Embedding 0.6B (GGUF exists) | EmbeddingGemma | Benchmark all three on Vietnamese + English notes in P5.2 |
| Reranking (later) | — | Qwen3-Reranker 0.6B | — | Adopt in P5.2 if retrieval quality needs it |
| Reading scanned PDFs | Qwen3.5-9B vision | — | DeepSeek-OCR (Python), Mistral OCR (API-only) | Use the 9B's vision; no extra model |
| Safety filter (only if you ever share the app) | — | Qwen3Guard 0.6B | ShieldGemma, Llama Guard | Not needed for a single-user local app |
| Speaker labels | `sherpa-onnx` diarization | — | — | Phase 2 of speech |

## 4. One caveat worth saying out loud

All of these are open weights you run offline, so no data leaves the Mac whichever family you choose. Some organisations still care about a model's provenance when they see it in a vendor's architecture slide; if that ever matters for a partner, the gateway lets you swap the text model for Gemma 4 or Ministral in a config change and keep everything else.

## 5. Decision

- **Primary:** Qwen3.5-9B (quality) + Qwen3.5-4B (fast), as already planned.
- **Benchmark in P3a (accuracy over speed):** Gemma 4 12B, Gemma 4 E4B, Ministral 3 14B and Qwen3.5-9B (thinking on, Q4 and Q6_K) on the same meeting transcript, same prompts, scored on summary accuracy, action-item recall, Vietnamese handling and tokens/second. Record the result in `DECISIONS.md`.
- **Speech stays Whisper.** Embeddings decided in P5.2.
- 27B on 16 GB is settled: impossible at 4-bit (16–19 GB of weights), not worth it at 2–3-bit (see ai-engine.md §10). Re-run this comparison when you change Macs (32 GB+ unlocks Qwen3.8-27B, Qwen3.6-27B/35B-A3B and Muse Glimmer).

Sources: ollama.com/library/nemotron-3-nano · Wikipedia "Nemotron" · huggingface.co/Qwen collections · ollama.com/library/qwen3.6 · GIGAZINE on Qwen3.5 Small (Mar 2026) · DataCamp on Muse Glimmer · aizolo "Mistral AI Models 2026" · Second Talent "Every Kimi model (Sep 2026)" · dev.to "Run DeepSeek locally in 2026" · atomic.chat "Best local LLM for 16 GB Mac (2026)" · benchlm.ai "Best local LLMs (Oct 2026)".
