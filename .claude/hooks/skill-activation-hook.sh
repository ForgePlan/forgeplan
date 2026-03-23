#!/bin/bash
# UserPromptSubmit hook — Forgeplan methodology + skill activation
# Skills: /forge (structured workflow), /fpf-simple (first principles thinking)

cat <<'EOF'
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "FORGEPLAN METHODOLOGY ACTIVE.\n\nSkills available:\n- /forge — structured engineering workflow (route → create → validate → code → evidence → activate)\n- /fpf-simple — first principles thinking (decompose, evaluate, compare, reason)\n\nBefore non-trivial tasks:\n1. forgeplan health — check project state\n2. forgeplan route \"description\" — determine depth\n3. If Standard+ → Shape PRD before coding\n4. After coding → Evidence → Activate\n\nRust skills: rust-expert, rust-pro, m01-ownership, m06-error-handling\n\nMethodology: Shape → Validate → Code → Evidence → Activate"
  }
}
EOF
