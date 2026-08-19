use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    catalog_version: u32,
    roles: Vec<CatalogRole>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogRole {
    identity_id: String,
    display_name: String,
    persona_id: String,
    role_id: String,
    department: String,
    use_when: String,
    required_inputs: Vec<String>,
    expected_outputs: Vec<String>,
    general_prompt: String,
    approval_boundary: String,
    default_model: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompanyRegistry {
    companies: Vec<Company>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Company {
    company_id: String,
    display_name: String,
    relay_url: Option<String>,
    context_ref: String,
    #[serde(default = "default_language")]
    default_language: String,
    #[serde(default = "active_rollout")]
    rollout_state: String,
}

fn default_language() -> String {
    "en".into()
}
fn active_rollout() -> String {
    "active".into()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanSummary {
    mode: &'static str,
    migrated_identities: usize,
    removed_duplicate_records: usize,
    memberships: usize,
    conflicts: Vec<String>,
    changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Receipt {
    schema_version: u32,
    created_at: String,
    managed_store: PathBuf,
    workforce_store: PathBuf,
    managed_backup: PathBuf,
    workforce_backup: Option<PathBuf>,
    managed_before_sha256: String,
    workforce_before_sha256: Option<String>,
    managed_after_sha256: String,
    workforce_after_sha256: String,
}

#[derive(Debug)]
struct Options {
    managed_store: PathBuf,
    workforce_store: PathBuf,
    catalog: PathBuf,
    company_registry: PathBuf,
    receipt: Option<PathBuf>,
    apply: bool,
}

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("buzz-workforce-migrate: {error:#}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<()> {
    if let Some(index) = args.iter().position(|arg| arg == "--rollback") {
        let receipt = args
            .get(index + 1)
            .ok_or_else(|| anyhow!("--rollback requires a receipt path"))?;
        return rollback(Path::new(receipt));
    }
    let options = parse_options(&args)?;
    let managed_before = fs::read(&options.managed_store)
        .with_context(|| format!("read {}", options.managed_store.display()))?;
    let workforce_before = fs::read(&options.workforce_store).ok();
    let catalog: Catalog = read_json(&options.catalog)?;
    let registry: CompanyRegistry = read_json(&options.company_registry)?;
    let mut managed: Vec<Value> = serde_json::from_slice(&managed_before)?;
    let mut workforce = match &workforce_before {
        Some(bytes) => serde_json::from_slice(bytes)?,
        None => json!({"schemaVersion":1,"revision":0,"identities":[],"communities":[]}),
    };
    let before_managed_value = serde_json::to_value(&managed)?;
    let before_workforce_value = workforce.clone();
    let mut conflicts = Vec::new();
    let stats = migrate(
        &mut managed,
        &mut workforce,
        &catalog,
        &registry,
        &mut conflicts,
    )?;
    let changed_without_revision = before_managed_value != serde_json::to_value(&managed)?
        || before_workforce_value != workforce;
    if changed_without_revision {
        let revision = workforce["revision"].as_u64().unwrap_or(0);
        workforce["revision"] = json!(revision.saturating_add(1));
    }
    let changed = changed_without_revision;
    let summary = PlanSummary {
        mode: if options.apply { "apply" } else { "dry-run" },
        migrated_identities: stats.identities,
        removed_duplicate_records: stats.removed,
        memberships: stats.memberships,
        conflicts,
        changed,
    };
    println!("{}", serde_json::to_string_pretty(&summary)?);
    if !summary.conflicts.is_empty() {
        bail!("migration refused because conflicts were found");
    }
    if !options.apply || !changed {
        return Ok(());
    }

    let receipt_path = options
        .receipt
        .unwrap_or_else(|| default_receipt_path(&options.managed_store));
    let stamp = Utc::now().format("%Y%m%dT%H%M%S%.fZ").to_string();
    let managed_backup = backup_path(&options.managed_store, &stamp);
    let workforce_backup = workforce_before
        .as_ref()
        .map(|_| backup_path(&options.workforce_store, &stamp));
    copy_new(&options.managed_store, &managed_backup)?;
    if let Some(path) = &workforce_backup {
        copy_new(&options.workforce_store, path)?;
    }
    let managed_after = serde_json::to_vec_pretty(&managed)?;
    let workforce_after = serde_json::to_vec_pretty(&workforce)?;
    atomic_write(&options.managed_store, &managed_after)?;
    atomic_write(&options.workforce_store, &workforce_after)?;
    let receipt = Receipt {
        schema_version: 1,
        created_at: Utc::now().to_rfc3339(),
        managed_store: options.managed_store,
        workforce_store: options.workforce_store,
        managed_backup,
        workforce_backup,
        managed_before_sha256: sha256(&managed_before),
        workforce_before_sha256: workforce_before.as_deref().map(sha256),
        managed_after_sha256: sha256(&managed_after),
        workforce_after_sha256: sha256(&workforce_after),
    };
    atomic_write(&receipt_path, &serde_json::to_vec_pretty(&receipt)?)?;
    eprintln!("receipt={}", receipt_path.display());
    Ok(())
}

#[derive(Default)]
struct Stats {
    identities: usize,
    removed: usize,
    memberships: usize,
}

fn migrate(
    managed: &mut Vec<Value>,
    workforce: &mut Value,
    catalog: &Catalog,
    registry: &CompanyRegistry,
    conflicts: &mut Vec<String>,
) -> Result<Stats> {
    ensure_workforce_shape(workforce)?;
    let relay_companies: BTreeMap<String, &Company> = registry
        .companies
        .iter()
        .filter_map(|company| {
            company
                .relay_url
                .as_deref()
                .map(|relay| (normalize_relay(relay), company))
        })
        .collect();
    let mut remove = BTreeSet::new();
    let mut stats = Stats::default();

    for role in &catalog.roles {
        let indices: Vec<usize> = managed
            .iter()
            .enumerate()
            .filter(|(_, record)| string(record, "persona_id") == Some(role.persona_id.as_str()))
            .map(|(index, _)| index)
            .collect();
        if indices.is_empty() {
            continue;
        }
        let records: Vec<&Value> = indices.iter().map(|index| &managed[*index]).collect();
        let conflict_count = conflicts.len();
        validate_group(role, &records, &relay_companies, conflicts);
        if conflicts.len() != conflict_count {
            continue;
        }
        let pubkey = required_string(records[0], "pubkey")?.to_lowercase();
        upsert_identity(workforce, role, &pubkey, catalog.catalog_version)?;
        stats.identities += 1;
        for record in &records {
            let relay = required_string(record, "relay_url")?;
            let company = relay_companies
                .get(&normalize_relay(relay))
                .ok_or_else(|| anyhow!("relay disappeared during validated migration"))?;
            ensure_community(workforce, company)?;
            if upsert_membership(workforce, &company.company_id, &role.identity_id)? {
                stats.memberships += 1;
            }
            upsert_company_route(workforce, company, role, record)?;
        }
        for index in indices.iter().skip(1) {
            remove.insert(*index);
        }
    }
    stats.removed = remove.len();
    for index in remove.into_iter().rev() {
        managed.remove(index);
    }
    Ok(stats)
}

fn validate_group(
    role: &CatalogRole,
    records: &[&Value],
    companies: &BTreeMap<String, &Company>,
    conflicts: &mut Vec<String>,
) {
    for field in [
        "pubkey",
        "persona_id",
        "system_prompt",
        "avatar_url",
        "name",
    ] {
        let values: BTreeSet<String> = records
            .iter()
            .map(|record| record.get(field).unwrap_or(&Value::Null).to_string())
            .collect();
        if values.len() != 1 {
            conflicts.push(format!("{} has conflicting {field}", role.display_name));
        }
    }
    if records.iter().any(|record| {
        record
            .get("runtime_pid")
            .is_some_and(|value| !value.is_null())
    }) {
        conflicts.push(format!(
            "{} has a running process receipt; stop it before migration",
            role.display_name
        ));
    }
    for record in records {
        match string(record, "relay_url") {
            Some(relay) if companies.contains_key(&normalize_relay(relay)) => {}
            Some(relay) => conflicts.push(format!(
                "{} uses unsupported relay {relay}",
                role.display_name
            )),
            None => conflicts.push(format!("{} has no relay_url", role.display_name)),
        }
    }
}

fn upsert_identity(
    workforce: &mut Value,
    role: &CatalogRole,
    pubkey: &str,
    catalog_version: u32,
) -> Result<()> {
    let identities = array_mut(workforce, "identities")?;
    if let Some(existing) = identities
        .iter()
        .find(|identity| string(identity, "identityId") == Some(&role.identity_id))
    {
        if string(existing, "pubkey") != Some(pubkey) {
            bail!(
                "existing workforce identity {} has another pubkey",
                role.identity_id
            );
        }
        return Ok(());
    }
    identities.push(json!({
        "identityId": role.identity_id,
        "pubkey": pubkey,
        "kind": "employee",
        "role": {
            "roleId": role.role_id,
            "department": role.department,
            "useWhen": role.use_when,
            "requiredInputs": role.required_inputs,
            "expectedOutputs": role.expected_outputs,
            "generalPrompt": role.general_prompt,
            "defaultModel": role.default_model,
            "approvalBoundary": role.approval_boundary,
            "catalogVersion": catalog_version,
            "lastVerifiedAt": Utc::now().to_rfc3339(),
        }
    }));
    Ok(())
}

fn ensure_community(workforce: &mut Value, company: &Company) -> Result<()> {
    let communities = array_mut(workforce, "communities")?;
    if communities
        .iter()
        .any(|item| string(item, "companyId") == Some(&company.company_id))
    {
        return Ok(());
    }
    communities.push(json!({
        "companyId": company.company_id,
        "displayName": company.display_name,
        "relayUrl": company.relay_url.as_deref(),
        "contextRef": company.context_ref,
        "defaultLanguage": company.default_language,
        "owners": [], "approvers": [],
        "rolloutState": company.rollout_state,
        "modelPolicy": {"roleOverrides": {}},
        "memberships": []
    }));
    Ok(())
}

fn upsert_membership(workforce: &mut Value, company_id: &str, identity_id: &str) -> Result<bool> {
    let community = community_mut(workforce, company_id)?;
    let memberships = array_mut(community, "memberships")?;
    if memberships
        .iter()
        .any(|item| string(item, "identityId") == Some(identity_id))
    {
        return Ok(false);
    }
    memberships.push(json!({
        "identityId": identity_id,
        "enabled": false,
        "startOnAppLaunch": false
    }));
    Ok(true)
}

fn upsert_company_route(
    workforce: &mut Value,
    company: &Company,
    role: &CatalogRole,
    record: &Value,
) -> Result<()> {
    let route = json!({
        "provider": string(record, "provider").unwrap_or_else(|| role.default_model["provider"].as_str().unwrap_or("")),
        "model": string(record, "model").unwrap_or_else(|| role.default_model["model"].as_str().unwrap_or("")),
        "purposeLabel": role.default_model["purposeLabel"],
        "approved": role.default_model["approved"],
        "health": role.default_model["health"],
        "lastHealthCheckAt": role.default_model["lastHealthCheckAt"]
    });
    let community = community_mut(workforce, &company.company_id)?;
    let overrides = community
        .get_mut("modelPolicy")
        .and_then(Value::as_object_mut)
        .and_then(|policy| policy.get_mut("roleOverrides"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| anyhow!("community modelPolicy.roleOverrides must be an object"))?;
    overrides.insert(role.role_id.clone(), route);
    Ok(())
}

fn ensure_workforce_shape(value: &mut Value) -> Result<()> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| anyhow!("workforce store must be an object"))?;
    object.entry("schemaVersion").or_insert(json!(1));
    object.entry("revision").or_insert(json!(0));
    object.entry("identities").or_insert(json!([]));
    object.entry("communities").or_insert(json!([]));
    Ok(())
}

fn community_mut<'a>(workforce: &'a mut Value, company_id: &str) -> Result<&'a mut Value> {
    array_mut(workforce, "communities")?
        .iter_mut()
        .find(|item| string(item, "companyId") == Some(company_id))
        .ok_or_else(|| anyhow!("community {company_id} not found"))
}

fn array_mut<'a>(value: &'a mut Value, key: &str) -> Result<&'a mut Vec<Value>> {
    value
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| anyhow!("{key} must be an array"))
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    string(value, key).ok_or_else(|| anyhow!("missing string field {key}"))
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn normalize_relay(relay: &str) -> String {
    relay.trim().trim_end_matches('/').to_lowercase()
}

fn parse_options(args: &[String]) -> Result<Options> {
    let value = |flag: &str| -> Result<PathBuf> {
        let index = args
            .iter()
            .position(|arg| arg == flag)
            .ok_or_else(|| anyhow!("missing required {flag}"))?;
        args.get(index + 1)
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("{flag} requires a path"))
    };
    Ok(Options {
        managed_store: value("--managed-store")?,
        workforce_store: value("--workforce-store")?,
        catalog: value("--catalog")?,
        company_registry: value("--company-registry")?,
        receipt: args
            .iter()
            .position(|arg| arg == "--receipt")
            .and_then(|index| args.get(index + 1))
            .map(PathBuf::from),
        apply: args.iter().any(|arg| arg == "--apply"),
    })
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
        .with_context(|| format!("parse {}", path.display()))
}

fn backup_path(path: &Path, stamp: &str) -> PathBuf {
    PathBuf::from(format!("{}.{}.bak", path.display(), stamp))
}

fn default_receipt_path(path: &Path) -> PathBuf {
    PathBuf::from(format!(
        "{}.{}.receipt.json",
        path.display(),
        Utc::now().format("%Y%m%dT%H%M%S%.fZ")
    ))
}

fn copy_new(source: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        bail!("refusing to overwrite backup {}", target.display());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, target)?;
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = PathBuf::from(format!("{}.tmp", path.display()));
    fs::write(&temp, bytes)?;
    fs::rename(&temp, path)?;
    Ok(())
}

fn rollback(receipt_path: &Path) -> Result<()> {
    let receipt: Receipt = read_json(receipt_path)?;
    let current_managed = fs::read(&receipt.managed_store)?;
    let current_workforce = fs::read(&receipt.workforce_store)?;
    if sha256(&current_managed) != receipt.managed_after_sha256
        || sha256(&current_workforce) != receipt.workforce_after_sha256
    {
        bail!("rollback refused: migrated files changed after the receipt was written");
    }
    let managed_before = fs::read(&receipt.managed_backup)?;
    if sha256(&managed_before) != receipt.managed_before_sha256 {
        bail!("managed backup hash mismatch");
    }
    atomic_write(&receipt.managed_store, &managed_before)?;
    match (&receipt.workforce_backup, &receipt.workforce_before_sha256) {
        (Some(path), Some(expected)) => {
            let bytes = fs::read(path)?;
            if sha256(&bytes) != *expected {
                bail!("workforce backup hash mismatch");
            }
            atomic_write(&receipt.workforce_store, &bytes)?;
        }
        (None, None) => fs::remove_file(&receipt.workforce_store)?,
        _ => bail!("invalid workforce backup receipt"),
    }
    println!("rollback complete");
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
