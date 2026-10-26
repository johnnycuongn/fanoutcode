//! Provider resolution for the headless helpers and LiveSpec notes (§7.4): Fanout Code's `model_provider` setting
//! picks the kind; models, efforts, agent counts and Jev switches stay in LiveSpec's own settings file,
//! `<app_config>/livespec.json` (A19). Written against livespec-core's API (LiveSpec's shell file was not available
//! to copy). Keys come from the keychain and never cross IPC.

use fanoutcode_core::settings::{keys, resolve as resolve_setting, ModelProvider, Scope};
use fanoutcode_core::shell_path;
use fanoutcode_core::{db, Error, Result};
use livespec_core::jev::key::pick_key;
use livespec_core::jev::Jev;
use livespec_core::llm::{AnthropicApi, ClaudeCli, OpenAiCompat, Provider, Secret};
use livespec_core::local::{catalog, LocalPaths, LocalServer};
use livespec_core::settings::{ClaudeAuth, ProviderSettings, Settings, Tier};
use livespec_core::shell_env::find_executable;
use tauri::{AppHandle, Manager};

use crate::secrets;
use crate::state::AppState;

/// Keychain accounts (service `app.fanoutcode.desktop`).
pub const ANTHROPIC_KEY: &str = "anthropic-api-key";
pub const TYPESAFE_KEY: &str = "typesafe-api-key";

/// LiveSpec's settings file, its provider kind replaced by Fanout Code's `model_provider` when they differ.
pub fn settings(app: &AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let mut s = Settings::load(&state.config_dir.join("livespec.json"));
    let kind = {
        let conn = state.db.lock().expect("db lock");
        let values = db::settings::load(&conn, &Scope::app()).unwrap_or_default();
        resolve_setting::<keys::ModelProvider>(&values, &Scope::app())
    };
    s.provider = match (kind, s.provider) {
        (ModelProvider::ClaudeCli, p @ ProviderSettings::ClaudeCli { .. }) => p,
        (ModelProvider::Api, p @ ProviderSettings::AnthropicApi { .. }) => p,
        (ModelProvider::Local, p @ ProviderSettings::Local { .. }) => p,
        (ModelProvider::ClaudeCli, _) => Settings::default().provider,
        (ModelProvider::Api, _) => ProviderSettings::AnthropicApi {
            fast_model: "claude-haiku-5-5".into(),
            deep_model: "claude-sonnet-5-5".into(),
            fast_effort: None,
            deep_effort: None,
        },
        (ModelProvider::Local, _) => ProviderSettings::Local {
            model_id: catalog::CATALOG[0].id.into(),
        },
    };
    s
}

/// A keychain read, off the async threads (it may block on a prompt).
async fn keychain(account: &'static str) -> Option<String> {
    tokio::task::spawn_blocking(move || secrets::read(account))
        .await
        .ok()
        .and_then(|r| r.ok().flatten())
        .map(|s| s.expose().to_owned())
}

/// The provider for `tier`, ready to stream.
pub async fn resolve(app: &AppHandle, tier: Tier) -> Result<Provider> {
    let http = app.state::<AppState>().http.clone();
    match settings(app).provider {
        ProviderSettings::ClaudeCli {
            binary_path,
            fast_model,
            deep_model,
            fast_effort,
            deep_effort,
            auth,
        } => {
            let path_env = tokio::task::spawn_blocking(shell_path::path).await.ok();
            let binary = match binary_path.filter(|b| !b.trim().is_empty()) {
                Some(b) => b.into(),
                None => find_executable("claude", path_env.as_deref()).ok_or_else(|| {
                    Error::Precondition("Install Claude Code, or choose another model provider in Settings.".into())
                })?,
            };
            let (model, effort) = match tier {
                Tier::Fast => (fast_model, fast_effort),
                Tier::Deep => (deep_model, deep_effort),
            };
            let mut cli = ClaudeCli::new(binary, model, path_env);
            cli.effort = effort;
            if auth == ClaudeAuth::ApiKey {
                let key = keychain(ANTHROPIC_KEY)
                    .await
                    .ok_or_else(|| Error::Precondition("Add your Anthropic API key in Settings.".into()))?;
                cli.api_key = Some(Secret::new(key));
            }
            Ok(Provider::ClaudeCli(cli))
        }
        ProviderSettings::AnthropicApi {
            fast_model,
            deep_model,
            fast_effort,
            deep_effort,
        } => {
            let key = keychain(ANTHROPIC_KEY)
                .await
                .ok_or_else(|| Error::Precondition("Add your Anthropic API key in Settings.".into()))?;
            let (model, effort) = match tier {
                Tier::Fast => (fast_model, fast_effort),
                Tier::Deep => (deep_model, deep_effort),
            };
            Ok(Provider::AnthropicApi(AnthropicApi {
                api_key: Secret::new(key),
                model,
                effort,
                http,
            }))
        }
        ProviderSettings::Local { model_id } => ensure_local(app, &model_id, http).await,
    }
}

/// The managed llama.cpp server for `model_id`, started once and reused while it lives.
pub async fn ensure_local(app: &AppHandle, model_id: &str, http: reqwest::Client) -> Result<Provider> {
    let state = app.state::<AppState>();
    let spec =
        catalog::find(model_id).ok_or_else(|| Error::Precondition(format!("Unknown local model {model_id}.")))?;
    let mut server = state.local_server.lock().await;
    let alive = server.as_mut().is_some_and(|s| s.model_id == model_id && s.is_alive());
    if !alive {
        if let Some(old) = server.take() {
            old.stop().await;
        }
        let paths = LocalPaths::new(&state.data_dir);
        let started = LocalServer::start(&http, &paths, spec.id, &paths.model_path(spec), spec.context_tokens).await?;
        *server = Some(started);
    }
    let s = server.as_ref().expect("started above");
    Ok(Provider::OpenAiCompat(OpenAiCompat {
        base_url: s.base_url(),
        model: s.model_id.clone(),
        context_tokens: s.context_tokens,
        http,
    }))
}

async fn typesafe_key() -> Option<String> {
    let keychain = keychain(TYPESAFE_KEY).await;
    pick_key(keychain, std::env::var("TYPESAFE_API_KEY").ok(), || None).map(|(k, _)| k)
}

/// Jev, when a TypeSafe key exists (keychain, then `TYPESAFE_API_KEY`).
pub async fn jev(app: &AppHandle) -> Option<Jev> {
    let model = settings(app).jev.model;
    let http = app.state::<AppState>().http.clone();
    typesafe_key().await.map(|k| Jev::new(k, model, http))
}

pub async fn typesafe_key_present(_app: &AppHandle) -> bool {
    typesafe_key().await.is_some()
}
