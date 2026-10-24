Engine-room kernel (P1.1+): registry.rs · hub.rs · theme.rs · glass.rs · menu.rs · tray.rs · windows.rs (Settings window, show main) · permissions.rs
· ai/ (mod · ollama · whisper · queue · log — the gateway: transcribe() + complete(), one job at a time)
· brand/ (tokens · import) · export/ (model · docx · pdf).
Rule: all DB access through hub.rs; all AI through ai/.
