#!/usr/bin/env bash
# install-custom.sh - Interactive installer for solana-auditor skill
# Lets the user choose installation scope and path before copying files.
set -euo pipefail

SKILL_NAME="solana-auditor"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

log()  { echo "[install-custom] $*"; }
die()  { echo "[install-custom] ERROR: $*" >&2; exit 1; }

prompt_choice() {
  local question="${1}"
  shift
  local options=("$@")
  echo ""
  echo "${question}"
  local i=1
  for opt in "${options[@]}"; do
    echo "  ${i}) ${opt}"
    i=$((i + 1))
  done
  printf "Enter choice [1-%d]: " "${#options[@]}"
  read -r choice
  # Validate
  if ! [[ "${choice}" =~ ^[0-9]+$ ]] || [ "${choice}" -lt 1 ] || [ "${choice}" -gt "${#options[@]}" ]; then
    die "Invalid choice '${choice}'."
  fi
  # Return chosen index (1-based) via global CHOICE_IDX
  CHOICE_IDX="${choice}"
}

prompt_yn() {
  local question="${1}"
  printf "%s [y/N] " "${question}"
  read -r reply
  case "${reply}" in
    [Yy]|[Yy][Ee][Ss]) return 0 ;;
    *) return 1 ;;
  esac
}

prompt_path() {
  local prompt="${1}"
  local default="${2}"
  printf "%s [%s]: " "${prompt}" "${default}"
  read -r user_input
  if [ -z "${user_input}" ]; then
    CUSTOM_PATH="${default}"
  else
    CUSTOM_PATH="${user_input}"
  fi
}

# ---------------------------------------------------------------------------
# Banner
# ---------------------------------------------------------------------------
cat <<'BANNER'
==========================================================
  solana-auditor skill - Custom Installer
  Practical Solana/Anchor security audit workflow skill
==========================================================
BANNER

# ---------------------------------------------------------------------------
# 1. Installation scope
# ---------------------------------------------------------------------------
prompt_choice \
  "Where do you want to install the skill?" \
  "Personal   (~/.claude/skills/${SKILL_NAME}/)  - available in all projects" \
  "Project    (./.claude/skills/${SKILL_NAME}/)  - current project only" \
  "Custom     (specify a path)"

SCOPE_CHOICE="${CHOICE_IDX}"

case "${SCOPE_CHOICE}" in
  1)
    SKILLS_DIR="${HOME}/.claude/skills"
    GLOBAL_EXTRAS=true
    ;;
  2)
    SKILLS_DIR="$(pwd)/.claude/skills"
    GLOBAL_EXTRAS=false
    ;;
  3)
    prompt_path "Enter the skills directory path" "${HOME}/.claude/skills"
    SKILLS_DIR="${CUSTOM_PATH}"
    GLOBAL_EXTRAS=false
    ;;
esac

DEST_SKILL="${SKILLS_DIR}/${SKILL_NAME}"
echo ""
log "Skill will be installed to: ${DEST_SKILL}/"

# ---------------------------------------------------------------------------
# 2. Optional: install agents / commands / rules
# ---------------------------------------------------------------------------
INSTALL_AGENTS=false
INSTALL_COMMANDS=false
INSTALL_RULES=false

if [ "${GLOBAL_EXTRAS}" = true ]; then
  EXTRAS_BASE="${HOME}/.claude"
else
  EXTRAS_BASE="$(pwd)/.claude"
fi

echo ""
echo "The skill includes agents, commands, and rules that enhance Claude Code."
echo "They will be placed in: ${EXTRAS_BASE}/{agents,commands,rules}/"
echo ""

if prompt_yn "Install agents (solana-auditor, audit-report-writer)?"; then
  INSTALL_AGENTS=true
fi
if prompt_yn "Install commands (/audit-program, /static-scan, /write-report)?"; then
  INSTALL_COMMANDS=true
fi
if prompt_yn "Install rules (audit-rigor quality gate, auto-attached to *.md)?"; then
  INSTALL_RULES=true
fi

# ---------------------------------------------------------------------------
# 3. Optional: Codex mirror
# ---------------------------------------------------------------------------
INSTALL_CODEX=false
if [ -d "${HOME}/.codex" ]; then
  echo ""
  if prompt_yn "Mirror to ~/.codex/skills/${SKILL_NAME}/ (Codex detected)?"; then
    INSTALL_CODEX=true
  fi
fi

# ---------------------------------------------------------------------------
# 4. Optional: solana-dev dependency
# ---------------------------------------------------------------------------
DEV_SKILL_DIR="${HOME}/.claude/skills/solana-dev"
CLONE_DEV=false
if [ ! -f "${DEV_SKILL_DIR}/SKILL.md" ]; then
  echo ""
  echo "The solana-dev skill is not installed. It is required for fix/program-change"
  echo "delegation (delegation.md -> ../solana-dev/)."
  if prompt_yn "Clone and install solana-dev skill now?"; then
    CLONE_DEV=true
  fi
fi

# ---------------------------------------------------------------------------
# 5. Confirm plan
# ---------------------------------------------------------------------------
echo ""
echo "----------------------------------------------------------"
echo "Installation plan:"
echo "  Skill destination : ${DEST_SKILL}/"
if [ "${INSTALL_AGENTS}" = true ];   then echo "  Agents            : ${EXTRAS_BASE}/agents/"; fi
if [ "${INSTALL_COMMANDS}" = true ]; then echo "  Commands          : ${EXTRAS_BASE}/commands/"; fi
if [ "${INSTALL_RULES}" = true ];    then echo "  Rules             : ${EXTRAS_BASE}/rules/"; fi
if [ "${INSTALL_CODEX}" = true ];    then echo "  Codex mirror      : ${HOME}/.codex/skills/${SKILL_NAME}/"; fi
if [ "${CLONE_DEV}" = true ];        then echo "  solana-dev clone  : ${DEV_SKILL_DIR}/"; fi
echo "----------------------------------------------------------"

if ! prompt_yn "Proceed with installation?"; then
  echo "Aborted."
  exit 0
fi

# ---------------------------------------------------------------------------
# 6. Execute
# ---------------------------------------------------------------------------

# -- solana-dev --
if [ "${CLONE_DEV}" = true ]; then
  command -v git >/dev/null 2>&1 || die "git is required to clone solana-dev but was not found in PATH."
  TMP_DEV="$(mktemp -d)"
  trap 'rm -rf "${TMP_DEV}"' EXIT
  log "Cloning solana-dev skill (shallow)..."
  git clone --depth 1 https://github.com/solana-foundation/solana-dev-skill "${TMP_DEV}/solana-dev-skill" \
    || die "Clone failed. Check network connection and repository URL."
  mkdir -p "${DEV_SKILL_DIR}"
  if [ -d "${TMP_DEV}/solana-dev-skill/skill" ]; then
    cp -r "${TMP_DEV}/solana-dev-skill/skill/." "${DEV_SKILL_DIR}/"
  elif [ -f "${TMP_DEV}/solana-dev-skill/SKILL.md" ]; then
    cp -r "${TMP_DEV}/solana-dev-skill/." "${DEV_SKILL_DIR}/"
  else
    die "Unexpected solana-dev-skill repo layout."
  fi
  trap - EXIT
  rm -rf "${TMP_DEV}"
  log "solana-dev installed -> ${DEV_SKILL_DIR}/"
fi

# -- Skill --
if [ ! -d "${SCRIPT_DIR}/skill" ]; then
  die "skill/ directory not found at ${SCRIPT_DIR}/skill. Run from repo root."
fi
rm -rf "${DEST_SKILL}"
mkdir -p "${DEST_SKILL}"
cp -r "${SCRIPT_DIR}/skill/." "${DEST_SKILL}/"
log "Skill installed -> ${DEST_SKILL}/"

# -- Agents --
if [ "${INSTALL_AGENTS}" = true ] && [ -d "${SCRIPT_DIR}/agents" ]; then
  mkdir -p "${EXTRAS_BASE}/agents"
  cp "${SCRIPT_DIR}/agents/"*.md "${EXTRAS_BASE}/agents/" 2>/dev/null || true
  log "Agents installed -> ${EXTRAS_BASE}/agents/"
fi

# -- Commands --
if [ "${INSTALL_COMMANDS}" = true ] && [ -d "${SCRIPT_DIR}/commands" ]; then
  mkdir -p "${EXTRAS_BASE}/commands"
  cp "${SCRIPT_DIR}/commands/"*.md "${EXTRAS_BASE}/commands/" 2>/dev/null || true
  log "Commands installed -> ${EXTRAS_BASE}/commands/"
fi

# -- Rules --
if [ "${INSTALL_RULES}" = true ] && [ -d "${SCRIPT_DIR}/rules" ]; then
  mkdir -p "${EXTRAS_BASE}/rules"
  cp "${SCRIPT_DIR}/rules/"*.md "${EXTRAS_BASE}/rules/" 2>/dev/null || true
  log "Rules installed -> ${EXTRAS_BASE}/rules/"
fi

# -- Codex mirror --
if [ "${INSTALL_CODEX}" = true ]; then
  CODEX_DEST="${HOME}/.codex/skills/${SKILL_NAME}"
  rm -rf "${CODEX_DEST}"
  mkdir -p "${CODEX_DEST}"
  cp -r "${DEST_SKILL}/." "${CODEX_DEST}/"
  log "Codex mirror installed -> ${CODEX_DEST}/"
fi

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
echo ""
echo "=========================================================="
echo " ${SKILL_NAME} installed successfully."
echo "=========================================================="
echo " Restart Claude Code (or run /reload) to activate."
echo " Trigger with: 'audit this Solana program'"
echo " or commands: /audit-program  /static-scan  /write-report"
echo "=========================================================="
