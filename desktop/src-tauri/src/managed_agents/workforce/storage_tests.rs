use std::fs;

use super::*;

fn proposed_fact(id: &str) -> ContextFact {
    ContextFact {
        fact_id: id.into(),
        statement: format!("Statement {id}"),
        source: "mail:rashid:2026-08-19".into(),
        proposed_at: "2026-08-19T10:00:00Z".into(),
        confidence_basis_points: 8000,
        status: ContextFactStatus::Proposed,
    }
}

fn empty_context() -> CompanyContext {
    CompanyContext {
        company_id: "acme".into(),
        version: 0,
        revision: 0,
        approved_at: None,
        facts: Vec::new(),
        documents: Vec::new(),
        templates: Default::default(),
    }
}

#[test]
fn missing_store_returns_versioned_empty_default() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = load_workforce_from_path(&dir.path().join("workforce.json"))
        .expect("missing store is valid");
    assert_eq!(store, WorkforceStore::default());
}

#[test]
fn invalid_json_is_preserved_and_returns_an_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("workforce.json");
    fs::write(&path, b"{invalid").expect("write fixture");
    let error = load_workforce_from_path(&path).expect_err("invalid store must fail");
    assert!(error.contains("preserved as .invalid"));
    assert_eq!(
        fs::read(path.with_extension("json.invalid")).expect("invalid backup"),
        b"{invalid"
    );
    assert_eq!(fs::read(path).expect("original remains"), b"{invalid");
}

#[test]
fn workforce_save_round_trips_atomically() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("workforce.json");
    let store = WorkforceStore::default();
    save_workforce_to_path(&path, &store).expect("save");
    assert_eq!(load_workforce_from_path(&path).expect("load"), store);
    assert!(!path.with_extension("json.tmp").exists());
}

#[cfg(unix)]
#[test]
fn workforce_and_context_files_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("temp dir");
    let workforce_path = dir.path().join("workforce.json");
    let context_path = dir.path().join("acme.json");
    save_workforce_to_path(&workforce_path, &WorkforceStore::default()).expect("save workforce");
    save_company_context_to_path(&context_path, &empty_context()).expect("save context");
    for path in [workforce_path, context_path] {
        let mode = fs::metadata(path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[test]
fn invalid_new_store_does_not_replace_last_valid_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("workforce.json");
    let valid = WorkforceStore::default();
    save_workforce_to_path(&path, &valid).expect("initial save");
    let before = fs::read(&path).expect("before bytes");

    let mut invalid = valid;
    invalid.schema_version += 1;
    assert!(save_workforce_to_path(&path, &invalid).is_err());
    assert_eq!(fs::read(&path).expect("after bytes"), before);
}

#[test]
fn proposal_approval_rejection_and_supersession_are_auditable() {
    let mut context = empty_context();
    context
        .propose_fact(proposed_fact("first"))
        .expect("propose");
    assert_eq!(
        context.version, 0,
        "untrusted proposal is not an approved version"
    );
    assert!(context.approved_facts().next().is_none());

    context
        .approve_fact("first", "thom", "2026-08-19T11:00:00Z")
        .expect("approve");
    assert_eq!(context.version, 1);
    assert_eq!(context.approved_facts().count(), 1);

    context
        .propose_fact(proposed_fact("rejected"))
        .expect("propose");
    context
        .reject_fact(
            "rejected",
            "thom",
            "2026-08-19T11:10:00Z",
            "source contradicted",
        )
        .expect("reject");
    assert_eq!(
        context.version, 1,
        "rejection does not change approved context"
    );

    context
        .propose_fact(proposed_fact("replacement"))
        .expect("propose replacement");
    context
        .approve_fact("replacement", "thom", "2026-08-19T11:20:00Z")
        .expect("approve replacement");
    context
        .supersede_fact("first", "replacement", "2026-08-19T11:21:00Z")
        .expect("supersede");
    assert_eq!(context.version, 3);
    let approved: Vec<_> = context
        .approved_facts()
        .map(|fact| fact.fact_id.as_str())
        .collect();
    assert_eq!(approved, vec!["replacement"]);
    assert!(matches!(
        context.facts[0].status,
        ContextFactStatus::Superseded { .. }
    ));
}

#[test]
fn fact_transitions_reject_duplicates_and_non_proposals() {
    let mut context = empty_context();
    context
        .propose_fact(proposed_fact("fact"))
        .expect("propose");
    assert!(context.propose_fact(proposed_fact("fact")).is_err());
    context
        .approve_fact("fact", "thom", "2026-08-19T11:00:00Z")
        .expect("approve");
    assert!(context
        .approve_fact("fact", "thom", "2026-08-19T12:00:00Z")
        .is_err());
    assert!(context
        .supersede_fact("fact", "missing", "2026-08-19T12:00:00Z")
        .is_err());
}

#[test]
fn context_load_rejects_company_path_mismatch() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("acme.json");
    let mut context = empty_context();
    context.company_id = "other".into();
    save_company_context_to_path(&path, &context).expect("save fixture");
    assert!(load_company_context_from_path(&path, "acme").is_err());
}
