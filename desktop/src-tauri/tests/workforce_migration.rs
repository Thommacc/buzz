use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const BIN: &str = env!("CARGO_BIN_EXE_buzz-workforce-migrate");

#[test]
fn three_company_records_migrate_idempotently_and_rollback_byte_exactly() {
    let temp = tempdir().expect("tempdir");
    let managed = temp.path().join("managed-agents.json");
    let workforce = temp.path().join("workforce.json");
    let catalog = temp.path().join("catalog.json");
    let registry = temp.path().join("companies.json");
    let receipt = temp.path().join("receipt.json");
    let receipt_second = temp.path().join("receipt-second.json");
    write_fixture(&managed, &catalog, &registry, false);
    let original = fs::read(&managed).expect("original");

    let dry_run = migrate(&managed, &workforce, &catalog, &registry, &receipt, false);
    assert!(dry_run.status.success(), "{}", stderr(&dry_run));
    assert_eq!(fs::read(&managed).expect("managed"), original);
    assert!(!workforce.exists());
    assert!(!receipt.exists());

    let applied = migrate(&managed, &workforce, &catalog, &registry, &receipt, true);
    assert!(applied.status.success(), "{}", stderr(&applied));
    let migrated_managed: Vec<Value> = read_json(&managed);
    assert_eq!(migrated_managed.len(), 1);
    assert_eq!(migrated_managed[0]["runtime_pid"], Value::Null);
    let migrated_workforce: Value = read_json(&workforce);
    assert_eq!(
        migrated_workforce["identities"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        migrated_workforce["communities"].as_array().unwrap().len(),
        3
    );
    let memberships: usize = migrated_workforce["communities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|community| community["memberships"].as_array().unwrap().len())
        .sum();
    assert_eq!(memberships, 3);
    let models: Vec<&str> = migrated_workforce["communities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|community| {
            community["modelPolicy"]["roleOverrides"]["billing"]["model"]
                .as_str()
                .unwrap()
        })
        .collect();
    assert_eq!(models, vec!["model-a", "model-b", "model-c"]);
    let managed_after_first = hash(&fs::read(&managed).unwrap());
    let workforce_after_first = hash(&fs::read(&workforce).unwrap());

    let second = migrate(
        &managed,
        &workforce,
        &catalog,
        &registry,
        &receipt_second,
        true,
    );
    assert!(second.status.success(), "{}", stderr(&second));
    assert_eq!(hash(&fs::read(&managed).unwrap()), managed_after_first);
    assert_eq!(hash(&fs::read(&workforce).unwrap()), workforce_after_first);
    assert!(!receipt_second.exists());

    let rollback = Command::new(BIN)
        .args(["--rollback", receipt.to_str().unwrap()])
        .output()
        .expect("rollback");
    assert!(rollback.status.success(), "{}", stderr(&rollback));
    assert_eq!(fs::read(&managed).expect("restored managed"), original);
    assert!(!workforce.exists());
}

#[test]
fn conflicts_and_running_receipts_refuse_without_writes() {
    let temp = tempdir().expect("tempdir");
    let managed = temp.path().join("managed-agents.json");
    let workforce = temp.path().join("workforce.json");
    let catalog = temp.path().join("catalog.json");
    let registry = temp.path().join("companies.json");
    let receipt = temp.path().join("receipt.json");
    write_fixture(&managed, &catalog, &registry, true);
    let original = fs::read(&managed).expect("original");

    let result = migrate(&managed, &workforce, &catalog, &registry, &receipt, true);
    assert!(!result.status.success());
    let error = stderr(&result);
    assert!(
        error.contains("conflict") || error.contains("running"),
        "{error}"
    );
    assert_eq!(fs::read(&managed).expect("managed"), original);
    assert!(!workforce.exists());
    assert!(!receipt.exists());
}

fn migrate(
    managed: &Path,
    workforce: &Path,
    catalog: &Path,
    registry: &Path,
    receipt: &Path,
    apply: bool,
) -> Output {
    let mut command = Command::new(BIN);
    command.args([
        "--managed-store",
        managed.to_str().unwrap(),
        "--workforce-store",
        workforce.to_str().unwrap(),
        "--catalog",
        catalog.to_str().unwrap(),
        "--company-registry",
        registry.to_str().unwrap(),
        "--receipt",
        receipt.to_str().unwrap(),
    ]);
    if apply {
        command.arg("--apply");
    } else {
        command.arg("--dry-run");
    }
    command.output().expect("migration command")
}

fn write_fixture(managed: &Path, catalog: &Path, registry: &Path, conflict: bool) {
    let records: Vec<Value> = [
        ("wss://one.example", "model-a"),
        ("wss://two.example", "model-b"),
        ("wss://three.example", "model-c"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (relay, model))| {
        json!({
            "pubkey": if conflict && index == 1 { "bb" } else { "aa" },
            "name": "Billie Billing",
            "persona_id": "persona-billing",
            "relay_url": relay,
            "system_prompt": "Handle billing",
            "avatar_url": "avatar.png",
            "provider": "openai",
            "model": model,
            "runtime_pid": if conflict && index == 2 { json!(42) } else { Value::Null }
        })
    })
    .collect();
    fs::write(managed, serde_json::to_vec_pretty(&records).unwrap()).unwrap();
    fs::write(
        catalog,
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "catalogVersion": 1,
            "roles": [{
                "identityId": "billie-billing",
                "displayName": "Billie Billing",
                "personaId": "persona-billing",
                "roleId": "billing",
                "department": "Finance",
                "useWhen": "Billing work",
                "requiredInputs": ["records"],
                "expectedOutputs": ["invoice"],
                "generalPrompt": "Handle billing",
                "approvalBoundary": "Approve payments",
                "defaultModel": {"provider":"openai","model":"default","purposeLabel":"Billing","approved":true,"health":"healthy","lastHealthCheckAt":"2026-08-19T00:00:00Z"}
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        registry,
        serde_json::to_vec_pretty(&json!({
            "companies": [
                {"companyId":"one","displayName":"One","relayUrl":"wss://one.example","contextRef":"contexts/one.json"},
                {"companyId":"two","displayName":"Two","relayUrl":"wss://two.example","contextRef":"contexts/two.json"},
                {"companyId":"three","displayName":"Three","relayUrl":"wss://three.example","contextRef":"contexts/three.json"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
