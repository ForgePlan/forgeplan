# PROB-001: LanceDB data loss

## Signal

`rm -rf .forgeplan` уничтожает все артефакты безвозвратно. 21 артефакт + 5 evidence packs потеряны за секунду. Нет бэкапа, нет экспорта, нет recovery.

## Root Cause

.forgeplan/ в .gitignore (правильно — LanceDB binary data не для git). Но нет механизма экспорта артефактов в git-trackable формат.

## Suggested Fix

- `forgeplan export` — dump всех артефактов в JSON/YAML (git-trackable)
- `forgeplan import` — restore из dump
- Markdown projections должны обновляться при каждом update (не только при new)
