#!/usr/bin/env bash
set -euo pipefail

CHANGELOG_FILE="${CHANGELOG_FILE:-CHANGELOG.md}"
REPO="${REPO:-${GITHUB_REPOSITORY:-}}"
AFTER_SHA="${AFTER_SHA:-${GITHUB_SHA:-}}"
BEFORE_SHA="${BEFORE_SHA:-}"

if [[ -z "${AFTER_SHA}" ]]; then
  AFTER_SHA="$(git rev-parse HEAD)"
fi

if [[ -z "${BEFORE_SHA}" || "${BEFORE_SHA}" =~ ^0+$ ]]; then
  if git rev-parse "${AFTER_SHA}^" >/dev/null 2>&1; then
    BEFORE_SHA="$(git rev-parse "${AFTER_SHA}^")"
  else
    BEFORE_SHA=""
  fi
fi

if [[ -n "${BEFORE_SHA}" ]]; then
  RANGE="${BEFORE_SHA}..${AFTER_SHA}"
else
  RANGE="${AFTER_SHA}"
fi

if [[ ! -f "${CHANGELOG_FILE}" ]]; then
  cat > "${CHANGELOG_FILE}" <<'EOF'
# Changelog

All notable changes to this project will be documented in this file.

<!-- changelog:start -->
<!-- changelog:end -->
EOF
fi

if ! grep -q '^<!-- changelog:start -->$' "${CHANGELOG_FILE}"; then
  printf '\n<!-- changelog:start -->\n<!-- changelog:end -->\n' >> "${CHANGELOG_FILE}"
fi

TMP_ENTRIES="$(mktemp)"
TMP_OUTPUT="$(mktemp)"
cleanup() {
  rm -f "${TMP_ENTRIES}" "${TMP_OUTPUT}"
}
trap cleanup EXIT

while IFS=$'\t' read -r commit_sha subject; do
  [[ -z "${commit_sha}" ]] && continue
  short_sha="${commit_sha:0:7}"
  entry_marker="<!-- sha:${commit_sha} -->"

  if [[ "${subject}" == chore\(changelog\):* ]]; then
    continue
  fi

  if grep -Fq "${entry_marker}" "${CHANGELOG_FILE}" || grep -Fq "${commit_sha}" "${CHANGELOG_FILE}"; then
    continue
  fi

  if [[ -z "${REPO}" ]] && grep -Fq "\`${short_sha}\`" "${CHANGELOG_FILE}"; then
    continue
  fi

  if [[ -n "${REPO}" ]]; then
    printf -- '- %s ([`%s`](https://github.com/%s/commit/%s)) %s\n' \
      "${subject}" "${short_sha}" "${REPO}" "${commit_sha}" "${entry_marker}" >> "${TMP_ENTRIES}"
  else
    printf -- '- %s (`%s`) %s\n' "${subject}" "${short_sha}" "${entry_marker}" >> "${TMP_ENTRIES}"
  fi
done < <(git log --reverse --pretty=format:'%H%x09%s' "${RANGE}")

if [[ ! -s "${TMP_ENTRIES}" ]]; then
  echo "No new changelog entries to add."
  exit 0
fi

today="$(date -u +%Y-%m-%d)"
today_header="### ${today}"
inside_section=0
inserted=0

while IFS= read -r line || [[ -n "${line}" ]]; do
  if [[ "${line}" == "<!-- changelog:start -->" ]]; then
    printf '%s\n' "${line}" >> "${TMP_OUTPUT}"
    inside_section=1
    continue
  fi

  if [[ ${inside_section} -eq 1 && "${line}" == "${today_header}" ]]; then
    printf '%s\n' "${line}" >> "${TMP_OUTPUT}"
    cat "${TMP_ENTRIES}" >> "${TMP_OUTPUT}"
    inserted=1
    continue
  fi

  if [[ ${inside_section} -eq 1 && "${line}" == "<!-- changelog:end -->" ]]; then
    if [[ ${inserted} -eq 0 ]]; then
      printf '\n%s\n' "${today_header}" >> "${TMP_OUTPUT}"
      cat "${TMP_ENTRIES}" >> "${TMP_OUTPUT}"
    fi
    printf '%s\n' "${line}" >> "${TMP_OUTPUT}"
    inside_section=0
    continue
  fi

  printf '%s\n' "${line}" >> "${TMP_OUTPUT}"
done < "${CHANGELOG_FILE}"

mv "${TMP_OUTPUT}" "${CHANGELOG_FILE}"
echo "Updated ${CHANGELOG_FILE}"
