use std::process::Command;

use tauri::AppHandle;

use crate::managed_agents::workforce::ResolvedWorkforceExecution;
use crate::managed_agents::KnownAcpRuntime;

pub(super) fn resolve_for_app(
    app: &AppHandle,
    pubkey: &str,
    relay_url: &str,
) -> Result<Option<ResolvedWorkforceExecution>, String> {
    crate::managed_agents::workforce::resolve_workforce_execution_for_app(
        app, pubkey, relay_url, None, None,
    )
}

pub(super) fn apply_resolved_config(
    workforce: Option<&ResolvedWorkforceExecution>,
    prompt: &mut Option<String>,
    model: &mut Option<String>,
    provider: &mut Option<String>,
) {
    let Some(workforce) = workforce else {
        return;
    };
    // Hermes is reference-only: only employee executions carry replacements.
    if let Some(resolved_prompt) = &workforce.system_prompt {
        *prompt = Some(resolved_prompt.clone());
    }
    if let Some(route) = &workforce.model {
        *model = Some(route.model.clone());
        *provider = Some(route.provider.clone());
    }
}

pub(super) fn apply_trusted_env(
    command: &mut Command,
    workforce: Option<&ResolvedWorkforceExecution>,
    runtime_meta: Option<&KnownAcpRuntime>,
) {
    let Some(workforce) = workforce else {
        return;
    };
    command.env("BUZZ_WORKFORCE_COMPANY_ID", &workforce.company_id);
    command.env("BUZZ_WORKFORCE_IDENTITY_ID", &workforce.identity_id);
    command.env(
        "BUZZ_WORKFORCE_CONTEXT_VERSION",
        workforce.context_version.to_string(),
    );
    command.env("BUZZ_WORKFORCE_CONTEXT_HASH", &workforce.context_hash);
    set_optional_env(
        command,
        "BUZZ_WORKFORCE_ROLE_VERSION",
        workforce.role_version.map(|version| version.to_string()),
    );
    set_optional_env(
        command,
        "BUZZ_WORKFORCE_ROLE_HASH",
        workforce.role_hash.clone(),
    );
    set_optional_env(
        command,
        "BUZZ_WORKFORCE_HERMES_PROFILE_REF",
        workforce.hermes_profile_ref.clone(),
    );

    // This is deliberately applied after user env: tenant-bound employee
    // routing cannot be shadowed by saved BUZZ_ACP_* values.
    if let Some(prompt) = &workforce.system_prompt {
        command.env("BUZZ_ACP_SYSTEM_PROMPT", prompt);
    }
    if let Some(route) = &workforce.model {
        command.env("BUZZ_ACP_MODEL", &route.model);
        if let Some(meta) = runtime_meta {
            for (key, value) in super::runtime_metadata_env_vars(
                meta.model_env_var,
                meta.provider_env_var,
                meta.provider_locked,
                Some(&route.model),
                Some(&route.provider),
            ) {
                command.env(key, value);
            }
        }
    }
}

fn set_optional_env(command: &mut Command, key: &str, value: Option<String>) {
    if let Some(value) = value {
        command.env(key, value);
    } else {
        command.env_remove(key);
    }
}
