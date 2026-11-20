#!/usr/bin/env python3
"""vLLM coder driver: spec + context files -> generated code -> apply.

Usage:
  vllm_coder.py <task_spec_file> [--ctx file1 file2 ...] [--out target_file]
  vllm_coder.py --gate <spec_file> <diff_or_code_file>

Modes:
  default : send spec + context to vLLM, print generated code (or write to --out)
  --gate  : quality-gate a diff/code file against spec, print JSON verdict
"""
import json
import subprocess
import sys
import urllib.request

VLLM_URL = "http://127.0.0.1:8000/v1/chat/completions"
MODEL = "qwen-coder"


def call(messages, max_tokens=4000, reasoning_effort="low", response_format=None):
    req = {
        "model": MODEL,
        "temperature": 0.1,
        "max_tokens": max_tokens,
        "messages": messages,
    }
    if reasoning_effort:
        req["reasoning_effort"] = reasoning_effort
    if response_format:
        req["response_format"] = response_format
    r = urllib.request.Request(
        VLLM_URL, data=json.dumps(req).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(r, timeout=180) as resp:
        d = json.loads(resp.read())
    m = d["choices"][0]["message"]
    return m.get("content") or m.get("reasoning") or ""


def gate(spec: str, code: str) -> dict:
    schema = {
        "type": "json_schema",
        "json_schema": {
            "name": "quality_gate",
            "strict": True,
            "schema": {
                "type": "object",
                "properties": {
                    "passed": {"type": "boolean"},
                    "score": {"type": "integer"},
                    "bugs": {"type": "array", "items": {"type": "string"}},
                    "missing_tests": {"type": "array", "items": {"type": "string"}},
                },
                "required": ["passed", "score", "bugs", "missing_tests"],
                "additionalProperties": False,
            },
        },
    }
    out = call(
        [
            {"role": "system", "content": (
                "You are a strict Principal Reviewer on a CI gate. Compare the diff/code "
                "against the SPEC. Set passed=false if any contract is violated, errors are "
                "unhandled, or required cases are untested. Output JSON only."
            )},
            {"role": "user", "content": f"=== SPEC ===\n{spec}\n\n=== CODE ===\n{code}"},
        ],
        max_tokens=1500,
        response_format=schema,
    )
    try:
        return json.loads(out)
    except json.JSONDecodeError:
        # extract first {...} block from reasoning-style output
        start = out.find("{")
        end = out.rfind("}")
        if start != -1 and end > start:
            try:
                return json.loads(out[start:end + 1])
            except json.JSONDecodeError:
                pass
        return {"passed": False, "score": 0, "bugs": [f"unparseable gate output: {out[:400]}"], "missing_tests": []}


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        sys.exit(2)

    if args[0] == "--gate":
        spec = open(args[1], encoding="utf-8").read()
        code_path = args[2]
        if code_path == "-":
            code = sys.stdin.read()
        elif code_path.endswith(".diff") or code_path == "git-diff":
            code = subprocess.check_output(["git", "diff"], text=True)
        else:
            code = open(code_path, encoding="utf-8").read()
        verdict = gate(spec, code)
        print(json.dumps(verdict, indent=2, ensure_ascii=False))
        sys.exit(0 if verdict.get("passed") else 1)

    spec_file = args[0]
    spec = open(spec_file, encoding="utf-8").read()
    ctx_files = []
    out_file = None
    i = 1
    while i < len(args):
        if args[i] == "--ctx":
            i += 1
            while i < len(args) and not args[i].startswith("--"):
                ctx_files.append(args[i])
                i += 1
        elif args[i] == "--out":
            out_file = args[i + 1]
            i += 2
        else:
            i += 1

    ctx = ""
    for f in ctx_files:
        content = open(f, encoding="utf-8").read()
        ctx += f"\n=== {f} ===\n{content}\n"

    out = call(
        [
            {"role": "system", "content": (
                "You are a Rust coder. Implement the SPEC exactly. Output ONLY the complete "
                "file content or unified diff — no prose, no markdown fences explanation."
            )},
            {"role": "user", "content": f"=== SPEC ===\n{spec}\n{ctx}"},
        ],
        max_tokens=8000,
        reasoning_effort="medium",
    )

    if out_file:
        with open(out_file, "w", encoding="utf-8") as fh:
            fh.write(out)
        print(f"wrote {out_file} ({len(out)} chars)")
    else:
        print(out)


if __name__ == "__main__":
    main()
