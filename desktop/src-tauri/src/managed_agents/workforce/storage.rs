use std::fs;
use std::path::{Path, PathBuf};

use tauri::AppHandle;

use super::{CompanyContext, WorkforceStore};

const WORKFORCE_FILE: &str = "workforce.json";

pub fn workforce_base_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let path = super::super::managed_agents_base_dir(app)?.join("workforce");
    fs::create_dir_all(path.join("contexts"))
        .map_err(|error| format!("failed to create workforce directories: {error}"))?;
    Ok(path)
}

pub fn load_workforce(app: &AppHandle) -> Result<WorkforceStore, String> {
    load_workforce_from_path(&workforce_base_dir(app)?.join(WORKFORCE_FILE))
}

pub fn save_workforce(app: &AppHandle, store: &WorkforceStore) -> Result<(), String> {
    save_workforce_to_path(&workforce_base_dir(app)?.join(WORKFORCE_FILE), store)
}

pub fn load_company_context(app: &AppHandle, company_id: &str) -> Result<CompanyContext, String> {
    load_company_context_from_path(&company_context_path(app, company_id)?, company_id)
}

pub fn save_company_context(app: &AppHandle, context: &CompanyContext) -> Result<(), String> {
    save_company_context_to_path(&company_context_path(app, &context.company_id)?, context)
}

fn company_context_path(app: &AppHandle, company_id: &str) -> Result<PathBuf, String> {
    if !safe_id(company_id) {
        return Err(format!("invalid company id {company_id:?}"));
    }
    Ok(workforce_base_dir(app)?
        .join("contexts")
        .join(format!("{company_id}.json")))
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(crate) fn load_workforce_from_path(path: &Path) -> Result<WorkforceStore, String> {
    if !path.exists() {
        return Ok(WorkforceStore::default());
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read workforce store {}: {error}", path.display()))?;
    let store: WorkforceStore = serde_json::from_slice(&bytes).map_err(|error| {
        super::super::storage::backup_invalid_store(path);
        format!("failed to parse workforce store (preserved as .invalid): {error}")
    })?;
    store
        .validate()
        .map_err(|error| format!("invalid workforce store: {error}"))?;
    Ok(store)
}

pub(crate) fn save_workforce_to_path(path: &Path, store: &WorkforceStore) -> Result<(), String> {
    store
        .validate()
        .map_err(|error| format!("invalid workforce store: {error}"))?;
    ensure_parent(path)?;
    let payload = serde_json::to_vec_pretty(store)
        .map_err(|error| format!("failed to serialize workforce store: {error}"))?;
    super::super::storage::atomic_write_json_restricted(path, &payload)
}

pub(crate) fn load_company_context_from_path(
    path: &Path,
    expected_company_id: &str,
) -> Result<CompanyContext, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read company context {}: {error}", path.display()))?;
    let context: CompanyContext = serde_json::from_slice(&bytes).map_err(|error| {
        super::super::storage::backup_invalid_store(path);
        format!("failed to parse company context (preserved as .invalid): {error}")
    })?;
    validate_context(&context)?;
    if context.company_id != expected_company_id {
        return Err(format!(
            "company context mismatch: expected {expected_company_id:?}, found {:?}",
            context.company_id
        ));
    }
    Ok(context)
}

pub(crate) fn save_company_context_to_path(
    path: &Path,
    context: &CompanyContext,
) -> Result<(), String> {
    validate_context(context)?;
    ensure_parent(path)?;
    let payload = serde_json::to_vec_pretty(context)
        .map_err(|error| format!("failed to serialize company context: {error}"))?;
    super::super::storage::atomic_write_json_restricted(path, &payload)
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("path {} has no parent", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))
}

fn validate_context(context: &CompanyContext) -> Result<(), String> {
    if !safe_id(&context.company_id) {
        return Err(format!("invalid company id {:?}", context.company_id));
    }
    let mut fact_ids = std::collections::HashSet::new();
    for fact in &context.facts {
        if !safe_id(&fact.fact_id) {
            return Err(format!("invalid fact id {:?}", fact.fact_id));
        }
        if !fact_ids.insert(fact.fact_id.as_str()) {
            return Err(format!("duplicate fact id {:?}", fact.fact_id));
        }
        if fact.confidence_basis_points > 10_000 {
            return Err(format!(
                "fact {:?} confidence exceeds 10000 basis points",
                fact.fact_id
            ));
        }
    }
    Ok(())
}
