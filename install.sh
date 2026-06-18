#!/usr/bin/env bash
# install.sh - Install solana-auditor skill into ~/.claude/skills/solana-auditor
# Usage: bash install.sh [-y|--yes] [-h|--help]
set -euo pipefail

SKILL_NAME="solana-auditor"
SKILLS_DIR="${HOME}/.claude/skills"
DEV_SKILL_NAME="solana-dev"
DEV_SKILL_REPO="https://github.com/solana-foundation/solana-dev-skill"
AUTO_YES=false

# Resolve the directory containing this script (works even when sourced via curl | bash with a tmp file)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<EOF
Usage: bash install.sh [OPTIONS]

Install the ${SKILL_NAME} skill for Claude Code / Codex.

Options:
  -y, --yes   Skip all confirmation prompts (non-interactive)
  -h, --help  Show this help and exit

Installs to:
  ${SKILLS_DIR}/${SKILL_NAME}/
  ~/.claude/agents/
  ~/.claude/commands/
  ~/.claude/rules/
  ~/.codex/skills/${SKILL_NAME}/   (if ~/.codex exists)
EOF
}

confirm() {
  local prompt="${1}"
  if [ "${AUTO_YES}" = true ]; then
    echo "${prompt} [auto-yes]"
    return 0
  fi
  printf "%s [y/N] " "${prompt}"
  read -r reply
  case "${reply}" in
    [Yy]|[Yy][Ee][Ss]) return 0 ;;
    *) return 1 ;;
  esac
}

log()  { echo "[install] $*"; }
die()  { echo "[install] ERROR: $*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Parse args
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    -y|--yes)  AUTO_YES=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "Unknown option: $1. Run with -h for help." ;;
  esac
done

# ---------------------------------------------------------------------------
# Preflight
# ---------------------------------------------------------------------------
command -v git >/dev/null 2>&1 || die "git is required but not found in PATH."
mkdir -p "${SKILLS_DIR}"

# ---------------------------------------------------------------------------
# Step 1: Ensure solana-dev skill (dependency for delegation)
# ---------------------------------------------------------------------------
if [ ! -f "${SKILLS_DIR}/${DEV_SKILL_NAME}/SKILL.md" ]; then
  log "solana-dev skill not found at ${SKILLS_DIR}/${DEV_SKILL_NAME}/"
  if confirm "Clone and install solana-dev skill (required for fix/program-change delegation)?"; then
    TMP_DEV="$(mktemp -d)"
    trap 'rm -rf "${TMP_DEV}"' EXIT
    log "Cloning ${DEV_SKILL_REPO} (shallow)..."
    git clone --depth 1 "${DEV_SKILL_REPO}" "${TMP_DEV}/solana-dev-skill" \
      || die "Failed to clone solana-dev skill. Check your network connection and the repo URL."
    # Copy the skill/ subtree into skills/solana-dev
    mkdir -p "${SKILLS_DIR}/${DEV_SKILL_NAME}"
    if [ -d "${TMP_DEV}/solana-dev-skill/skill" ]; then
      cp -r "${TMP_DEV}/solana-dev-skill/skill/." "${SKILLS_DIR}/${DEV_SKILL_NAME}/"
    elif [ -f "${TMP_DEV}/solana-dev-skill/SKILL.md" ]; then
      cp -r "${TMP_DEV}/solana-dev-skill/." "${SKILLS_DIR}/${DEV_SKILL_NAME}/"
    else
      die "solana-dev-skill repo layout unexpected - no skill/ dir or SKILL.md found."
    fi
    log "solana-dev installed -> ${SKILLS_DIR}/${DEV_SKILL_NAME}/"
    trap - EXIT
    rm -rf "${TMP_DEV}"
  else
    log "Skipping solana-dev installation. Delegation features will not work until it is installed."
  fi
else
  log "solana-dev skill already present at ${SKILLS_DIR}/${DEV_SKILL_NAME}/ - skipping."
fi

# ---------------------------------------------------------------------------
# Step 2: Install solana-auditor skill (idempotent: remove then copy)
# ---------------------------------------------------------------------------
DEST_SKILL="${SKILLS_DIR}/${SKILL_NAME}"
log "Installing ${SKILL_NAME} -> ${DEST_SKILL}/"

if [ -d "${DEST_SKILL}" ]; then
  log "Removing existing installation at ${DEST_SKILL}/"
  rm -rf "${DEST_SKILL}"
fi
mkdir -p "${DEST_SKILL}"

if [ -d "${SCRIPT_DIR}/skill" ]; then
  cp -r "${SCRIPT_DIR}/skill/." "${DEST_SKILL}/"
else
  die "skill/ directory not found at ${SCRIPT_DIR}/skill. Run install.sh from the repo root."
fi
log "Skill files installed -> ${DEST_SKILL}/"

# ---------------------------------------------------------------------------
# Step 3: Install agents, commands, rules into ~/.claude/
# ---------------------------------------------------------------------------
CLAUDE_DIR="${HOME}/.claude"
mkdir -p "${CLAUDE_DIR}/agents" "${CLAUDE_DIR}/commands" "${CLAUDE_DIR}/rules"

if [ -d "${SCRIPT_DIR}/agents" ] && [ -n "$(ls -A "${SCRIPT_DIR}/agents" 2>/dev/null)" ]; then
  cp "${SCRIPT_DIR}/agents/"*.md "${CLAUDE_DIR}/agents/" 2>/dev/null || true
  log "Agents installed -> ${CLAUDE_DIR}/agents/"
fi

if [ -d "${SCRIPT_DIR}/commands" ] && [ -n "$(ls -A "${SCRIPT_DIR}/commands" 2>/dev/null)" ]; then
  cp "${SCRIPT_DIR}/commands/"*.md "${CLAUDE_DIR}/commands/" 2>/dev/null || true
  log "Commands installed -> ${CLAUDE_DIR}/commands/"
fi

if [ -d "${SCRIPT_DIR}/rules" ] && [ -n "$(ls -A "${SCRIPT_DIR}/rules" 2>/dev/null)" ]; then
  cp "${SCRIPT_DIR}/rules/"*.md "${CLAUDE_DIR}/rules/" 2>/dev/null || true
  log "Rules installed -> ${CLAUDE_DIR}/rules/"
fi

# ---------------------------------------------------------------------------
# Step 4: Mirror to ~/.codex/skills/ if Codex is present
# ---------------------------------------------------------------------------
CODEX_DIR="${HOME}/.codex"
if [ -d "${CODEX_DIR}" ]; then
  CODEX_SKILL="${CODEX_DIR}/skills/${SKILL_NAME}"
  log "Codex directory detected - mirroring to ${CODEX_SKILL}/"
  rm -rf "${CODEX_SKILL}"
  mkdir -p "${CODEX_SKILL}"
  cp -r "${DEST_SKILL}/." "${CODEX_SKILL}/"
  log "Codex mirror installed -> ${CODEX_SKILL}/"
else
  log "~/.codex not found - skipping Codex mirror (not required)."
fi

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
echo ""
echo "========================================================"
echo " ${SKILL_NAME} installed successfully."
echo "========================================================"
echo " Skill:    ${DEST_SKILL}/"
echo " Agents:   ${CLAUDE_DIR}/agents/"
echo " Commands: ${CLAUDE_DIR}/commands/"
echo " Rules:    ${CLAUDE_DIR}/rules/"
if [ -d "${CODEX_DIR}" ]; then
  echo " Codex:    ${CODEX_DIR}/skills/${SKILL_NAME}/"
fi
echo ""
echo " Restart Claude Code (or run /reload) to activate."
echo " Trigger the skill by saying: 'audit this Solana program'"
echo " or type /audit-program, /static-scan, /write-report."
echo "========================================================"
