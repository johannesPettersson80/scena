use crate::app::prelude::*;

const EXPECTED_REPOSITORY: &str = "johannesPettersson80/scena";
const PROVENANCE_FILENAME: &str = "ci-provenance.json";
const OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";
const SLSA_PROVENANCE_V1: &str = "https://slsa.dev/provenance/v1";

#[derive(Debug, Clone)]
pub(super) struct VerifiedCiProvenance {
    pub(super) manifest: Value,
    pub(super) verification_receipt_sha256: String,
}

pub(crate) fn canonical_artifact_tree_digest(root: &Path) -> Result<(String, usize), String> {
    let mut files = Vec::new();
    collect_artifact_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut tree = Sha256::new();
    for (relative, digest) in &files {
        tree.update(relative.as_bytes());
        tree.update([0]);
        tree.update(digest.as_bytes());
        tree.update(b"\n");
    }
    Ok((hex_digest(tree.finalize()), files.len()))
}

fn collect_artifact_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "failed to read artifact directory {}: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if kind.is_symlink() {
            return Err(format!(
                "CI provenance refuses symbolic-link artifact {relative}"
            ));
        }
        if kind.is_dir() {
            collect_artifact_files(root, &path, files)?;
        } else if kind.is_file() && relative != PROVENANCE_FILENAME {
            let bytes = fs::read(&path)
                .map_err(|error| format!("failed to read artifact {relative}: {error}"))?;
            files.push((relative, hex_digest(Sha256::digest(bytes))));
        }
    }
    Ok(())
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn validate_ci_provenance_manifest(
    artifact_root: &Path,
    manifest: &Value,
    expected_commit: &str,
    expected_context: &Value,
    now: u64,
) -> Result<(), String> {
    validate_ci_provenance_identity(manifest, expected_commit, expected_context, now)?;
    let object = manifest
        .as_object()
        .expect("CI provenance identity validation requires an object");
    let (actual_digest, actual_count) = canonical_artifact_tree_digest(artifact_root)?;
    require_exact_string(object, "artifact_digest", &actual_digest)?;
    if object.get("artifact_file_count").and_then(Value::as_u64) != Some(actual_count as u64) {
        return Err(format!(
            "CI provenance artifact_file_count does not match the downloaded tree ({actual_count})"
        ));
    }
    Ok(())
}

fn validate_ci_provenance_identity(
    manifest: &Value,
    expected_commit: &str,
    expected_context: &Value,
    now: u64,
) -> Result<(), String> {
    let object = manifest
        .as_object()
        .ok_or_else(|| "CI provenance manifest must be a JSON object".to_string())?;
    require_exact_string(object, "schema", "scena.ci_provenance.v1")?;
    require_exact_string(object, "repository", EXPECTED_REPOSITORY)?;
    require_exact_string(object, "issuer", OIDC_ISSUER)?;
    require_exact_string(object, "source_commit", expected_commit)?;
    require_exact_bool(object, "release_evidence", false)?;
    let workflow_ref = required_string(object, "workflow_ref")?;
    let allowed_prefix = format!("{EXPECTED_REPOSITORY}/.github/workflows/");
    if !workflow_ref.starts_with(&allowed_prefix)
        || !["ci.yml", "release.yml"]
            .iter()
            .any(|workflow| workflow_ref.starts_with(&format!("{allowed_prefix}{workflow}@")))
    {
        return Err(format!(
            "CI provenance workflow_ref is not an approved scena workflow: {workflow_ref}"
        ));
    }
    validate_exact_hex40(required_string(object, "workflow_sha")?, "workflow_sha")?;
    validate_exact_hex40(expected_commit, "source_commit")?;
    for field in [
        "repository",
        "workflow_ref",
        "workflow_sha",
        "ref",
        "run_id",
        "run_attempt",
        "job",
        "source_commit",
    ] {
        if manifest.get(field) != expected_context.get(field) {
            return Err(format!(
                "CI provenance {field} does not match the current trusted workflow context"
            ));
        }
    }
    if required_string(object, "job")?.trim().is_empty() {
        return Err("CI provenance job must be non-blank".to_string());
    }
    if !required_string(object, "run_id")?
        .bytes()
        .all(|byte| byte.is_ascii_digit())
        || object
            .get("run_attempt")
            .and_then(Value::as_u64)
            .is_none_or(|attempt| attempt == 0)
    {
        return Err("CI provenance run id and attempt must be positive integers".to_string());
    }
    let timestamp = object
        .get("generated_at_unix_seconds")
        .and_then(Value::as_u64)
        .ok_or_else(|| "CI provenance timestamp must be numeric".to_string())?;
    if timestamp > now.saturating_add(RELEASE_ARTIFACT_MAX_FUTURE_SKEW_SECONDS)
        || now.saturating_sub(timestamp) > RELEASE_ARTIFACT_MAX_AGE_SECONDS
    {
        return Err("CI provenance timestamp is stale or too far in the future".to_string());
    }
    if manifest.get("release_rejection_codes") != Some(&json!(["CI_ATTESTATION_NOT_YET_VERIFIED"]))
        || manifest.get("attestation")
            != Some(&json!({
                "predicate_type": SLSA_PROVENANCE_V1,
                "verification_status": "pending",
            }))
    {
        return Err(
            "unsigned CI manifest must remain pending and explicitly non-release".to_string(),
        );
    }
    validate_exact_hex64(
        required_string(object, "artifact_digest")?,
        "artifact_digest",
    )?;
    if object
        .get("artifact_file_count")
        .and_then(Value::as_u64)
        .is_none_or(|count| count == 0)
    {
        return Err("CI provenance artifact_file_count must be positive".to_string());
    }
    Ok(())
}

pub(super) fn verify_ci_provenance(
    checkout_root: &Path,
    artifact_root: &Path,
    expected_commit: &str,
) -> Result<VerifiedCiProvenance, String> {
    let expected_context = trusted_workflow_context_from_env()?;
    let manifest_path = artifact_root.join(PROVENANCE_FILENAME);
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).map_err(|error| {
            format!(
                "strict release staging requires {}: {error}",
                manifest_path.display()
            )
        })?)
        .map_err(|error| format!("CI provenance manifest is invalid JSON: {error}"))?;
    validate_ci_provenance_manifest(
        artifact_root,
        &manifest,
        expected_commit,
        &expected_context,
        current_unix_seconds(),
    )?;
    let verification_receipt_sha256 =
        verify_attestation(checkout_root, &manifest_path, &manifest, expected_commit)?;
    Ok(VerifiedCiProvenance {
        manifest,
        verification_receipt_sha256,
    })
}

pub(super) fn verify_staged_ci_provenance(
    checkout_root: &Path,
    artifact_root: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    let expected_context = trusted_workflow_context_from_env()?;
    let manifest_path = artifact_root.join(PROVENANCE_FILENAME);
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).map_err(|error| {
            format!(
                "release readiness requires signed {}: {error}",
                manifest_path.display()
            )
        })?)
        .map_err(|error| format!("staged CI provenance manifest is invalid JSON: {error}"))?;
    validate_ci_provenance_identity(
        &manifest,
        expected_commit,
        &expected_context,
        current_unix_seconds(),
    )?;
    let metadata_path = artifact_root.join("staging-metadata.json");
    let metadata: Value =
        serde_json::from_str(&fs::read_to_string(&metadata_path).map_err(|error| {
            format!(
                "release readiness requires {}: {error}",
                metadata_path.display()
            )
        })?)
        .map_err(|error| format!("staging metadata is invalid JSON: {error}"))?;
    for field in [
        "schema",
        "repository",
        "workflow_ref",
        "workflow_sha",
        "ref",
        "run_id",
        "run_attempt",
        "job",
        "source_commit",
        "artifact_digest",
        "artifact_file_count",
        "issuer",
        "generated_at_unix_seconds",
    ] {
        if metadata["ci_provenance"].get(field) != manifest.get(field) {
            return Err(format!(
                "staging metadata {field} does not match the signed CI provenance manifest"
            ));
        }
    }
    if metadata["release_evidence"] != true
        || metadata["ci_provenance"]["attestation"]["verification_status"] != "verified"
    {
        return Err("staging metadata does not record verified CI provenance".to_string());
    }
    verify_attestation(checkout_root, &manifest_path, &manifest, expected_commit)?;
    Ok(())
}

fn verify_attestation(
    checkout_root: &Path,
    manifest_path: &Path,
    manifest: &Value,
    expected_commit: &str,
) -> Result<String, String> {
    let reachable = ProcessCommand::new("git")
        .current_dir(checkout_root)
        .args(["cat-file", "-e", &format!("{expected_commit}^{{commit}}")])
        .status()
        .map_err(|error| format!("failed to check source commit reachability: {error}"))?;
    if !reachable.success() {
        return Err(format!(
            "CI provenance source commit {expected_commit} is not reachable in the staging checkout"
        ));
    }
    let repository = required_value_string(manifest, "repository")?;
    let workflow_ref = required_value_string(manifest, "workflow_ref")?;
    let workflow = workflow_ref
        .split_once('@')
        .map(|(workflow, _)| workflow)
        .ok_or_else(|| "CI provenance workflow_ref is missing @ref".to_string())?;
    let source_ref = required_value_string(manifest, "ref")?;
    let output = ProcessCommand::new("gh")
        .args([
            "attestation",
            "verify",
            manifest_path
                .to_str()
                .ok_or_else(|| "CI provenance path is not UTF-8".to_string())?,
            "--repo",
            repository,
            "--signer-workflow",
            workflow,
            "--source-digest",
            expected_commit,
            "--source-ref",
            source_ref,
            "--format=json",
        ])
        .output()
        .map_err(|error| format!("failed to execute gh attestation verify: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "CI provenance attestation verification failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let receipt: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("gh attestation verification output is invalid JSON: {error}"))?;
    if receipt.as_array().is_none_or(Vec::is_empty) {
        return Err("gh attestation verification returned no verified attestations".to_string());
    }
    Ok(hex_digest(Sha256::digest(&output.stdout)))
}

fn trusted_workflow_context_from_env() -> Result<Value, String> {
    if env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        return Err("strict release staging requires a trusted GitHub Actions context".to_string());
    }
    let string = |name: &str| {
        env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("strict release staging requires non-blank {name}"))
    };
    let attempt = string("GITHUB_RUN_ATTEMPT")?
        .parse::<u64>()
        .map_err(|_| "GITHUB_RUN_ATTEMPT must be numeric".to_string())?;
    Ok(json!({
        "repository": string("GITHUB_REPOSITORY")?,
        "workflow_ref": string("GITHUB_WORKFLOW_REF")?,
        "workflow_sha": string("GITHUB_WORKFLOW_SHA")?.to_lowercase(),
        "ref": string("GITHUB_REF")?,
        "run_id": string("GITHUB_RUN_ID")?,
        "run_attempt": attempt,
        "job": string("GITHUB_JOB")?,
        "source_commit": string("GITHUB_SHA")?.to_lowercase(),
    }))
}

fn required_value_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("CI provenance {field} must be non-blank"))
}

fn required_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    field: &str,
) -> Result<&'a str, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("CI provenance {field} must be non-blank"))
}

fn require_exact_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
    expected: &str,
) -> Result<(), String> {
    let actual = required_string(object, field)?;
    if actual != expected {
        return Err(format!(
            "CI provenance {field} must be {expected:?}, got {actual:?}"
        ));
    }
    Ok(())
}

fn require_exact_bool(
    object: &serde_json::Map<String, Value>,
    field: &str,
    expected: bool,
) -> Result<(), String> {
    if object.get(field).and_then(Value::as_bool) != Some(expected) {
        return Err(format!("CI provenance {field} must be {expected}"));
    }
    Ok(())
}

fn validate_exact_hex40(value: &str, field: &str) -> Result<(), String> {
    if value.len() != 40
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(format!(
            "CI provenance {field} must be an exact lowercase 40-hex SHA"
        ));
    }
    Ok(())
}

fn validate_exact_hex64(value: &str, field: &str) -> Result<(), String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!(
            "CI provenance {field} must be an exact 64-hex SHA-256 digest"
        ))
    }
}
