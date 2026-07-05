// M4+: whisper-rs integration.
//
// Two backends:
//   - Local  : whisper.cpp via whisper-rs (feature "stt", needs libclang-dev).
//   - Remote : POST 16kHz mono f32 samples as JSON to {remote_url}/inference,
//              expect { "text": "..." }. ponytail: send WAV instead; add when
//              a real whisper.cpp HTTP server is decided.
//
// WhisperEngine is Send + Sync (enum of String / Arc<WhisperContext>) so it can
// live behind an Arc in AppState.

/// One engine covers either backend; chosen at construction.
pub enum WhisperEngine {
    Remote {
        url: String,
        language: String,
    },
    #[cfg(feature = "stt")]
    Local {
        ctx: std::sync::Arc<whisper_rs::WhisperContext>,
        language: String,
        n_threads: i32,
    },
}

impl WhisperEngine {
    /// `remote_url` base when mode == "remote".
    pub fn new_remote(remote_url: &str, language: &str) -> Self {
        WhisperEngine::Remote {
            url: remote_url.trim_end_matches('/').to_string(),
            language: language.to_string(),
        }
    }

    /// Load whisper.cpp model. Only available with `stt` feature.
    #[cfg(feature = "stt")]
    pub fn new_local(model_path: &str, _gpu: bool, language: &str) -> Result<Self, String> {
        if model_path.is_empty() {
            return Err("stt.model_path is empty — set path to a ggml .bin model".to_string());
        }
        let params = whisper_rs::WhisperContextParameters::default();
        let ctx = whisper_rs::WhisperContext::new_with_params(model_path, params)
            .map_err(|e| format!("whisper context load failed: {e}"))?;
        Ok(WhisperEngine::Local {
            ctx: std::sync::Arc::new(ctx),
            language: language.to_string(),
            // 8 threads is a sane default on most desktops; ponytail: num_cpus.
            n_threads: 8,
        })
    }

    #[cfg(not(feature = "stt"))]
    pub fn new_local(_model_path: &str, _gpu: bool, _language: &str) -> Result<Self, String> {
        Err("in-process STT not compiled in (rebuild with --features stt)".to_string())
    }

    /// Transcribe 16kHz mono f32 samples → text. Async (remote does HTTP,
    /// local offloads to spawn_blocking so the runtime isn't blocked).
    pub async fn transcribe(&self, samples: &[f32]) -> Result<String, String> {
        match self {
            WhisperEngine::Remote { url, language } => {
                self.transcribe_remote(url, language, samples).await
            }
            #[cfg(feature = "stt")]
            WhisperEngine::Local {
                ctx,
                language,
                n_threads,
            } => {
                let ctx = ctx.clone();
                let lang = language.clone();
                let n = *n_threads;
                let samples = samples.to_vec();
                tokio::task::spawn_blocking(move || {
                    Self::transcribe_local(&ctx, &lang, n, &samples)
                })
                .await
                .map_err(|e| format!("whisper worker panicked: {e}"))?
            }
        }
    }

    #[cfg(feature = "stt")]
    fn transcribe_local(
        ctx: &whisper_rs::WhisperContext,
        language: &str,
        n_threads: i32,
        samples: &[f32],
    ) -> Result<String, String> {
        // NOTE: whisper-rs 0.14 point releases shift exact method names
        // (create_state vs new_state, full_n_segments return type). Adjust
        // shapes here when building with --features stt. Can't verify without
        // libclang-dev on this host.
        let mut state = ctx
            .create_state()
            .map_err(|e| format!("whisper create_state failed: {e}"))?;

        let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy {
            best_of: 1,
        });
        params.set_n_threads(n_threads);
        params.set_translate(false);
        Self::set_language(&mut params, language);

        state
            .full(params, samples)
            .map_err(|e| format!("whisper full() failed: {e}"))?;

        let n_segments = state
            .full_n_segments()
            .map_err(|e| format!("whisper full_n_segments failed: {e}"))?;
        let mut text = String::new();
        for i in 0..n_segments {
            match state.full_get_segment_text(i) {
                Ok(s) => text.push_str(&s),
                Err(e) => eprintln!("stt: segment {i} text err: {e}"),
            }
        }
        Ok(text.trim().to_string())
    }

    #[cfg(feature = "stt")]
    fn set_language(params: &mut whisper_rs::FullParams, language: &str) {
        // whisper-rs 0.14 exposes WhisperLanguage enum; map common codes.
        use whisper_rs::WhisperLanguage as WL;
        let lang = match language.to_ascii_lowercase().as_str() {
            "auto" | "" => WL::Auto,
            "en" => WL::En,
            "id" => WL::Id,
            _ => WL::Auto,
        };
        params.set_language(lang);
    }

    async fn transcribe_remote(
        &self,
        url: &str,
        language: &str,
        samples: &[f32],
    ) -> Result<String, String> {
        let endpoint = format!("{url}/inference");
        let body = serde_json::json!({
            "audio": samples,
            "sample_rate": 16000u32,
            "language": language,
        });
        let client = reqwest::Client::new();
        let res = client
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("remote STT request failed: {e}"))?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(format!("remote STT {status}: {text}"));
        }
        let v: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("remote STT bad json: {e}"))?;
        v["text"]
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| "remote STT: missing \"text\" field".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_engine_constructs() {
        let eng = WhisperEngine::new_remote("http://example.invalid", "auto");
        assert!(matches!(eng, WhisperEngine::Remote { .. }));
    }
}