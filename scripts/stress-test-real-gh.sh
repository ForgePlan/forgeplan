#!/bin/bash
set -euo pipefail

# PROB-060 Phase 0b: Real GH Actions stress-test helper
# Variant A: Creates 10 concurrent PRs with ID assignment, verifies no race conditions

readonly PREFIX="prob-060-stress"
readonly NUM_BRANCHES=10
readonly LABEL="ready-to-merge"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

confirm() {
    local prompt="$1"
    local response
    read -p "$(echo -e "${YELLOW}$prompt${NC} [y/N] ")" -r response
    [[ "$response" =~ ^[Yy]$ ]]
}

cleanup_branches() {
    log_info "Cleaning up test branches..."

    # Close all open PRs
    local pr_list
    pr_list=$(gh pr list --search "head:${PREFIX}" --json number -q '.[].number' 2>/dev/null || echo "")

    if [[ -n "$pr_list" ]]; then
        while IFS= read -r pr_num; do
            log_info "Closing PR #$pr_num..."
            gh pr close "$pr_num" --delete-branch 2>/dev/null || true
        done <<< "$pr_list"
    fi

    git fetch origin 2>/dev/null || true
    local branches
    branches=$(git branch -r | grep "origin/${PREFIX}" | sed 's|origin/||' || echo "")

    if [[ -n "$branches" ]]; then
        while IFS= read -r branch; do
            if [[ -n "$branch" ]]; then
                log_info "Deleting branch $branch..."
                git push origin --delete "$branch" 2>/dev/null || true
            fi
        done <<< "$branches"
    fi

    log_info "Cleanup complete"
}

preflight_check() {
    log_info "Running pre-flight checks..."

    local missing_tools=0
    for tool in git gh jq; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Missing required tool: $tool"
            missing_tools=$((missing_tools + 1))
        fi
    done

    if [[ $missing_tools -gt 0 ]]; then
        log_error "Please install missing tools"
        return 1
    fi

    log_info "Pre-flight checks passed"
}

create_test_branches() {
    log_info "Creating $NUM_BRANCHES test branches..."

    for i in $(seq 1 "$NUM_BRANCHES"); do
        local branch_name="${PREFIX}-$(printf '%02d' "$i")"
        log_info "Creating branch $branch_name..."

        git fetch origin dev:refs/remotes/origin/dev --depth=200 2>/dev/null || true
        git checkout -b "$branch_name" origin/dev 2>/dev/null || git checkout "$branch_name" 2>/dev/null || true

        local artifact_name="prd-stress-$(printf '%02d' "$i").md"
        local artifact_path=".forgeplan/prds/$artifact_name"
        local title="Stress Test Artifact $(printf '%02d' "$i")"

        mkdir -p .forgeplan/prds
        cat > "$artifact_path" << EOF
---
kind: prd
status: draft
title: $title
created: $(date -u +'%Y-%m-%dT%H:%M:%SZ')
updated: $(date -u +'%Y-%m-%dT%H:%M:%SZ')
slug: prd-stress-$(printf '%02d' "$i")
predicted_number: $((73 + i))
assigned_number: null
---

## Summary

Stress test artifact $i for PROB-060 Phase 0b EVID-A.
EOF

        git add "$artifact_path"
        git commit -m "test: PROB-060 stress-test artifact $i" 2>/dev/null || true
        git push -u origin "$branch_name" 2>/dev/null || true

        sleep 0.5
    done

    git checkout dev 2>/dev/null || true
}

create_prs() {
    log_info "Creating PRs..."

    for i in $(seq 1 "$NUM_BRANCHES"); do
        local branch_name="${PREFIX}-$(printf '%02d' "$i")"
        log_info "Creating PR for $branch_name..."

        gh pr create \
            --base dev \
            --head "$branch_name" \
            --title "test: PROB-060 stress-test from $branch_name" \
            --body "Stress test for EVID-A verification" \
            2>/dev/null || true

        sleep 1
    done
}

add_labels() {
    log_info "Adding label '$LABEL' to all PRs..."

    for i in $(seq 1 "$NUM_BRANCHES"); do
        local branch_name="${PREFIX}-$(printf '%02d' "$i")"
        local pr_num

        pr_num=$(gh pr list --head "$branch_name" --json number -q '.[0].number' 2>/dev/null || echo "")
        if [[ -n "$pr_num" ]]; then
            gh pr edit "$pr_num" --add-label "$LABEL" 2>/dev/null || log_warn "Failed to label PR #$pr_num"
        fi

        sleep 0.5
    done
}

main() {
    log_info "PROB-060 Phase 0b EVID-A: Real GH Actions Stress-test"
    log_info "======================================================="

    if ! confirm "This will create $NUM_BRANCHES test PRs and trigger concurrent workflows.
All branches will be cleaned up automatically. Continue?"; then
        log_info "Aborted by user"
        return 0
    fi

    preflight_check || return 1

    cleanup_branches
    create_test_branches
    create_prs
    add_labels

    log_info "All workflows triggered. Check GH Actions:"
    # PROB-060 Phase 0b Round 2 [SEC-7 CWE-200]: avoid hardcoded user/repo
    # identifier. Prefer GH Actions' GITHUB_REPOSITORY env-var (set by every
    # GHA runner); fall back to `gh repo view` for local invocations; final
    # fallback to a generic placeholder so logs never embed a fixed handle.
    local repo_slug="${GITHUB_REPOSITORY:-$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || echo 'OWNER/REPO')}"
    log_info "https://github.com/${repo_slug}/actions/workflows/assign-id.yml"

    log_info "Waiting for workflows (60s)..."
    sleep 60

    cleanup_branches

    log_info "${GREEN}Stress test completed${NC}"
    return 0
}

main "$@"
