#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'usage: %s <completed-run-id> [output-directory]\n' "${0##*/}" >&2
  exit 2
}

[[ $# -ge 1 && $# -le 2 ]] || usage

run_id="$1"
[[ "$run_id" =~ ^[0-9]+$ ]] || usage

for command in gh jq; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'required command is unavailable: %s\n' "$command" >&2
    exit 1
  fi
done

repository="${SCENA_GITHUB_REPOSITORY:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}"
output_dir="${2:-target/ci-failure-evidence/run-${run_id}}"

if [[ -e "$output_dir" ]] && [[ -n "$(find "$output_dir" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null)" ]]; then
  printf 'refusing to overwrite existing CI evidence: %s\n' "$output_dir" >&2
  exit 1
fi

mkdir -p "$output_dir/logs" "$output_dir/annotations" "$output_dir/artifacts"

gh run view "$run_id" --repo "$repository" \
  --json databaseId,workflowName,event,status,conclusion,headBranch,headSha,createdAt,updatedAt,url,jobs \
  >"$output_dir/run.json"

if [[ "$(jq -r '.status' "$output_dir/run.json")" != "completed" ]]; then
  printf 'run %s is not completed; evidence collection requires terminal state\n' "$run_id" >&2
  exit 1
fi

jq -r '
  .jobs[]
  | select(.conclusion != "success" and .conclusion != "skipped" and .conclusion != "neutral")
  | [.databaseId, .name, (.conclusion // "unknown")]
  | @tsv
' "$output_dir/run.json" >"$output_dir/failed-jobs.tsv"

while IFS=$'\t' read -r job_id job_name conclusion; do
  [[ -n "$job_id" ]] || continue
  safe_name="$(printf '%s' "$job_name" | tr -cs 'A-Za-z0-9._-' '_')"
  log_path="$output_dir/logs/${job_id}-${safe_name}-${conclusion}.log"
  annotation_path="$output_dir/annotations/${job_id}-${safe_name}.json"
  gh api "repos/${repository}/actions/jobs/${job_id}/logs" >"$log_path" || {
    printf 'could not collect job log for %s (%s)\n' "$job_name" "$job_id" >&2
    exit 1
  }
  gh api --paginate "repos/${repository}/check-runs/${job_id}/annotations" \
    | jq -s 'add // []' >"$annotation_path"
done <"$output_dir/failed-jobs.tsv"

artifact_count="$(gh api "repos/${repository}/actions/runs/${run_id}/artifacts" --jq '.total_count')"
if [[ "$artifact_count" -gt 0 ]]; then
  gh run download "$run_id" --repo "$repository" --dir "$output_dir/artifacts"
fi

jq -n \
  --arg repository "$repository" \
  --argjson run_id "$run_id" \
  --arg head_sha "$(jq -r '.headSha' "$output_dir/run.json")" \
  --arg conclusion "$(jq -r '.conclusion // "unknown"' "$output_dir/run.json")" \
  --argjson failed_job_count "$(wc -l <"$output_dir/failed-jobs.tsv" | tr -d ' ')" \
  --argjson artifact_count "$artifact_count" \
  '{
    schema: "scena.ci_failure_evidence.v1",
    repository: $repository,
    run_id: $run_id,
    head_sha: $head_sha,
    conclusion: $conclusion,
    failed_job_count: $failed_job_count,
    artifact_count: $artifact_count,
    classification_status: "unclassified",
    allowed_classifications: [
      "product defect",
      "test-harness defect",
      "environment failure",
      "policy failure",
      "provenance failure"
    ]
  }' >"$output_dir/summary.json"

{
  printf '# CI failure root-cause checkpoint\n\n'
  printf -- '- Repository: `%s`\n' "$repository"
  printf -- '- Run: `%s`\n' "$run_id"
  printf -- '- Commit: `%s`\n' "$(jq -r '.headSha' "$output_dir/run.json")"
  printf -- '- Exact failure signature: TODO\n'
  printf -- '- Classification: TODO (product, test-harness, environment, policy, or provenance)\n'
  printf -- '- Elapsed investigation time: TODO\n'
  printf -- '- Remediation attempts: 0\n'
  printf -- '- Release-candidate pushes: 0\n'
  printf -- '- Full-matrix runs: 1\n'
  printf -- '- User-required actions: 0\n'
  printf -- '- Competing causes and discriminating probe: TODO\n'
} >"$output_dir/root-cause-checkpoint.md"

printf 'ci_failure_evidence=%s\n' "$output_dir"
printf 'repository=%s\n' "$repository"
printf 'run_id=%s\n' "$run_id"
printf 'head_sha=%s\n' "$(jq -r '.headSha' "$output_dir/run.json")"
printf 'failed_job_count=%s\n' "$(wc -l <"$output_dir/failed-jobs.tsv" | tr -d ' ')"
printf 'artifact_count=%s\n' "$artifact_count"
