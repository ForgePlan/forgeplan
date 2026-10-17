---
depth: standard
id: EVID-178
kind: evidence
last_modified_at: 2026-10-16T21:55:39.633640+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: ADR-027
  relation: informs
- target: RFC-014
  relation: informs
status: active
title: ADR-027 RFC-014 final independent core-policy guardian and staged lifecycle gate
---

---
assigned_number: 178
created: 2026-10-16
predicted_number: 178
slug: evid-adr-027-rfc-014-final-independent-core-policy-guardian-and-staged
updated: 2026-10-16
---

# EVID-178: Финальный независимый guardian core policy ADR-027 / RFC-014

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

## Gate Verdict and Exact Authority

gate_verdict: PASS
gate_scope: current r2 core documentation-source acceptance and explicitly staged evidence-displacement lifecycle only
source_acceptance: PASS
new_source_findings: 0
historical_source_findings: SRC-H1/SRC-M1/SRC-M2/SRC-L1 fixed and independently verified on r2
current_decision_activation_ready: false
current_decision_score_gate: NOT_SATISFIED, ADR-027=0.30 and RFC-014=0.30
staged_evidence_lifecycle_authorized: true, only as ordered below
conditional_decision_activation_authorized: true, ONLY after actual successful displacement, observed scores >=0.50 and >0, validation and baseline checks below
unconditional_activation_authorized: false
activation_or_supersede_performed: false
runtime_verified: false
shipped_new_template_verified: false
website_or_producer_acceptance: false

**PASS принимает исправленный исходный регламент r2 и разрешает оркестратору только точную последовательность ниже. Он НЕ означает, что нынешний score уже достаточен.** Сначала завершение корректных Evidence, затем canonical вытеснение исторического r1 показания, затем новое фактическое измерение score/validation/актуальности. Только положительный результат последнего шага разрешает отдельную активацию решений. При любом отказе или несоблюдённом условии ADR-027/RFC-014 остаются draft; --force, raw status update, переписывание body и снятие неудобных links запрещены. Никакой автоматической активации этот документ не выполняет.

## Identity, Independence and Owner Authority

Дата review: 2026-10-17. Owning root `/Users/explosovebit/Work/ForgePlan`. Новый независимый OMP `openai-codex/gpt-6.1-sol`, назначенный High, approved run-local overlay, Profile B final guardian. Это не creator ADR/RFC, не MiniMax source writer и не контексты EVID-174/175/176/177. Дочерние агенты не запускались. Модель сообщена средой; High назначен поручением, независимая серверная телеметрия фактического effort недоступна. Единственный автор проекта Eli Rum, https://elirum.me. Эта identity не является доказательством четырёх фактически запущенных fictional S/C reviewers.

Полностью прочитан исходный owner permission `/Users/explosovebit/.omp/agent/sessions/-Work-GertsAi-gertsai_codex/2026-10-16T13-13-31-137Z_01a144d8-a501-7048-9171-61e9652343dd/local/website-core-policy-approval.md`; собственный raw SHA256 `6baeae5735372ed2e2a07bb0761c5aeb89643bd0fd5d7d8055f0361a9b3dc1c8` совпал. Ответ «Да, оформить и в core» на «Применить и в ForgePlan core принцип: независимый review может завершиться PASS без квоты на дефекты, если проверка полная и доказательства достаточны?»; одобрено «Отдельно оформить изменение регламента с независимой проверкой; остальные gates остаются обязательными.» Это достаточное основание независимо рассмотреть amendment без выдумки, не преждевременная общая сила draft и не owner acceptance любого риска.

Поручение финального guardian ограничивает запись одним новым Evidence и его informs ADR-027/RFC-014. Tracker остаётся у parent; website/producer, source/body fixes, paid reasoning/modelprovider/auth/config/Hindsight, build/tests/linters/formatter/server/browser/install/delete/commit/push/merge/release/deploy здесь не выполняются. Canonical activation/supersede делает только owning orchestrator после этого gate и фактических проверок, не guardian.

## Full Canonical Chain and Current Pins

Все объекты независимо получены `forgeplan get ID --json` из owning cwd, каждый exit0. Полностью прочитаны текущие ADR-027/RFC-014, active ADR-025/003 со всеми amendments, ADR-020, NOTE-054/055/056, вся независимая цепочка EVID-174/175/176/177, включая полные исходные engine outputs, fictional inputs, собственные ответы обоих source reviewers, metadata/read-path/probe/persistence appendices. Повторяющиеся JSON значения проверялись на точное равенство и читались через полный общий объект плюс исчерпывающие различия; никакие поля G1/G2/G3 или J не заменены ожидаемой таблицей. В частности полный EVID-177 имеет 158607 UTF-8 bytes и совпал с переданным pin, не с авторским summary. Protected записи получены для проверки сохранности, не нового website verdict.

Body SHA256 ниже — точные UTF-8 JSON body bytes, включая внутренний template YAML, без внешнего projection frontmatter/нормализации. Полные slug/status/depth/title/links закреплены в Appendix A.

- ADR-027: draft/deep, 43087 bytes, SHA256 `8478e0267c1eeb4d8f1442476ff49bf4f4744c56835678db264c7e7798cad6d1`.
- RFC-014: draft/deep, 44551 bytes, SHA256 `1e7412e50aad464695de1e0096c5371c3e1baabb0181ab3abdf0ecad84511423`.
- ADR-025: active/standard, 4918 bytes, SHA256 `3e3ca3782908b1c81ddbedab0aa816dc54b702c8cd303171a5ded8088e6e7652`.
- ADR-003: active/standard, 17857 bytes, SHA256 `8d034028af55e8ecaa1c2a5047322a6997c903f71c75dc78e2112d748a9ac763`.
- ADR-020: active/standard, 13308 bytes, SHA256 `861a0e39647daea3362db9e163bc4dfbdafa1c2398b1de1e1512af44b543985e`.
- NOTE-054: draft/tactical, 42134 bytes, SHA256 `f884501e650d1ad24e848e3cf9d8934d09254de6103fa91345e575815f28f317`.
- NOTE-055: draft/standard, 14877 bytes, SHA256 `d32b2ba178e65e0f797ac209835e23f213cf045c297d21e842a456c59cb259e8`.
- NOTE-056: draft/tactical, 40192 bytes, SHA256 `67d1aa41749f82a7efec0f13680d9bda8ce724bf559c9df26b7bb70b54a10e51`.
- EVID-174: draft/standard, 40701 bytes, SHA256 `0f5668c6302d0a1591264a5183a5f962c3c9d354f438d4d9193bcc6ee5fad8af`.
- EVID-175: draft/standard, 49364 bytes, SHA256 `2932fe565d8d5d9c95aaf944e4921f407af77810fd6fa348aba0a58fc1f32fb2`.
- EVID-176: draft/standard, 145129 bytes, SHA256 `3d0bb5864be86caa3d66d5877cec0809c42a1f9903e00060e1eacb368c604a2a`.
- EVID-177: draft/standard, 158607 bytes, SHA256 `639436a15987ea1bccb1016cfb92b4c7d718a894270cacd8449df394b53caea3`.
- EVID-172: draft/tactical, 22610 bytes, SHA256 `166459e3b24ace7fab5effaa917094914b38cdb09bba1e7c8c39e52ec74171d2`.
- EVID-173: draft/standard, 33806 bytes, SHA256 `9bde8017f7bf2a4ea81e137534085e712b137f634005c3a796d136974d1adb6e`.
- NOTE-053: draft/standard, 40187 bytes, SHA256 `961b084cab0b0ca1ee60c49a8f08668490301ef3e4a01128aea822e601ab2783`.
- PRD-087: draft/deep, 34406 bytes, SHA256 `803d422426a6826066c17ed165f28564e7979b5af429cad62d4f2d91d149a83a`.
- SPEC-007: draft/deep, 39292 bytes, SHA256 `5adc34c6f86ca693580b86c8208b304ff79658b7978b781efb3788850ff60678`.

ADR-027 slug `adr-finite-evidence-backed-independent-review-without-a-defect-quota`; RFC-014 slug `rfc-core-review-policy-bounded-documentation-cutover`. Оба draft/deep. Значимые actual links: ADR based_on active ADR-025 и ADR-003, informs RFC; RFC based_on тех же родителей; NOTE-054 informs RFC, NOTE-055/056 informs ADR; EVID-174/175/176/177 informs оба. EVID-176 draft/standard weakens, historical source BLOCKER, body и все четыре старых finding сохранены. EVID-177 draft/standard supports, source-only PASS, activation_authorized=false. Новая запись не скрывает EVID-176 из цепочки.

EVID-174 и EVID-175 проверили проект и pre-source последовательность, не r2 implementation. EVID-176 действительно нашёл расхождения на r1; EVID-177 самостоятельно проверил четыре исправления и новый candidate. Этот guardian заново сопоставил normative contract с actual source и всей историей, а не принял MiniMax self-report.

## Actual Source Identity, Preservation and Read Path

Собственные read-only identity checks: branch `docs/freshness-sweep`, HEAD `db96045c0fac5188f4b60d08c3e447f53c1ba0dd`; commits since RFC base=0. HEAD не выдан за dirty candidate. Все17 raw file SHA пересчитаны независимо; encoding в RFC order `sha256  relative-path\n`; manifest SHA256 **98b2bd7870314d18e1b89af9e59a43dcb19e3b5f627843eaef8f0846ad7ebb43**. Все17 bytes равны `/tmp/forgeplan-minimax-core-review-20261016`; worker dirty diff paths ровно17 allowlisted. HEAD bytes всех17 равны original RFC before hashes. Actual scoped diff прочитан полностью, 192 insertions/88 deletions.

Собственное чтение: оба QUALITY-GATES полностью, root redlines/pipeline/architecture/audit требования, все изменённые строки полного diff и актуальные окружающие разделы всех17 путей, operative prompts внутри/после fence, schema checklist и template author comments. Полные17-file source reads независимо выполнены EVID-177; совпадение всех текущих hashes подтверждает применимость именно этого полного чтения. Guardian не приписывает себе повторное прочтение всех исторических release paragraphs CHANGELOG: здесь они проверены на отсутствие delta, а не переаудированы.

Путь реально пройден CLAUDE:516/685 → EN/RU QUALITY-GATES §2 → обе PRD-SCHEMA:73–82 и оба USAGE prompts:118–135 → RFC AC1–5 / full fictional inputs → actual independent results / этот Evidence. Цели ADR/RFC канонически прочитаны. Все39 reference targets/anchor checks EVID-177 проверены против текущих файлов; оба heading `## 2. Adversarial Review Protocol` существуют. Comments template:277/279 ведут к прочитанным QUALITY-GATES/GLOSSARY. Ссылки в GUIDE/COURSE code fences являются буквальными read-path ссылками, не обещанием отрисованной навигации. Обе after-prompt Markdown ссылки на :135 существуют и ведут к правильному языку.

QUALITY-GATES обоих языков вне §2 побайтно равны HEAD: Verification Gate, BMAD13, R_eff/min/CL/decay/threshold/depth не изменены. Diff root сохраняет redline7, validation/ADI/activation, 2+ audit, trigger ≥5 commits OR new public API и security/code роли. Полный протокол остаётся один в §2; другие15 мест — краткие правила/ссылки, не второй стандарт.

Current full-tree snapshot ДО создания этого EVID: 1790 существующих tracked/nonignored untracked regular/symlink entries, SHA256 `a501467a63637d780be1a559d70a535ca453d27ecdff1fa7fdb4ea24a0dff844`. Manifest sorted by relative path, строки `kind sha256  path\n`; symlink hash по bytes цели без перехода. Это проверка сохранности, не clean-tree/semantic audit всех файлов. Исключение только EVID-177 и снятие ровно одного documented appended LF с двух NONprotected ADR/RFC projections воспроизвело EVID-177 pre-write snapshot: 1789 entries, exact `c982696cbc3e681bc2419003b53659e6e7a9a0c73d1cfd91843490b4bed69f65`. Значит после r2 independent review нет скрытой source/user delta; canonical additions/framing отделены. Чужие Cargo/config/.cargo/docx/PRD024/047 dirty не сброшены.

Protected raw projections5/5 совпали с RFC/EVID177; canonical body/status/depth/slug/title/inbound/outbound всех15 записей EVID177 appendix совпали, кроме ожидаемых EVID177 informs additions у ADR/RFC. EVID172/173, NOTE053, PRD087/SPEC007 защищены целиком. Ни одного нового edge к ним не создаётся. Все17 source bytes повторно совпали непосредственно перед подготовкой записи.

## Finding Closure — Independent Determination

| Finding | Severity retained | Actual current location | Guardian disposition / why |
|---|---|---|---|
| SRC-H1 | HIGH | QUALITY-GATES EN/RU:96 | fixed, independently verified. Теперь draft явно не действует сам; precedence только после actual bounded application, независимой проверки и orchestrator lifecycle. Узкое owner amendment-review authority не даёт общей activation/runtime силы. D4 восстановлен. |
| SRC-M1 | MEDIUM | QUALITY-GATES EN/RU:88 | fixed, independently verified. Повторы определяются закрытием реальной проблемы И существующими gates; искусственного cap/неуказанного active-parent ceiling нет. Critical2/числа/профили сохраняются. |
| SRC-M2 | MEDIUM | QUALITY-GATES EN/RU:107–117 | fixed, independently verified. PASS — конъюнкция complete criteria/evidence/rounds/preserved gates. Retained justified Warning/MEDIUM/LOW имеют явный terminal путь без удаления findings; optional nondefect observation тоже не создаёт обязательную квоту. HIGH/mandatory-gap запреты действуют одновременно, первая ветвь не обход их. Reviewer проверяет разрешение, не принимает риск за владельца. |
| SRC-L1 | LOW | USAGE-BY-ROLE EN/RU:132–135 | fixed, independently verified. Правильный operative текст внутри fence, отдельная краткая ссылка после fence:135 в обоих языках. Target/anchor прочитаны. |

Новых substantive source findings:0. Старый EVID-176 r1 BLOCKER остаётся исторически истинным; здесь состояние исправлений относится только к r2. Никакого actual runtime bypass/потери безопасности не утверждается: r1 finding был дефектом текста, а не наблюдённым инцидентом.

## Complete Interpretation Evidence, Not Expected Results

Полные15 inputs EVID176 и EVID177 точно равны. Compact JSON SHA256 `05837ad5100c2abc596531d48f93d71818f6a6bf5e6ecc730bf64aa85ed48216`; каждый actual target body digest отдельно проверен. SYN-r1:268 bytes/SHA `a4470a220071f5377e876bf8d267e2fbd1360889935f27863dd080ad1d0f6bd2`; SYN-r2:317 bytes/SHA `032fb7d99ccaeb444c42dd78d0e433ffcf5628eba8740443c6933a5256de68d6`. Каждый quote/line согласован с полным target. Прочитаны все K1–K5/KR и fictional review records S1/C1/S2/C2, не только названия cases.

| Case | Actual r1 result → actual r2 result | Guardian assessment after reading input and both answers |
|---|---|---|
| A | PASS → PASS | Четыре текущие строки/K1–K4, G!=V, полное scoped покрытие; допустим только fictional local design, не automatic green. |
| B | CONCERNS → CONCERNS | Обязательная K3 skipped без результата; нельзя задним числом заменить отсутствующую запись V собственным чтением. |
| C | CONCERNS → CONCERNS | r2 добавляет Critical clause, reviewed r1 устарел; требуется новая проверка дельты/K5, не перенос verdict. |
| D | BLOCKER → BLOCKER | Заданный H-001 open HIGH/resolution=null блокирует. Allegation не доказана самим r1:4; это premise для disposition, не реальная core уязвимость. |
| E | CONCERNS → BLOCKER | G==G при другой session label не независимость. Обе оценки исключают PASS; r2 более строгая severity не новый source defect. |
| F | PASS → PASS | Runtime действительно вне design criteria; результат не выходит за design. |
| F2 | BLOCKER → BLOCKER | KR required/unavailable/null и producer absent; design результат не закрывает whole/runtime gate. |
| G1 | CONCERNS → BLOCKER | S/C оба только round1; top-level K5 satisfied не доказывает round2. |
| G2 | CONCERNS → BLOCKER | C rounds1/2 не заменяет отсутствующий security профиль S. |
| G3 | PASS → PASS | Четыре полные current records S1/C1/S2/C2 с содержательными adverse checks и пятью цитатами; fictional validation/ADI/BMAD/R_eff не реальные quorum/runtime measurements. |
| N-empty | CONCERNS → CONCERNS | checks=[], counterargument=null и all good не подтверждают coverage. |
| N-dirty | CONCERNS → CONCERNS | Dirty r2 вне claimed commit-only pin; reviewed r1 не current bytes. |
| N-skipped-as-NA | CONCERNS → CONCERNS | required K3/null evidence не становится неприменимым из-за нуля findings. |
| J-authorized-warning | CONCERNS → PASS | Отличающийся исход действительно обусловлен corrected Process5: retained W-001 Warning, явное fictional owner authorization, mandatory_criteria_failed=[]; без стирания/понижения finding. Нет waiver HIGH/mandatory. |
| P-observation | PASS → PASS | Открытый optional O-001 severity Pass не реальный дефект; observation severity сама не gate verdict. |

Это проверка смысловой достаточности фактического независимого применения source, не новый запуск production/security/API, не повтор source-authoring и не empirical сравнение процессов. Согласен с выводами r2 только в указанных границах. Содержательное различие J подтверждает, что новая эвиденция не копирует старую expected table. Fictional owner не принимал реальный риск проекта, fictional S/C не засчитаны за actual reviewers.

## Architecture, ADI and All Preserved Gates

1. **Active architecture / redline7.** Самостоятельно согласен с EVID174/175: ADR-025 boundary test оставляет исполнение процесса у внешнего оркестратора, core хранит договорённости/verdict/history. ADR-003 задаёт file-first canonical mutation. Actual17-file instruction cutover не новый runtime/API/storage и является реализацией именно этого docs-only решения; не объявлением Markdown Rust-кодом и не общим waiver. Реализация уже есть, независимый r2 source Evidence есть. Новый active RFC до source не требовался под применимыми active parents. Новые решения до завершающих lifecycle условий всё ещё draft.
2. **Deep ADI.** Прочитан полный authored ADI с настоящими альтернативами/следствиями/отклонениями/границами, плюс два отдельных complete engine analyses: RFC NOTE054 3 hypotheses/3 deductions/8 evidence-needed, successful exit0; ADR NOTE056 4/4/10, successful saved completion wall56.66s. NOTE055 exit1 остаётся failure без accepted analysis, login hint не диагноз auth. Exact current RFC prefix воспроизвёл input `24292ed4a1be007c9def4b7b7e3531cafaaf3ea0a470dabe0b35efd4df5bd7a3`, ADR prefix — `e757631639c42e4b44f38a93c9dddb16c260400f465d2b45fb049c3cb5d49da2`; поздние sections только factual execution/handoff, не normative delta. Requested model/high отражены в сохранённых argv; server effort не измерен. Engine не independent source reviewer. Нового engine вызова не нужно для закрытия уже доказанного prerequisite.
3. **Roles/count/trigger.** Сохраняется общий 2+ independent audit: EVID177 actual source/code-policy reviewer и этот отдельный guardian непосредственно проверяют текущий r2, EVID174/175 — отдельно прежний design. EVID176 не переименован в текущий PASS. Trigger специализированного security+code panel — ≥5 commits OR new public API. В этой bounded delta0 новых commits с base и только17 docs/template/changelog, нового API нет; trigger не достигнут. Это обоснованная неприменимость специализированного panel к этой приёмке, не утверждение, что один verifier исполнил security роль. При расширении delta trigger перепроверяется. Deep остаётся Deep, Critical2 не отменён; G1/G2 это независимо демонстрируют, а G3 не выдаётся за реальные раунды.
4. **Verification Gate5.** Следствия D1–D4 сопоставлены с реальным текстом и adverse inputs; сильнейший контраргумент ниже; не только creator self-evidence — owner source, ранее active parents и четыре отдельные независимые EVID; tail risks и слабое звено ниже. CL3 относится к same-context direct audit текущего source/chain, не к эмпирическому улучшению качества.
5. **BMAD13 в применимом объёме.** Тип ADR/RFC соответствует policy/no new API; обязательные sections validate; текст/объём narrow; owner brief покрыт;17 paths/AC измеримы; traceability owner→D1–D4→AC→source→cases→Evidence; техническое HOW уместно в RFC (PRD leakage/persona требования не навязаны отсутствующему новому PRD); domain authority/security/consent учтены; CLI embedded/source различены; SMART/current pins есть; holistic4/5 как оценка документации, не R_eff; implementation checklist старого design snapshot не выдан за текущие выполненные автором галочки; completeness доказывает actual evidence; finalization этим canonical report. Обязательные проверки не заменены N/A.
6. **Runtime/security/consent.** Нового исполняемого поведения, network/auth/permissions/API изменения нет. Новый scan/build/runtime не acceptance criterion этих source docs и явно запрещён поручением. Реальный read-path/interpretation smoke и прежнее actual CLI template measurement учтены, не объявлены тестами runtime. Consent — exact owner authorization; без принятия reviewer чужого риска. Website/runtime/producer/security acceptance иных поверхностей не закрываются.
7. **Score/validation/activation.** Никакое содержательное PASS не отменяет нормативный R_eff>=0.50 для решений или redline7>0. Текущий score ниже порога; допустимо только staged решение ниже. Structural validate — не independent acceptance. Lifecycle guards сохраняются; generic hints не authorization.

## Template Probe Reuse — Actual Measurement, Unchanged Inputs

Полностью прочитан retained EVID176 init/new/get receipt, повторно полностью сохранённый EVID177: `/tmp/forgeplan-policy-source-verifier-gIndFS`, каждый exit0, один действительно созданный и прочитанный PRD. get stdout11924 bytes/SHA `8321dede4a1b8959e96cfd4a88b4835442af59d3bb34ceab4ee2c481ac274255`; body SHA `55e5a2c730aa7dd939df42fd85aed5948dfbc79ebe3e4a6970cf0be037d68271`. Новый probe мной не запускался. Сейчас отдельно прочитаны actual saved comments: «Ревьюер ОБЯЗАН найти минимум 1 проблему.» / «0 найденных проблем = недостаточно тщательный review.»

Собственные raw hashes: binary `/opt/homebrew/bin/forgeplan` `55c32b6d230b4ebd56a6c4e3f38a6fb20b5c31f8ee91558c0850fc92c3771113` (retained0.38.0); template source `75418bd03761723dcc596d25dd922015f759aba7eff7a5d6cc0dd66cdae59363`; saved probe projection `e9a8a4279f329919b4a99bb31ba0f2e4c2cd92ea95c72213fa373f090b891e92`, все прежние. Full-tree reconstruction подтверждает сохранность других relevant owning inputs/config. Четыре r2 corrections не меняют emission inputs. Reuse достаточен только для вывода OLD emitted embedded template / changed source template. Retained help не нашёл supported template override; никаких guessed flags/build/install не применяется. Это прямо допустимая RFC AC4 граница, не shipped-new-template PASS и не основание менять норму приёмки. Shipped/public/installed requirement — отдельный revisit trigger.

## Observed Validation and Scores — Before Displacement

Собственные `forgeplan validate <ID> --json` для ADR027/RFC014/EVID174/175/176/177: каждый exit0, passed=true, errors0,warnings0,findings=[]. Existing stderr Session routing→coding/create-prd advisory оставлен без repair; generic activate hints не исполнены. `forgeplan score <ID> --json` реально выполнен для тех же6 объектов, каждый exit0. Полные два decision score JSON сохранены Appendix C.

- ADR-027: **R_eff0.30**, weakest_link ADR-025. Four direct evidence: E174/E175/E177 Supports CL3 score1.0; E176 Weakens CL3 score0.5; все draft/nonexcluded/nonexpired. Factors: CL penalty ADR025, CL penalty ADR003, skipped RFC014 draft. CI point0.5, evidence_count4 — не final recursive R_eff.
- RFC-014: **R_eff0.30**, weakest_link ADR-025; те же direct evidence, CL penalties обоих active parents. CI point0.5 не разрешает активацию.
- Leaf EVID174/175/177 own score каждый **0.80**; leaf EVID176 own score **0.30**, actual factor `Leaf evidence scored on its own fields: Weakens CL3 = 0.30`. Это отличается от0.50 direct contribution EVID176 в двух decision breakdowns; цифры не смешаны и не исправлены body/CL манипуляцией. Leaf Standard Evidence — завершённая запись отрицательного измерения, не решение о принятии политики. Низкий отрицательный receipt score не делает receipt неистинным.

Health фактически exit0: total451, needs_attention,2 at-risk PRD005/RFC004,10 advisory mismatches; unrelated debt не исправлялся, whole-project healthy не заявлено.

## Explicit Staged Lifecycle Permission and Stop Conditions

Это конечное ограниченное разрешение, не проект нового source/website цикла. Основание вытеснения — исправления реально проверены другой independent EVID177/current text, не желание повысить score. ADR020 active и QUALITY-GATES §4 сохраняют terminal packs в графе/истории, исключая их из CURRENT weakest-link min; draft продолжает участвовать. Поэтому одна новая supports запись или activation старой НЕ вытесняет r1 автоматически.

**Stage1 — завершить Evidence, не принять policy.** Оркестратору разрешены canonical validate/get/score и normal `forgeplan activate EVID-174`, EVID-175, EVID-177 и этот EVID-178 как завершённых независимых записей их точного заявленного объёма. Сначала актуальные validation/body/status/links/pins, затем успешная activation каждого; не --force. EVID177 replacement предпочтительно activate до old displacement, без draft-successor warning. Допустима normal activation EVID176 **исключительно как завершённого исторического r1 BLOCKER measurement**, сохраняя weakens/CL3/audit/body/findings/informs. Это не source/policy PASS. EVID176 должен стать active до supersede: текущий supported canonical route — active→superseded, не draft→superseded.

Почему Stage1 не нарушает R_eff0.50 decision threshold: активируются Standard Evidence receipts, включая отрицательный, а не Deep решения. Root запрет «без реализации/evidence» не требует бесконечной цепочки Evidence для самого Evidence: выполненная независимая проверка с полным материалом — реализация этой записи, positive leaf scores измерены. Normative0.50 — принятие решения; score optional для Standard по §5. Даже машинная разрешимость не отменяет ручную проверку полноты. Actual CLI supersede/activate help и прочитанные `crates/forgeplan-core/src/lifecycle/transitions.rs:13–45`, `lifecycle/mod.rs:60–98,143–157,327–337,424–467` подтверждают разделение lightweight evidence completion и decision gates. Local source — corroboration supported contract, не утверждение, что будущая команда уже прошла на установленном binary. Если каноническая команда отказывает — остановка, без invented flag, force, status bypass или нового reasoning.

**Stage2 — честное вытеснение r1.** Только после существующей/завершённой current independent replacement и Stage1 разрешено `forgeplan supersede EVID-176 --by EVID-177` в owning cwd. CLI help прямо active→superseded; актуальная таблица переходов не допускает draft→superseded. Поэтому здесь не разрешён пробный raw draft transition. Сохранить EVID176 body SHA/weakens/BLOCKER/четыре finding и оба informs; добавить только canonical replacement relation/status/lifecycle metadata. После команды readback обеих записей/отношения и body pins. Не unlink, не deprecate вместо replacement, не переписать verdict, не трогать protected EVID172/173. Canonical edge orientation принимается по actual operation, не ручная перестановка ради словаря: checkout implementation пишет old→new supersedes; проверить реальный результат, не выдумывать обратное ребро.

**Stage3 — обязательный observed gate, не прогноз.** После displacement заново `forgeplan score ADR-027 --json`, `forgeplan score RFC-014 --json`, `forgeplan validate ADR-027 --json`, `forgeplan validate RFC-014 --json`. Нужны actual nonzero successful output и независимая проверка: оба R_eff>=0.50 И >0; EVID176 виден как excluded terminal в breakdown/factors, EVID177/178 current, parent CL penalties не спрятаны; validation passed/errors0. Прогноз повышения score здесь НЕ наблюдён и не основание шага4. Нельзя редактировать active parents, CL, graph, evidence body или порог для устранения остаточного слабого звена. Если хотя бы один score ниже0.50 либо gate не проходит, **STOP: policy остаётся draft, precise blocker — фактический residual score/gate**, этот PASS не waiver и не разрешение расширить scope.

Перед Stage4 перепроверить source17 manifest, оба нормативных body pins/slug/depth/outbound, полный chain и protected baseline. Разрешённые Stage1/2 metadata/статус/новая replacement связь — ожидаемые выходы этого review, не unseen normative delta и не причина обязательного нового source-authoring. Иные relevant body/source/criteria/evidence изменения требуют fresh independent review; нерелевантный framing LF — только с доказанным равенством canonical body и recorded explanation. Существенная новая находка/обязательный пропуск немедленно отменяют Stage4 authority.

**Stage4 — только условная decision activation.** Если и только если Stage1–3 реально прошли с указанными facts/pins и остальными preserved prerequisites, этот финальный guardian разрешает оркестратору normal canonical `forgeplan activate ADR-027`, затем RFC-014, без force/raw status update. Между ними вновь измерить оба score/validation: activation первого может изменить recursive dependency calculation у второго; при ухудшении ниже0.50 STOP до второго. После обоих readback статусов/body/links/source и оба score>=0.50. Никакое создание нового RFC/ADR, повтор reasoning или новый source cycle на неизменной baseline не требуется только ради ещё одного формального gate. Это явное условное разрешение exact staged sequence, не утверждение нынешней достаточности0.30 и не automatic authority при провале Stage3. При failure первого/второго не скрывать частичное состояние и не делать raw rollback.

Весь lifecycle исполняет только owning orchestrator. Этот guardian ничего не активировал/не supersede-ил. Нынешняя policy остаётся draft; general D4 precedence наступает только после действительного выполнения требуемых lifecycle действий. Нет website Ready, EVID173 P1 re-review, producer или implementation/publication authority.

## Strongest Counterargument, Limits and Revisit

Главный риск — назвать удаления неудобного слабого evidence «вытеснением», а условный PASS показать как готовую policy activation. Против этого: full historical r1 BLOCKER сохранён; другой independent reviewer и current source закрывают все4 конкретных условия; supersedes оставляет историю/links; Stage3 измеряет настоящий recursive score с parent penalties; missing threshold не объявлен N/A. Риск формального заполнения coverage остаётся: данный review подтверждает проверяемость и текст, не невозможность ошибки/сговора и не measured production benefit.

Tail scenarios: relevant dirty bytes меняются после verdict; future actor использует design/source PASS как runtime/website permission; canonical persistence теряет body/history. Контроль: re-pin/re-review, отдельные gates/явные scopes, full body/metadata/raw readback. Вероятности неизвестны, не выдуманы. Rollback — остановить ошибочный переход, сохранить новый Evidence и получить owning решение, не стирать историю и не автоматически восстанавливать квоту.

Слабое звено сейчас — **не source findings, а всё ещё неудовлетворённый фактический score threshold и законное завершение lifecycle**. Оно разрешимо только указанными реальными действиями/наблюдением; гарантии результата нет. При residual failure достижимая работа guardian завершена этим доказательным report, Deep решения остаются draft с точным blocker.

## Persistence Contract

Создан только этот новый canonical EVID-178, draft/standard, informs ровно ADR-027/RFC-014, без auto-activation. После обеих links собственный canonical readback/validate проверяет полное тело и status/depth/slug/links, отсутствие source/protected/previous-evidence body changes. Фактически наблюдённые результаты этой проверки добавляются отдельным checkpoint; final hash сообщается снаружи тела, не самоссылкой. Никакие старые записи/источники guardian не исправляет.

## Appendix A — Exact Pre-existing Canonical Metadata Pins

```json
{
  "ADR-027": {
    "status": "draft",
    "depth": "deep",
    "slug": "adr-finite-evidence-backed-independent-review-without-a-defect-quota",
    "title": "Finite evidence-backed independent review without a defect quota",
    "body_bytes": 43087,
    "body_sha256": "8478e0267c1eeb4d8f1442476ff49bf4f4744c56835678db264c7e7798cad6d1",
    "links": {
      "inbound": [
        {
          "relation": "informs",
          "source": "NOTE-055"
        },
        {
          "relation": "informs",
          "source": "NOTE-056"
        },
        {
          "relation": "informs",
          "source": "EVID-174"
        },
        {
          "relation": "informs",
          "source": "EVID-175"
        },
        {
          "relation": "informs",
          "source": "EVID-176"
        },
        {
          "relation": "informs",
          "source": "EVID-177"
        }
      ],
      "outbound": [
        {
          "relation": "based_on",
          "target": "ADR-025"
        },
        {
          "relation": "based_on",
          "target": "ADR-003"
        },
        {
          "relation": "informs",
          "target": "RFC-014"
        }
      ]
    }
  },
  "RFC-014": {
    "status": "draft",
    "depth": "deep",
    "slug": "rfc-core-review-policy-bounded-documentation-cutover",
    "title": "Core review policy bounded documentation cutover",
    "body_bytes": 44551,
    "body_sha256": "1e7412e50aad464695de1e0096c5371c3e1baabb0181ab3abdf0ecad84511423",
    "links": {
      "inbound": [
        {
          "relation": "informs",
          "source": "ADR-027"
        },
        {
          "relation": "informs",
          "source": "NOTE-054"
        },
        {
          "relation": "informs",
          "source": "EVID-174"
        },
        {
          "relation": "informs",
          "source": "EVID-175"
        },
        {
          "relation": "informs",
          "source": "EVID-176"
        },
        {
          "relation": "informs",
          "source": "EVID-177"
        }
      ],
      "outbound": [
        {
          "relation": "based_on",
          "target": "ADR-025"
        },
        {
          "relation": "based_on",
          "target": "ADR-003"
        }
      ]
    }
  },
  "ADR-025": {
    "status": "active",
    "depth": "standard",
    "slug": "adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer",
    "title": "Orchestration sits above ForgePlan; the core is the contract layer",
    "body_bytes": 4918,
    "body_sha256": "3e3ca3782908b1c81ddbedab0aa816dc54b702c8cd303171a5ded8088e6e7652",
    "links": {
      "inbound": [
        {
          "relation": "based_on",
          "source": "ADR-026"
        },
        {
          "relation": "informs",
          "source": "EVID-169"
        },
        {
          "relation": "based_on",
          "source": "ADR-027"
        },
        {
          "relation": "based_on",
          "source": "RFC-014"
        }
      ],
      "outbound": [
        {
          "relation": "based_on",
          "target": "ADR-001"
        },
        {
          "relation": "refines",
          "target": "ADR-009"
        }
      ]
    }
  },
  "ADR-003": {
    "status": "active",
    "depth": "standard",
    "slug": null,
    "title": "Markdown files as source of truth — LanceDB as index layer",
    "body_bytes": 17857,
    "body_sha256": "8d034028af55e8ecaa1c2a5047322a6997c903f71c75dc78e2112d748a9ac763",
    "links": {
      "inbound": [
        {
          "relation": "informs",
          "source": "PRD-044"
        },
        {
          "relation": "informs",
          "source": "PRD-001"
        },
        {
          "relation": "informs",
          "source": "PRD-078"
        },
        {
          "relation": "based_on",
          "source": "PRD-073"
        },
        {
          "relation": "based_on",
          "source": "RFC-004"
        },
        {
          "relation": "based_on",
          "source": "RFC-003"
        },
        {
          "relation": "informs",
          "source": "RFC-010"
        },
        {
          "relation": "informs",
          "source": "ADR-015"
        },
        {
          "relation": "based_on",
          "source": "ADR-018"
        },
        {
          "relation": "informs",
          "source": "PROB-074"
        },
        {
          "relation": "informs",
          "source": "PROB-073"
        },
        {
          "relation": "contradicts",
          "source": "PROB-072"
        },
        {
          "relation": "based_on",
          "source": "PROB-048"
        },
        {
          "relation": "informs",
          "source": "PROB-075"
        },
        {
          "relation": "informs",
          "source": "PROB-061"
        },
        {
          "relation": "informs",
          "source": "PROB-062"
        },
        {
          "relation": "based_on",
          "source": "PROB-082"
        },
        {
          "relation": "informs",
          "source": "PROB-047"
        },
        {
          "relation": "informs",
          "source": "EVID-098"
        },
        {
          "relation": "informs",
          "source": "EVID-095"
        },
        {
          "relation": "informs",
          "source": "EVID-039"
        },
        {
          "relation": "informs",
          "source": "EVID-094"
        },
        {
          "relation": "informs",
          "source": "NOTE-015"
        },
        {
          "relation": "informs",
          "source": "NOTE-050"
        },
        {
          "relation": "informs",
          "source": "NOTE-042"
        },
        {
          "relation": "refines",
          "source": "ADR-026"
        },
        {
          "relation": "based_on",
          "source": "ADR-027"
        },
        {
          "relation": "based_on",
          "source": "RFC-014"
        }
      ],
      "outbound": [
        {
          "relation": "informs",
          "target": "PROB-014"
        }
      ]
    }
  },
  "ADR-020": {
    "status": "active",
    "depth": "standard",
    "slug": null,
    "title": "R_eff excludes terminal-status evidence from the weakest-link min",
    "body_bytes": 13308,
    "body_sha256": "861a0e39647daea3362db9e163bc4dfbdafa1c2398b1de1e1512af44b543985e",
    "links": {
      "inbound": [
        {
          "relation": "informs",
          "source": "PROB-084"
        },
        {
          "relation": "informs",
          "source": "EVID-154"
        },
        {
          "relation": "informs",
          "source": "EVID-156"
        },
        {
          "relation": "based_on",
          "source": "PRD-086"
        }
      ],
      "outbound": [
        {
          "relation": "refines",
          "target": "ADR-002"
        }
      ]
    }
  },
  "NOTE-054": {
    "status": "draft",
    "depth": "tactical",
    "slug": null,
    "title": "ADI analysis of RFC-014",
    "body_bytes": 42134,
    "body_sha256": "f884501e650d1ad24e848e3cf9d8934d09254de6103fa91345e575815f28f317",
    "links": {
      "inbound": [],
      "outbound": [
        {
          "relation": "informs",
          "target": "RFC-014"
        }
      ]
    }
  },
  "NOTE-055": {
    "status": "draft",
    "depth": "standard",
    "slug": "note-adr-027-bounded-deep-engine-invocation-failure-and-preserved-receipts",
    "title": "ADR-027 bounded Deep engine invocation failure and preserved receipts",
    "body_bytes": 14877,
    "body_sha256": "d32b2ba178e65e0f797ac209835e23f213cf045c297d21e842a456c59cb259e8",
    "links": {
      "inbound": [],
      "outbound": [
        {
          "relation": "informs",
          "target": "ADR-027"
        }
      ]
    }
  },
  "NOTE-056": {
    "status": "draft",
    "depth": "tactical",
    "slug": null,
    "title": "ADI analysis of ADR-027",
    "body_bytes": 40192,
    "body_sha256": "67d1aa41749f82a7efec0f13680d9bda8ce724bf559c9df26b7bb70b54a10e51",
    "links": {
      "inbound": [],
      "outbound": [
        {
          "relation": "informs",
          "target": "ADR-027"
        }
      ]
    }
  },
  "EVID-174": {
    "status": "draft",
    "depth": "standard",
    "slug": "evid-adr-027-rfc-014-independent-pre-source-architecture-and-policy-review",
    "title": "ADR-027 RFC-014 independent pre-source architecture and policy review",
    "body_bytes": 40701,
    "body_sha256": "0f5668c6302d0a1591264a5183a5f962c3c9d354f438d4d9193bcc6ee5fad8af",
    "links": {
      "inbound": [],
      "outbound": [
        {
          "relation": "informs",
          "target": "ADR-027"
        },
        {
          "relation": "informs",
          "target": "RFC-014"
        }
      ]
    }
  },
  "EVID-175": {
    "status": "draft",
    "depth": "standard",
    "slug": "evid-adr-027-rfc-014-independent-guardian-source-phase-gate",
    "title": "ADR-027 RFC-014 independent guardian source-phase gate",
    "body_bytes": 49364,
    "body_sha256": "2932fe565d8d5d9c95aaf944e4921f407af77810fd6fa348aba0a58fc1f32fb2",
    "links": {
      "inbound": [],
      "outbound": [
        {
          "relation": "informs",
          "target": "ADR-027"
        },
        {
          "relation": "informs",
          "target": "RFC-014"
        }
      ]
    }
  },
  "EVID-176": {
    "status": "draft",
    "depth": "standard",
    "slug": "evid-adr-027-rfc-014-independent-integrated-source-verification",
    "title": "ADR-027 RFC-014 independent integrated source verification",
    "body_bytes": 145129,
    "body_sha256": "3d0bb5864be86caa3d66d5877cec0809c42a1f9903e00060e1eacb368c604a2a",
    "links": {
      "inbound": [],
      "outbound": [
        {
          "relation": "informs",
          "target": "ADR-027"
        },
        {
          "relation": "informs",
          "target": "RFC-014"
        }
      ]
    }
  },
  "EVID-177": {
    "status": "draft",
    "depth": "standard",
    "slug": "evid-adr-027-rfc-014-r2-independent-corrected-core-policy-source-verification",
    "title": "ADR-027 RFC-014 r2 independent corrected core policy source verification",
    "body_bytes": 158607,
    "body_sha256": "639436a15987ea1bccb1016cfb92b4c7d718a894270cacd8449df394b53caea3",
    "links": {
      "inbound": [],
      "outbound": [
        {
          "relation": "informs",
          "target": "ADR-027"
        },
        {
          "relation": "informs",
          "target": "RFC-014"
        }
      ]
    }
  },
  "EVID-172": {
    "status": "draft",
    "depth": "tactical",
    "slug": "evid-prd-087-spec-007-r1-independent-shape-fitness-audit",
    "title": "PRD-087 SPEC-007 r1 independent Shape fitness audit",
    "body_bytes": 22610,
    "body_sha256": "166459e3b24ace7fab5effaa917094914b38cdb09bba1e7c8c39e52ec74171d2",
    "links": {
      "inbound": [],
      "outbound": [
        {
          "relation": "informs",
          "target": "PRD-087"
        },
        {
          "relation": "informs",
          "target": "SPEC-007"
        }
      ]
    }
  },
  "EVID-173": {
    "status": "draft",
    "depth": "standard",
    "slug": "evid-prd-087-spec-007-note-053-independent-finite-local-shape-re-review",
    "title": "PRD-087 SPEC-007 NOTE-053 independent finite local Shape re-review",
    "body_bytes": 33806,
    "body_sha256": "9bde8017f7bf2a4ea81e137534085e712b137f634005c3a796d136974d1adb6e",
    "links": {
      "inbound": [],
      "outbound": [
        {
          "relation": "informs",
          "target": "PRD-087"
        },
        {
          "relation": "informs",
          "target": "SPEC-007"
        },
        {
          "relation": "informs",
          "target": "NOTE-053"
        }
      ]
    }
  },
  "NOTE-053": {
    "status": "draft",
    "depth": "standard",
    "slug": "note-prd-087-spec-007-bounded-authored-adi-and-factual-shape-handoff-resolution",
    "title": "PRD-087 SPEC-007 bounded authored ADI and factual Shape handoff resolution",
    "body_bytes": 40187,
    "body_sha256": "961b084cab0b0ca1ee60c49a8f08668490301ef3e4a01128aea822e601ab2783",
    "links": {
      "inbound": [
        {
          "relation": "informs",
          "source": "EVID-173"
        }
      ],
      "outbound": [
        {
          "relation": "informs",
          "target": "PRD-087"
        },
        {
          "relation": "informs",
          "target": "SPEC-007"
        }
      ]
    }
  },
  "PRD-087": {
    "status": "draft",
    "depth": "deep",
    "slug": "prd-website-onboarding-task-stack-selector-and-revision-bound-public-guide",
    "title": "Website onboarding — task/stack selector and revision-bound public guide",
    "body_bytes": 34406,
    "body_sha256": "803d422426a6826066c17ed165f28564e7979b5af429cad62d4f2d91d149a83a",
    "links": {
      "inbound": [
        {
          "relation": "based_on",
          "source": "SPEC-007"
        },
        {
          "relation": "informs",
          "source": "EVID-172"
        },
        {
          "relation": "informs",
          "source": "NOTE-053"
        },
        {
          "relation": "informs",
          "source": "EVID-173"
        }
      ],
      "outbound": [
        {
          "relation": "refines",
          "target": "PRD-024"
        },
        {
          "relation": "refines",
          "target": "PRD-047"
        }
      ]
    }
  },
  "SPEC-007": {
    "status": "draft",
    "depth": "deep",
    "slug": "spec-website-onboarding-bounded-selection-and-public-guidance-behavior",
    "title": "Website onboarding — bounded selection and public guidance behavior",
    "body_bytes": 39292,
    "body_sha256": "5adc34c6f86ca693580b86c8208b304ff79658b7978b781efb3788850ff60678",
    "links": {
      "inbound": [
        {
          "relation": "informs",
          "source": "EVID-172"
        },
        {
          "relation": "informs",
          "source": "NOTE-053"
        },
        {
          "relation": "informs",
          "source": "EVID-173"
        }
      ],
      "outbound": [
        {
          "relation": "based_on",
          "target": "PRD-087"
        }
      ]
    }
  }
}
```

## Appendix B — Actual Source Manifest and Protected Raw Pins

```text
7063a2299bafb4289ce064c67798f50b07e9a57072dd119dac05441495328791  CLAUDE.md
7b7753212d903e35defa21566d51446829af284d0b12b0e5c403c79102fe2cdd  docs/methodology/QUALITY-GATES.md
0b3a428ebec7c9f3ac73a2bd5b5cb84a5d41886ae42d32172a0477ffdbaa8259  docs/methodology/QUALITY-GATES.ru.md
af5cdc68127138acac3d9ef74c10fa57a2b54868f0491ea0973f18a5a1d80ebf  docs/methodology/HOW-TO-USE.md
3f497168e0ebd228286346c54d0115fc5920b3cb057402eb0951a7ed989109bf  docs/methodology/HOW-TO-USE.ru.md
ec890f9637a4b09068f5e3bb98f266d8882d4db6301c0f9d48c7cf31ab79c8b8  docs/methodology/GLOSSARY.md
db1798c37307a90e62459e5597424834eafdde62f674541e82f2f7fc55446722  docs/methodology/GLOSSARY.ru.md
a9a022b9f50eee8bbf73a2eab4875b74ee8a26456e6686763d0acd3234a446a2  docs/methodology/FORGEPLAN-GUIDE.md
5d61e16166a3ebd8245d18eee4a94dd9c26e22b2f43b34952aa32f0308d1c1f9  docs/methodology/FORGEPLAN-GUIDE.ru.md
e83f8dbc3bed20abe683c89d59bcf2803c00dd5a74c3028707441b8fd4e02f4f  docs/methodology/METHODOLOGY-COURSE.md
11b8c2a2a8edd3c2139ba35d17e32ac78f81db4e9f662bef4f052f4ab78d0494  docs/methodology/METHODOLOGY-COURSE.ru.md
25c7a066debeb49b0945e0256b4df13062c08525912364d0b8a1a4c13564b92a  docs/methodology/USAGE-BY-ROLE.md
82624ca91e27556843d8f23853b7fee19f113f13b680f7b4f5d70697fcbbfb18  docs/methodology/USAGE-BY-ROLE.ru.md
816460a572e1a57d32a1686bfec8494d69547ba3fc9395bc44daf0fb0105082d  docs/schemas/PRD-SCHEMA.md
0beb0fd29ba2f0b8235f648f9a2b1428db28620f0e9b3e61b5d3271b5819875c  docs/schemas/PRD-SCHEMA.ru.md
75418bd03761723dcc596d25dd922015f759aba7eff7a5d6cc0dd66cdae59363  templates/prd/_TEMPLATE.md
05735e0bf61ecafab256cb44c0157d5ea4e24faf10917a83089661b121f6ed1c  CHANGELOG.md
```

```json
{
  ".forgeplan/evidence/EVID-172-prd-087-spec-007-r1-independent-shape-fitness-audit.md": "a0959046856a371d457bc497f6251a827ece9f28b2956663aa6b15ae433c7e7d",
  ".forgeplan/evidence/EVID-173-prd-087-spec-007-note-053-independent-finite-local-shape-re-review.md": "5336eddc3526238893d78ebbaa8ef3392b9f56ac2dcd0634ca5d2de77317885e",
  ".forgeplan/notes/NOTE-053-prd-087-spec-007-bounded-authored-adi-and-factual-shape-handoff-resolution.md": "52788c120dbfdd8fb19c5b51f31005892f8cd53730472da16de56be27014d8ac",
  ".forgeplan/prds/PRD-087-website-onboarding-task-stack-selector-and-revision-bound-public-guide.md": "9f83bd7974b82e07b1b2cb2e90787e1ca742e2afa145c2c8e0f6f52294286e98",
  ".forgeplan/specs/SPEC-007-website-onboarding-bounded-selection-and-public-guidance-behavior.md": "4abb2402c0e24e84383a5dd5cb0b29853502e6d5f5f66d0236c1949405d64833"
}
```

## Appendix C — Actual Before-displacement Decision Score Results

```json
{
  "ADR-027": {
    "_next_action": "Add more evidence or improve congruence level (CL3 = same context)",
    "evidence": [
      {
        "congruence_level": 3,
        "excluded": false,
        "expired": false,
        "id": "EVID-174",
        "score": 1.0,
        "status": "draft",
        "verdict": "Supports"
      },
      {
        "congruence_level": 3,
        "excluded": false,
        "expired": false,
        "id": "EVID-175",
        "score": 1.0,
        "status": "draft",
        "verdict": "Supports"
      },
      {
        "congruence_level": 3,
        "excluded": false,
        "expired": false,
        "id": "EVID-176",
        "score": 0.5,
        "status": "draft",
        "verdict": "Weakens"
      },
      {
        "congruence_level": 3,
        "excluded": false,
        "expired": false,
        "id": "EVID-177",
        "score": 1.0,
        "status": "draft",
        "verdict": "Supports"
      }
    ],
    "factors": [
      "CL penalty applied for ADR-025 (relation: based_on)",
      "CL penalty applied for ADR-003 (relation: based_on)",
      "Skipped RFC-014 (status: draft)"
    ],
    "fgr": {
      "formality": 0.8461538461538461,
      "grade": "B",
      "granularity": 0.4,
      "overall": 0.6036810736797685,
      "reliability": 0.6499999999999999
    },
    "id": "ADR-027",
    "r_eff": 0.3,
    "r_eff_ci": {
      "evidence_count": 4,
      "high": 0.65,
      "insufficient": false,
      "low": 0.35,
      "point": 0.5,
      "stale_count": 0,
      "width": 0.30000000000000004
    },
    "title": "Finite evidence-backed independent review without a defect quota",
    "weakest_link": "ADR-025"
  },
  "RFC-014": {
    "_next_action": "Add more evidence or improve congruence level (CL3 = same context)",
    "evidence": [
      {
        "congruence_level": 3,
        "excluded": false,
        "expired": false,
        "id": "EVID-174",
        "score": 1.0,
        "status": "draft",
        "verdict": "Supports"
      },
      {
        "congruence_level": 3,
        "excluded": false,
        "expired": false,
        "id": "EVID-175",
        "score": 1.0,
        "status": "draft",
        "verdict": "Supports"
      },
      {
        "congruence_level": 3,
        "excluded": false,
        "expired": false,
        "id": "EVID-176",
        "score": 0.5,
        "status": "draft",
        "verdict": "Weakens"
      },
      {
        "congruence_level": 3,
        "excluded": false,
        "expired": false,
        "id": "EVID-177",
        "score": 1.0,
        "status": "draft",
        "verdict": "Supports"
      }
    ],
    "factors": [
      "CL penalty applied for ADR-025 (relation: based_on)",
      "CL penalty applied for ADR-003 (relation: based_on)"
    ],
    "fgr": {
      "formality": 0.8461538461538461,
      "grade": "A",
      "granularity": 1.0,
      "overall": 0.8193212706006457,
      "reliability": 0.6499999999999999
    },
    "id": "RFC-014",
    "r_eff": 0.3,
    "r_eff_ci": {
      "evidence_count": 4,
      "high": 0.65,
      "insufficient": false,
      "low": 0.35,
      "point": 0.5,
      "stale_count": 0,
      "width": 0.30000000000000004
    },
    "title": "Core review policy bounded documentation cutover",
    "weakest_link": "ADR-025"
  }
}
```

## Persistence Checkpoint — Actually Observed After Both Informs

Обе связи EVID178→ADR027/RFC014 созданы canonical MCP с auto_activate_source_if_complete=false. Actual CLI get EVID178 exit0: draft/standard, canonical slug `evid-adr-027-rfc-014-final-independent-core-policy-guardian-and-staged`, inbound=[], outbound ровно эти2 informs. Всё подготовленное guardian body (60320 UTF-8 bytes) найдено целиком без изменения; API добавил169 bytes внутреннего template metadata. Checkpoint body ДО этого приложения:60489 bytes, SHA256 `daeb7ef24ca1a6d25ea9e08745c1aded3574acae4a73f42dd3e18503358adb23`; это не самоссылочный final hash.

Actual `forgeplan validate EVID-178 --json` exit0: passed=true, errors0,warnings0,findings=[]. Повторный score ПОСЛЕ обеих links: ADR0270.30/weakest ADR025, RFC0140.30/weakest ADR025, EVID1780.80/weakest null. Значит новая supports запись сама не сняла старый current weakens и не удовлетворила decision0.50. Hints activation не исполнены; текущая policy activation по-прежнему не разрешена без Stage1–3.

Самостоятельно повторно получены все17 pre-existing canonical records из AppendixA: прежние body/status/depth/slug/title/outbound; единственные ожидаемые inbound добавления EVID178 у ADR027/RFC014. Ни одно историческое Evidence не переписано. Source17/17 побайтно прежние; protected raw5/5 прежние. Из1790 baseline entries1788 byte-identical; только два NONprotected target projections получили ровно1 appended LF, что доказано SHA(current без одного final LF)==before, canonical body прежний. Единственный добавленный nonignored path — `.forgeplan/evidence/EVID-178-adr-027-rfc-014-final-independent-core-policy-guardian-and-staged-lifecycle-gate.md`.

```json
[
  {
    "path": ".forgeplan/adrs/ADR-027-finite-evidence-backed-independent-review-without-a-defect-quota.md",
    "before": "d6dd9741f46cf19560f4c0e490fca5b2afcdf44158b42236d399195cf952515f",
    "after": "b1d592de40926327bb5cffdf1cf71793b8fc8352eeb3d3d0c3380dc4b0fd9517",
    "only_one_appended_LF": true
  },
  {
    "path": ".forgeplan/rfcs/RFC-014-core-review-policy-bounded-documentation-cutover.md",
    "before": "0983232b5159c0d19ea168574299e531546dd61be13c2eabbf62cda3fd4f06d1",
    "after": "a37457f4cbab3ac33b58f9867aba902c1df4665ff6cb0a08db987c82d6a8107c",
    "only_one_appended_LF": true
  }
]
```

Framing side effect canonical sequence сообщён tool QA и принят. Его внутренний trigger не изолирован, raw repair/reindex не выполнялись; это не нормативное изменение и не разрешение менять protected историю. Следующий final get/validate/preservation compare проверяет и это приложение; final body SHA передаётся отдельно.

Итог не изменён: **PASS current r2 source + точное условное staged lifecycle разрешение**, не PASS нынешнего decision score и не уже исполненная активация. Исторический EVID176 остаётся draft/weakens/r1 BLOCKER до разрешённых будущих canonical действий оркестратора. Website/producer/runtime/shipped-new-template не приняты.


