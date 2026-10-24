#!/usr/bin/env bash
# Attach the originals bucket when Secret Manager has the key.
# A missing secret leaves the server on database mode and exits 0.
# The key file is mode 0400, owned by uid 65532 (distroless nonroot).
# The parent directory stays mode 0700 root, so other host users cannot read it.
set -euo pipefail

PREFIX="${STACK_PREFIX:-edgequake}"
INSTALL_ROOT="${STACK_INSTALL_ROOT:-/opt/${PREFIX}}"
ENV_FILE="${STACK_ENV_FILE:-${INSTALL_ROOT}/.env}"
SECRET_NAME="${STACK_ORIGINALS_SECRET:-${PREFIX}-gcs-originals-key}"
KEY_FILE="${INSTALL_ROOT}/secrets/gcs-originals.json"
CONTAINER_KEY="/run/secrets/gcs-originals.json"
CONTAINER_UID=65532
CONTAINER_GID=65532

log() { echo "[originals] $*" >&2; }

PROJECT="${STACK_PROJECT:-}"
if [[ -z "${PROJECT}" ]]; then
  PROJECT="$(curl -sf -H "Metadata-Flavor: Google" \
    "http://metadata.google.internal/computeMetadata/v1/project/project-id" || true)"
fi
if [[ -z "${PROJECT}" ]]; then
  log "no project id; leaving original store unchanged"
  exit 0
fi

STORE_URL="gs://${PROJECT}-${PREFIX}-originals/demo"

# A sourced .env points this at the container path. gcloud must not use it.
unset GOOGLE_APPLICATION_CREDENTIALS || true

install -d -m 0700 -o root -g root "${INSTALL_ROOT}/secrets"

place_placeholder() {
  if [[ -f "${KEY_FILE}" ]]; then
    return 0
  fi
  printf '%s\n' '{}' >"${KEY_FILE}"
  chown "${CONTAINER_UID}:${CONTAINER_GID}" "${KEY_FILE}"
  chmod 0400 "${KEY_FILE}"
}

TMP="$(mktemp)"
trap 'rm -f "${TMP}"' EXIT

if ! gcloud secrets versions access latest --secret="${SECRET_NAME}" --project="${PROJECT}" >"${TMP}" 2>/dev/null; then
  log "secret ${SECRET_NAME} unavailable; leaving original store unchanged"
  place_placeholder
  exit 0
fi

if ! python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); assert d.get("type")=="service_account" and d.get("private_key") and d.get("client_email")' "${TMP}"; then
  log "secret ${SECRET_NAME} is not a service-account key; leaving original store unchanged"
  place_placeholder
  exit 0
fi

install -m 0400 -o "${CONTAINER_UID}" -g "${CONTAINER_GID}" "${TMP}" "${KEY_FILE}"

upsert() {
  local key="$1" val="$2"
  if [[ ! -f "${ENV_FILE}" ]]; then
    log "missing ${ENV_FILE}; key written, env lines skipped"
    return 0
  fi
  if grep -q "^${key}=" "${ENV_FILE}"; then
    sed -i "s|^${key}=.*|${key}=${val}|" "${ENV_FILE}"
  else
    printf '%s=%s\n' "${key}" "${val}" >>"${ENV_FILE}"
  fi
}

upsert EDGEQUAKE_ORIGINAL_STORE object
upsert EDGEQUAKE_ORIGINAL_STORE_URL "${STORE_URL}"
upsert GOOGLE_APPLICATION_CREDENTIALS "${CONTAINER_KEY}"
if [[ -f "${ENV_FILE}" ]]; then
  chmod 600 "${ENV_FILE}"
fi
log "original store object ${STORE_URL}"
