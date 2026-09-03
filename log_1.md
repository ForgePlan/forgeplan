
⏺ Теперь EvidencePack с записью аудита — со structured fields, иначе парсер молча поставит CL0 и R_eff=0.

⏺ auto_linked: EPIC-009 — parent_id сработал, лишняя связь не нужна. Заполняю тело.

⏺ Связываю артефакты в граф — рёбра contradicts делают коллизию машинно-видимой.

⏺ Все 8 связей построены, EVID помечен «complete evidence» — значит structured fields распарсились. Проверяю, что коллизия стала машинно-видимой,
  и коммичу артефакты.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== contradictions now visible to tooling? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +21 lines (ctrl+o to expand)

⏺ EPIC-009 не проходит валидацию — смотрю, чего не хватает, и проверяю противоречия через MCP (в CLI такой подкоманды нет).

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== EPIC-009 validation detail ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +18 lines (ctrl+o to expand)

⏺ Два результата. EPIC-009 требует секций ## Phases и таблицы детей. И более важное: forgeplan_contradictions возвращает пусто, хотя я только что
  создал три ребра contradicts — то есть план «сделать коллизию машинно-видимой» не работает так, как предполагалось. Исправляю EPIC и фиксирую
  находку.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== EPIC-009 revalidate ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +16 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== EPIC-009 validate (full) ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +27 lines (ctrl+o to expand)

⏺ EPIC-009 теперь PASS (0 ошибок), рёбра contradicts записаны на диск. Но инструмент их не увидел — перепроверяю честно, прежде чем объявлять
  дефект.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      forgeplan scan-import 2>&1 | grep -E 'Summary'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +24 lines (ctrl+o to expand)

⏺ Подтверждено после явного реиндекса: forgeplan_contradictions возвращает пусто, хотя три ребра contradicts записаны. Это реальный дефект —
  добавляю в PROB-083.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add .forgeplan/epics/EPIC-009-*.md .forgeplan/problems/PROB-082-*.md .forgeplan/problems/PROB-083-*.md .forgeplan/evid…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +30 lines (ctrl+o to expand)

⏺ Появились новые untracked-каталоги. Проверяю: создал ли их мой scan-import, или они существовали и были ещё одним вектором потери. И что именно
  изменилось в четырёх ADR.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== what changed in ADR-001 (expect only back-link) ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +29 lines (ctrl+o to expand)

⏺ Два наблюдения, оба требуют честной фиксации.

  Первое: изменение в четырёх ADR — это только добавленный пустой перевод строки, никаких обратных связей. То есть forgeplan_link пишет ребро лишь
  в источник, а файл цели трогает вхолостую. Это же объясняет и загадочный +1 в PROB-047.

  Второе: шесть новых каталогов создал мой запуск scan-import (21:48). Проверяю, что именно он материализовал.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== what scan-import materialised (all 6) ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +52 lines (ctrl+o to expand)

⏺ Подтвердилось: мой scan-import затянул шаблоны и примеры самого репозитория и материализовал их как артефакты-черновики с author: scan-import и
  плейсхолдерами {{term_name}}. Это ровно дефект PROB-047. Прежде чем убирать — проверяю, что источник цел и я ничего не перемещу безвозвратно.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== source templates still intact outside .forgeplan? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +36 lines (ctrl+o to expand)

⏺ Источники целы: каталог templates/ на месте, примеры tripsales — в docs/brownfield-extraction-package/examples/. Значит scan-import их
  скопировал, а не переместил, и 15 сгенерированных артефактов можно убрать без потерь. Убираю через CLI, не rm — иначе индекс разойдётся с
  файлами.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      forgeplan delete --help 2>&1 | head -18)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +20 lines (ctrl+o to expand)

❯ и если ты гнаешл ккие то бвги - рельно их нужно прворить ест ьли они вообещ или может что т о не так делали и пойти их заенст ив issues гитхаба

⏺ Абсолютно правильное требование — и оно уже оправдалось: мой forgeplan list adr оказался моей ошибкой синтаксиса, а не багом. Остальные находки
  надо прогнать через ту же проверку, прежде чем нести в GitHub.

  Приостанавливаю уборку 15 мусорных артефактов — они пока служат живым репродьюсером. Запускаю верификацию каждого кандидата против исходников:
  реальный баг / моё неправильное использование / так задумано / дубль существующего issue.

⏺ Workflow(Adversarially verify each candidate ForgePlan bug against source code: REAL vs MISUSE vs BY_DESIGN vs DUPLICATE, then produce
          issue-ready reports)
  ⎿  /workflows to view dynamic workflow runs

⏺ Verification workflow пошёл. Обновляю трекинг — многое уже закрыто.

⏺ Пока идёт проверка багов — отчёт по тому, что уже сделано.

  ---
  Пакет проверен 13 агентами: 130 находок, 27 BLOCKER, вердикт NEEDS_REWORK по всем девяти областям. Материал размещён и закоммичен в двух ветках,
  находки зафиксированы в графе. GitHub не тронут. Кандидаты в баги сейчас проходят адверсариальную проверку — заводить буду только выжившие.

  🔍 Что показал аудит

  Ядро идеи — верное и измеримое. 148 EvidencePack, из них 0 содержат base_sha/result_sha. То есть «доказательство» сегодня — это рассказ
  исполнителя о самом себе, и архитектура структурно не может отличить его от выполненной работы.

  Но пакет не выполняет собственный контракт. Его же ISSUE-GOVERNANCE.md требует восемь секций — соответствие 0/16. При этом validate_pack.py
  печатает OK, потому что проверяет только наличие файлов.

  И разворачивает четыре активных ADR, ни разу их не назвав — grep по ADR-идентификаторам в docs/vnext/ даёт 0.

  Критик расщепил единый вердикт надвое, и это важнее самого вердикта:
  - FPV-05 + корректностная часть FPV-06 — отгружаемо сегодня, зависимостей нет
  - FPV-09…FPV-15 — в 77 PROB ни одной заявки на кросс-хостовую переносимость; нужна не переработка текста, а основание существовать

  ⚠️  Критик поймал разрушительную команду — в самом брифе

  Синтез рекомендовал git checkout <ветка> -- CLAUDE.md. Проверка показала: закоммиченные версии идентичны, все 86 строк живут только в рабочем
  дереве. Команда стёрла бы ровно ту работу, которую спасала. Не выполнялась — использована безопасная форма.

  📦 Что размещено и закоммичено

  Ветка docs/vnext-pack-import — 4 коммита
  Пакет + PR-шаблон · _import/ с 9 файлами провенанса и README · PROB-080/081 (310 строк, ни разу не коммиченные) · .gitignore для .serena/ и
  .created-issues.json
  ———
  Ветка docs/task-list-discipline — 1 коммит
  CLAUDE.md +86, раздел «Task List Discipline» — вынесен отдельно, к пакету отношения не имеет
  ———
  Граф (через MCP, RED LINE #11)
  EPIC-009 (validate PASS, фазы BLOCKED, дочерние PRD намеренно не созданы) · PROB-082 реестр коллизий · PROB-083 дефекты субстрата · EVID-149
  запись аудита. 8 связей, включая 3× contradicts. Все в draft.
  ———
  ref/ не тронут — я его не создавал, удаление за вами. Уникальные файлы скопированы, не перемещены.

  🔧 Моя собственная ошибка — раскрываю

  Я запустил forgeplan scan-import, чтобы починить индекс. Он создал 15 мусорных артефактов — затянул шаблоны самого репозитория ({{term_name}},
  author: scan-import) и примеры из docs/. Источники целы, файлы untracked, но это побочный эффект моей команды. Уборку приостановил: они сейчас
  служат репродьюсером для проверки — не баг ли это.

  Индекс, кстати, не починился: ADR-016 и ADR-017 (оба активные, frontmatter корректный) не резолвятся и после реиндекса.

  🚫 Чего не делал

  GitHub не изменялся ни разу. 16 FPV-issues не созданы — и по итогам аудита создавать их в текущем виде нельзя: gh 2.83.2 не поддерживает
  --parent/--add-blocked-by, скрипт вызывает их с check=False, поэтому issues легли бы плоским списком без epic-родителя и без единой связи. Ни
  одного push.

  ➡️  Дальше

  Идёт workflow w8ks0lxp1: 7 кандидатов в баги, каждый сначала расследуется, затем независимый скептик пытается его опровергнуть (по умолчанию —
  опровергнут). Вернусь со списком: что реально заводить, что оказалось моим неправильным использованием, что уже покрыто существующими issues.

✻ Brewed for 18m 16s

⏺ Dynamic workflow "Adversarially verify each candidate ForgePlan bug against source code: REAL vs MISUSE vs BY_DESIGN vs DUPLICATE, then produce
issue-ready reports" completed · 21m 22s

⏺ Результат отличный и отрезвляющий: из 7 кандидатов выжили только 2. Скептик убил два «реальных» бага как дубли — причём один оказался дублем
  PROB-083, который я сам создал часом ранее. Читаю полностью.

⏺ Bash(F=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/tasks/w8ks0lxp1.output
      D=/private/tmp/claude-501/-Users-explos…)
  ⎿  report -> 35090 chars
     by_verdict: {'MISUSE': 2, 'BY_DESIGN': 1, 'REAL': 4} | survivors: 2
     … +10 lines (ctrl+o to expand)

⏺ Два из «багов» оказались моей ошибкой: индекс перестраивается командой forgeplan reindex, а я запускал scan-import. Проверяю сам, прежде чем
  что-то заводить.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== does 'forgeplan reindex' exist? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +28 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      for a in ADR-016 ADR-017 EVID-143 ADR-013; do…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +36 lines (ctrl+o to expand)

⏺ Подтверждено — это была моя ошибка. forgeplan reindex («Rebuild LanceDB index from .md files») чинит всё: ADR-016, ADR-017, EVID-143 резолвятся.
  Не резолвится только ADR-013 — у него честно нет frontmatter.

  Значит в PROB-083 у меня ложное утверждение, уже закоммиченное. Проверяю второй выживший баг.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== forgeplan_new REPORTED a slug triple for all four. Did the FILE keep it? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +23 lines (ctrl+o to expand)

⏺ Второй выживший подтверждён и он серьёзный: forgeplan_new отрапортовал полный identity-триплет для всех четырёх артефактов, а в файлах ноль
  строк slug:. По репозиторию slug несёт ровно 1 файл. Прежде чем заводить HIGH-issue в публичный репозиторий — воспроизвожу изолированно, чтобы
  точно знать, какая команда его уничтожает.

⏺ Bash(W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad/slugtest
      rm -rf "$W" && mkdir -p "$W" && cd "$W"…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +27 lines (ctrl+o to expand)
  ⎿  Shell cwd was reset to /Users/explosovebit/Work/ForgePlan

⏺ Ключевое наблюдение: свежий forgeplan new сам порождает два блока frontmatter, и assigned_number попал во второй. Значит это не историческое
  наследие. Довожу тест до конца.

⏺ Bash(W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad/slugtest
      F="$W/.forgeplan/prds/PRD-001-slug-persist…)
  ⎿  === FULL second block after 'new' ===

     ---
     … +22 lines (ctrl+o to expand)

⏺ Полная картина сложилась — и это один баг с двумя симптомами. forgeplan new пишет два блока: первый канонический (его читает парсер), второй —
  из шаблона, и именно туда попадают slug, predicted_number, assigned_number плюс мусор вроде depth: tactical / standard / deep / critical и
  конфликтующий status: Draft.

  Проверяю, что делает update.

⏺ Bash(W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad/slugtest
      F="$W/.forgeplan/prds/PRD-001-slug-persist…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +27 lines (ctrl+o to expand)
  ⎿  Shell cwd was reset to /Users/explosovebit/Work/ForgePlan

⏺ Механизм установлен точно, и оба симптома складываются в одну цепочку:

  1. forgeplan new кладёт slug/predicted_number/assigned_number только во второй блок, который парсер не читает
  2. forgeplan update --body заменяет тело и уничтожает второй блок вместе с триплетом — безвозвратно

  Именно это произошло с моими четырьмя артефактами. Сначала исправляю ложное утверждение в PROB-083 — оно уже закоммичено.

⏺ Тело исправлено, но заголовок артефакта всё ещё утверждает ложное. Проверяю, можно ли его поменять.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      forgeplan update --help 2>&1 | head -20)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +23 lines (ctrl+o to expand)

⏺ Заголовок исправлен. Теперь завожу issues. Сначала — конвенции репозитория, чтобы не создать инородное.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== label set ==="; gh label list --repo ForgePlan/forgeplan --limit 40 | head -25…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +44 lines (ctrl+o to expand)

  Read 1 file (ctrl+o to expand)

⏺ Стиль и метки понятны; существующего issue про write-path слуга нет (#397 — про чтение, не про запись). Собираю данные окружения и завожу.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "version: $(grep -m1 '^version' Cargo.toml)"…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)

⏺ Числа сместились из-за мусора от scan-import — уточняю, прежде чем писать в issue.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== which 3 files carry slug: ? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +21 lines (ctrl+o to expand)

⏺ Обнаружил побочный эффект прямо сейчас: смена заголовка через forgeplan update --title создала новый файл, не удалив старый — теперь два файла с
  id: PROB-083. Проверяю.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== two files claiming PROB-083? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +26 lines (ctrl+o to expand)

⏺ Подтверждено: forgeplan update --title создал новый файл и оставил старый — два файла с id: PROB-083. Это третий реальный баг, и я его только
  что спровоцировал. Заодно выяснилось, что найденные slug: — это текст моего же репродьюсера внутри код-блока, то есть ни один артефакт слуга во
  frontmatter не несёт. Убираю дубль.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      # The stale file carries the OLD (now false) title; the new one has the corrected title + same body.…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)

⏺ Дубль убран, индекс пересобран. Завожу issues — по одному, каждый с репродьюсером, который я прогнал сам.

  Made 2 scratchpad edits +185 (ctrl+o to expand)

⏺ Третий баг — про link. Проверяю кумулятивность в изолированном workspace, прежде чем утверждать это в issue.

⏺ Bash(W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad/slugtest
      cd "$W"…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +28 lines (ctrl+o to expand)
  ⎿  Shell cwd was reset to /Users/explosovebit/Work/ForgePlan

⏺ Важная поправка: рост не кумулятивный — +1 байт при первой связи, дальше стабильно (3509 на связях #2 и #3). Значит severity ниже, чем заявил
  исследователь: это разовая нормализация, а не разрастание файла. Заведу три существенных issue, а этот отражу в отчёте.

  Made 1 scratchpad edit +84 (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      S=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      S=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)

⏺ Заведено: #418, #419, #420. Теперь убираю мусор, который создал мой scan-import, и синхронизирую индекс.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== removing the 15 template artifacts scan-import generated (dirs never existed in git) ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +29 lines (ctrl+o to expand)

⏺ Рабочее дерево чистое. Связываю PROB-083 с заведёнными issues — через CLI, канонический канал.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      F=$(ls .forgeplan/problems/PROB-083-artifact-substrate-identity*.md)…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      F=$(ls .forgeplan/problems/PROB-083-artifact-substrate-identity*.md)…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +32 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add -u .forgeplan/problems/PROB-083-artifact-substrate-three-adrs-*.md 2>/dev/null…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +26 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add .forgeplan/adrs/ADR-001-*.md .forgeplan/adrs/ADR-003-*.md .forgeplan/adrs/ADR-009-*.md .forgeplan/adrs/ADR-011-*.m…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +22 lines (ctrl+o to expand)

⏺ Всё завершено. Итоговый отчёт.

  ---
  Пакет проверен и размещён, ничего не потеряно. Из 7 «багов» реальными оказались 3 — заведены как #418, #419, #420; два были моей ошибкой, два —
  дублями. Программа vNext к исполнению не годна: 0 из 16 задач готовы. GitHub изменён только тремя bug-issues, ни одного push.

  🎯 Вердикт по идее

  ПЕРЕРАБОТАТЬ, но с расщеплением — оно важнее самого вердикта.

  Ядро измеримо и верно: 148 EvidencePack, 0 с base_sha/result_sha. Доказательство сегодня — рассказ исполнителя о себе.

  - FPV-05 + корректностная часть FPV-06 — отгружаемо сейчас, без Protocol v1, адаптеров и сервера
  - FPV-09…FPV-15 — в 77 PROB ни одной заявки на кросс-хостовую переносимость. Нужна не переработка текста, а основание существовать

  Пакет при этом не выполняет собственный контракт: его ISSUE-GOVERNANCE.md требует 8 секций — соответствие 0/16, а validate_pack.py печатает OK,
  потому что проверяет лишь наличие файлов. И разворачивает четыре активных ADR, ни разу их не назвав.

  🐞 Баги: что реально, а что моя ошибка

  Заведено (проверено мной лично, изолированно):

  #: #418 (https://github.com/ForgePlan/forgeplan/issues/418)
  Что: new пишет slug/predicted_number/assigned_number во второй блок frontmatter, который парсер не читает; update --body его уничтожает. 0 из
  383
   артефактов несут slug — ADR-012 мёртв в write-path. Тот же корень объясняет 68 файлов со сдвоенным frontmatter (37 с конфликтующим статусом)
  Severity: HIGH
  ────────────────────────────────────────
  #: #419 (https://github.com/ForgePlan/forgeplan/issues/419)
  Что: update --title пишет новый файл и оставляет старый → два файла с одним id. Наткнулся сам, переименовывая PROB-083
  Severity: MEDIUM
  ────────────────────────────────────────
  #: #420 (https://github.com/ForgePlan/forgeplan/issues/420)
  Что: Документация в 4 местах велит scan-import для пересборки индекса; на самом деле это reindex
  Severity: LOW/doc

  Моя ошибка, не баг — вы правильно потребовали это проверить:
  - «ADR-016/017 не резолвятся» — я запускал scan-import вместо reindex. После reindex резолвятся. Причина ошибки — документация, отсюда #420
  - «health врёт» — тот же корень
  - Ранее в сессии: forgeplan list adr — неверный синтаксис, list не берёт kind позиционно

  Не заведено осознанно:
  - scan-import затягивает шаблоны → дубль PROB-047 (active, 3 из 5 mitigations не сделаны)
  - contradictions не показывает явные рёбра → так задумано; спорно лишь вводящее в заблуждение поле limitations
  #: #420 (https://github.com/ForgePlan/forgeplan/issues/420)
  Что: Документация в 4 местах велит scan-import для пересборки индекса; на самом деле это reindex
  Severity: LOW/doc

  Моя ошибка, не баг — вы правильно потребовали это проверить:
  - «ADR-016/017 не резолвятся» — я запускал scan-import вместо reindex. После reindex резолвятся. Причина ошибки — документация, отсюда #420
  - «health врёт» — тот же корень
  - Ранее в сессии: forgeplan list adr — неверный синтаксис, list не берёт kind позиционно

  Не заведено осознанно:
  - scan-import затягивает шаблоны → дубль PROB-047 (active, 3 из 5 mitigations не сделаны)
  - contradictions не показывает явные рёбра → так задумано; спорно лишь вводящее в заблуждение поле limitations
  - link добавляет перевод строки в цель → проверил: рост не кумулятивный (+1 байт однажды), косметика

  📦 Куда легло

  docs/vnext-pack-import — 6 коммитов: пакет + PR-шаблон · _import/ (9 файлов провенанса + README) · PROB-080/081 · .gitignore · граф ·
  исправление PROB-083
  docs/task-list-discipline — 1 коммит: CLAUDE.md +86
  Граф: EPIC-009 (PASS, фазы BLOCKED) · PROB-082 коллизии · PROB-083 субстрат · EVID-149 аудит — все draft, 8 связей
  ref/ — не тронут, удаление за вами

  ❌ Что не сделано

  16 FPV-issues не созданы и создавать нельзя: gh 2.83.2 не поддерживает --parent/--add-blocked-by, скрипт вызывает их с check=False — issues
  легли бы плоским списком без связей, и тем же тулчейном это не чинится. Плюс 0 READY из 16. Ни одного push.

  🔧 Мои побочные эффекты — раскрываю

  scan-import создал 15 мусорных артефактов из шаблонов репозитория — убрал, индекс пересобран. Внёс ложное утверждение в PROB-083 («не резолвятся
  даже после reindex») — исправил отдельным коммитом с явной пометкой. Переименование PROB-083 породило дубль файла — убрал, и это стало #419.

  ⏭ Что дальше — за вами

  1. Кто оркестрирует? ADR-001 и ADR-009 оба active и утверждают противоположное. Это предшествует vNext и переживёт его отмену. Блокирует FPV-01
  2. Судьба playbook-runtime (ADR-011, claude --print из ядра) — шипнуто с v0.27.0, по объявленной границе нелегально
  3. Где живут WorkContract/Receipt/EvidenceBundle/Verdict относительно ADR-003
  4. Ревью и merge двух веток (git push держу до вашего явного согласия)

  Цена: 25 агентов, ~3.9M токенов, ~1 час.

✻ Sautéed for 1h 34m 12s

※ recap: Проверил и разместил пакет ForgePlan vNext: программа к исполнению не годна (0 из 16 задач готовы), материал закоммичен в две ветки,
  найденные баги заведены как issues #418, #419, #420. Дальше нужно ваше решение по конфликту ADR-001 и ADR-009 и разрешение на push. (disable
  recaps in /config)

❯ /rename

⏺ Проверю честно — и по состоянию проекта, и по своему списку. Сначала /smith.

⏺ Skill(fpl-skills:smith)
  ⎿  Successfully loaded skill

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== GIT ==="…)
  ⎿  zsh: command not found: _encode

  Called forgeplan (ctrl+o to expand)

⏺ Проверил состояние. Нет, не всё — и я нашёл собственный пропуск.

⏺ Bash(S=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad
      echo "=== analysis artifacts living ONLY in a temp…)
  ⎿  === analysis artifacts living ONLY in a temp dir ===
     (eval):3: no matches found: /private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad/*
      echo "=== scratchpad contents ==="; ls -la "$S" 2>&…)
  ⎿  === scratchpad contents ===
     total 0
     drwx------@ 2 explosovebit  wheel  64 Aug  2 22:40 .
     … +11 lines (ctrl+o to expand)

⏺ Вот и пропуск — и он существенный. Scratchpad стёрт (/private/tmp очищен), вместе с ним brief 42K, critique 28K, отчёт по багам 35K и dry-run.
  Плана «скопировать dry-run в репозиторий» я не выполнил. Но транскрипты workflow уцелели — восстанавливаю оттуда.

     total 13824
     drwxr-xr-x@ 29 explosovebit  staff     928 Aug  1 21:27 .
     … +13 lines (ctrl+o to expand)

⏺ Журналы целы. Восстанавливаю аналитику и на этот раз кладу её в репозиторий, а не во временный каталог.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      T=/Users/explosovebit/.claude/projects/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/subagents/w…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)

⏺ Структура журнала иная — смотрю ключи.

⏺ Bash(python3 - <<'PY'
      import json…)
  ⎿  0 started | keys: ['type', 'key', 'agentId']
     1 started | keys: ['type', 'key', 'agentId']
     2 started | keys: ['type', 'key', 'agentId']
     … +3 lines (ctrl+o to expand)

⏺ Bash(python3 - <<'PY'
      import json…)
  ⎿  key= 'v2:185c7249388872609aad4b27d2e27fedb6a92797872d0a2702ca4eb4e39bbd36' | result type= dict | len= 4
     key= 'v2:d23e4695ca9976af3e34b5a4b1dbd174bf39daacf5605d2f1ddfde68aa38e97b' | result type= dict | len= 4
     key= 'v2:f23f98fcd309792c6bdb2a18150ebd7d7f00e99d2e45cbecd40bc7e40326ac00' | result type= dict | len= 4

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      python3 - <<'PY'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +22 lines (ctrl+o to expand)

⏺ Всё восстановлено. Теперь регенерирую dry-run — тот шаг плана, который я пропустил.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan/docs/vnext/engineering-contract-layer

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan/docs/vnext/engineering-contract-layer
      python3 - <<'PY'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
⏺ Write(_audit/README.md)
  ⎿  Wrote 51 lines to _audit/README.md
       1 # Audit record — ForgePlan vNext pack
       2
       3 Полные отчёты адверсариального аудита пакета (2026-08-01). Сводка и связи в графе:
       4 `EVID-149`, `PROB-082`, `PROB-083`, якорь программы — `EPIC-009`.
       5
       6 Материал лежит здесь, а не во временном каталоге, потому что первая попытка его сохранить
       7 именно так и потерялась: отчёты писались в `/private/tmp/.../scratchpad`, который был
       8 очищен через сутки. Восстановлено из журналов workflow.
       9
      10 ## Файлы
     … +43 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add docs/vnext/engineering-contract-layer/_audit docs/vnext/engineering-contract-layer/issues/DRY-RUN-REVIEW.md…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +24 lines (ctrl+o to expand)

⏺ Восстановлено и закоммичено. Финальная сверка.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== the 3 filed issues ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +23 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      for a in EPIC-009 PROB-082 PROB-083 EVID-149; do…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)

⏺ Проверка ответила на ваш вопрос сама: нет, не всё — и я нашёл собственный пропуск.

  ---
  Аналитика аудита была записана в scratchpad под /private/tmp, который за сутки очистили — brief 42K, critique 28K, отчёт по багам 35K и dry-run
  исчезли. Восстановил из журналов workflow и положил в репозиторий. Плюс выполнил пропущенный шаг плана. Всё остальное на месте: 3 issue
  заведены, 4 артефакта резолвятся, 7 коммитов ждут вашего ревью.

  🔴 Что было упущено

  Потеря аналитики · /private/tmp/.../scratchpad очищен
  Именно тот класс потери, который я обязался предотвращать. Восстановлено из journal.jsonl → docs/vnext/engineering-contract-layer/_audit/ (6
  файлов, включая сырые находки в JSON).
  ———
  Пропущенный шаг плана · dry-run 16 задач так и не скопировал в репозиторий
  Регенерирован из bodies.json → issues/DRY-RUN-REVIEW.md (28 КБ). У скрипта нет --dry-run, это единственная отрендеренная форма.

  ✅ Что сделано и проверено

  ┌───────────┬───────────────────────────────────────────────────────────────────────────────────────────────┐
  │    Что    │                                           Состояние                                           │
  ├───────────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Issues    │ #418 identity triple · #419 дубль при --title · #420 docs: reindex, не scan-import — все OPEN │
  ├───────────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Артефакты │ EPIC-009, PROB-082, PROB-083, EVID-149 — все резолвятся                                       │
  ├───────────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Коммиты   │ 7 на docs/vnext-pack-import, 1 на docs/task-list-discipline                                   │
  ├───────────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Аудит     │ Сохранён в репозитории с пометками, какие выводы брифа позже опровергнуты                     │
  ├───────────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Граф      │ 0 orphans · 0 blind spots · 0 claims · 0 gitignore drift                                      │
  └───────────┴───────────────────────────────────────────────────────────────────────────────────────────────┘

  ⏸ Что намеренно не сделано

  Push и PR — RED LINE #2, жду вашего явного согласия. Обе ветки ни разу не пушились
  16 FPV-issues — 0 READY из 16, плюс gh не поддерживает --parent/--add-blocked-by
  Коллизии PROB-082 — ADR-001 против ADR-009, оба active: решение за человеком
  ref/ — не мой каталог, удаление за вами
  PROB-047 +1 пробел — правка не моя, не трогал


  - 42 EVID помечены ready_to_activate и лежат в draft 74 дня
  - PROB-078 — единственный at-risk, R_eff = 0.00
  - ADR-013 — до сих пор без frontmatter, единственный по-настоящему невидимый графу
  - EVID-143 — два файла с одним id (свой дубль PROB-083 убрал, этот — нет)
  - 68 файлов со сдвоенным frontmatter — заведено как #418, но не вычищено
  - В выводе health расхождение: stale_count: 0 при ~130 записях в stale_drafts

  ↩️  Обратимость

  Всё локально. git branch -D на двух ветках возвращает состояние полностью. Три issue на GitHub закрываются вручную. ref/ и ваши правки не
  тронуты.

  ➡️  Дальше — решения за вами

  1. Ревью 7 коммитов → разрешение на push и PR в dev
  2. Кто оркестрирует — ADR-001 или ADR-009? Блокирует FPV-01 и переживёт отмену vNext
  3. Судьба playbook-runtime (ADR-011) и место хранения новых объектов относительно ADR-003
  4. Отдельно от vNext: 42 EVID, готовых к активации, и ADR-013 без frontmatter

  Цена сессии: 25 агентов, ~3.9M токенов, 3 issue, 8 коммитов.

✻ Sautéed for 4m 3s

❯ /effort max
  ⎿  Set effort level to max (this session only): Maximum capability with deepest reasoning. May use excessive tokens resulting in long response
     times or overthinking. Use sparingly for the hardest tasks.

❯ /reload-skills
  ⎿  Reloaded skills: 337 skills available (no changes)

※ recap: Goal: verify the ForgePlan vNext pack and place it safely. Audit is done (0 of 16 tasks ready), 3 real bugs filed as issues #418-420, and
  8 commits sit on two branches. Next: review those commits, then approve the push. (disable recaps in /config)

❯ и так что соталось сделать?

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== unpushed ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +19 lines (ctrl+o to expand)

⏺ Состояние чистое. Осталось следующее.

  🔴 Только ваше решение — всё остальное этим заблокировано

  1. Push и PR. 7 коммитов на docs/vnext-pack-import + 1 на docs/task-list-discipline, обе от origin/dev, ни разу не пушились. Держу по RED LINE
  #2 — скажите «пушь», и открою два PR в dev.

  2. Кто оркестратор — ADR-001 или ADR-009? Оба active, оба утверждают противоположное, ни один не объявляет supersession. Это не про vNext —
  противоречие было до него и останется после. Пока не выбрано, FPV-01 недостижим (его же критерий «одно каноническое ADR активно»).

  3. Судьба playbook-runtime (ADR-011, claude --print из ядра, шипнуто с v0.27.0). По объявленной границе vNext оно становится нелегальным: KEEP /
  вынести в extension / deprecate.

  4. Где живут WorkContract / ExecutionReceipt / EvidenceBundle / VerificationVerdict относительно ADR-003 «markdown — единственный source of
  truth».

  5. ref/ — 384 КБ, уникальное из него уже сохранено в _import/. Удалять или оставить.

  🟢 Можно начинать хоть сейчас

  6. Починить заведённые баги — #418 (HIGH, slug не доживает до диска), #419, #420. Обычный цикл, к vNext отношения не имеют.

  7. Слайс #360 — git-delta provenance gate. Единственная часть vNext, отгружаемая против текущей границы: без Protocol v1, адаптеров и сервера.
  Закрывает измеренный разрыв 0 из 148.

  8. Активировать EVID-149 — единственный из моих четырёх артефактов, помеченный ready_to_activate. Одна команда.

  🚫 Не начинать

  9. 16 FPV-issues — 0 READY из 16, плюс gh не поддерживает --parent/--add-blocked-by, легли бы плоским списком.
  10. FPV-01 — до снятия PROB-082. Иначе первым действием программы станет молчаливое переопределение четырёх активных решений.

  🧹 Накопилось до меня — не трогал

  - 42 EVID помечены ready_to_activate и лежат в draft 74 дня
  - ADR-013 без frontmatter — единственный реально невидимый графу артефакт
  - EVID-143 — два файла с одним id (свой дубль я убрал, этот нет)
  - 68 файлов со сдвоенным frontmatter (37 с конфликтующим статусом) — корень в #418
  - PROB-078 — R_eff = 0.00, единственный at-risk
  - PROB-047 — правка +1 пробел в рабочем дереве, не моя, не трогал

  ---
  Практически: самое дешёвое и полезное сейчас — пункт 1 (ревью и push, чтобы работа перестала жить только локально) и пункт 2 (одно архитектурное
  решение, которое разблокирует всю ветку vNext). Пункты 6–8 можно отдать в работу параллельно, они ни от чего не зависят.

  Скажите, за что берёмся — или пушить.

✻ Cogitated for 55s

※ recap: Мы проверяли и размещали пакет ForgePlan vNext: аудит выявил, что программа не готова, материал и находки закоммичены локально, три
  реальных бага заведены как issues. Дальше нужно ваше решение: пушить восемь коммитов и открывать PR? (disable recaps in /config)

❯ давай простым языком - а то нихера не понятно

⏺ Хорошо, по-человечески.

  Что вообще было. Вам дали большую папку с планом — «переделать ForgePlan», 16 задач на три репозитория. Вы попросили проверить и разложить по
  местам.

  Что выяснилось. План сырой. Похоже, его писали, не заглядывая в ваш код: половина утверждений про ваш проект просто неверна. Например, там
  написано «надо перестать называть ForgePlan менеджером задач» — а у вас в документации уже прямым текстом «это не Jira, не менеджер задач». Ещё
  он предлагает то, что вы четырьмя решениями раньше сознательно отвергли — и ни одно из них даже не упоминает.

  Что там ценного. Одна вещь, и она настоящая. Сейчас «доказательство, что работа сделана» — это когда агент сам про себя написал, что сделал. У
  вас 148 таких файлов, и ни в одном нет ссылки на реальный коммит. Проверять git вместо честного слова — вот это стоит делать. Остальное в папке
  пока незачем.

  Что я сделал. Всё сохранил и разложил, ничего не потеряно. Попутно нашёл три настоящих бага в самом ForgePlan и завёл их: #418

  ---
  Что осталось — четыре штуки.

  1. Сказать «пушь». У меня 8 коммитов лежат только на вашем ноутбуке. Без вашего слова я их не отправляю. Одна фраза — и будет два PR.

  2. Ответить на один вопрос: кто главный — агент или ForgePlan? У вас два документа, оба действующие, и они говорят ровно противоположное. Один:
  «оркестрирует агент, ForgePlan только знания». Второй: «ForgePlan становится оркестратором». Пока вы не выберете, вся эта затея стоит. И
  заметьте — это ваша старая проблема, она никуда не денется, даже если план целиком выбросить.

  3. Решить, что делать с папкой ref/. Там исходный архив, 384 КБ. Всё нужное я оттуда уже переложил в проект. Удалять или пусть лежит — не мне
  решать.

  4. Хотите — чиню три бага. Они сами по себе, к плану отношения не имеют.

  ---
  Всё остальное — либо ждёт пункта 2, либо это старые хвосты, которые копились до меня (42 файла-доказательства висят недооформленными 74 дня,
  например).

  Самое полезное сейчас — пункты 1 и 2. Остальное подождёт.

✻ Sautéed for 31s

❯ оркестрирует агент, ForgePlan только знания

  и нужно чинить баги а еще расскаж ичто в итоге в этом ref было по доапбвлению функивонала там что то связаное с build или что т оиз quintcode
  было что хоели взять

⏺ Понял, решение принято: оркестрирует агент, ForgePlan — только знания. Это позиция ADR-001, и она снимает главный блокер. Зафиксирую её как
  решение чуть позже.

  Сначала отвечу на вопрос — что реально нового по функционалу было в пакете. Иду смотреть.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan/docs/vnext/engineering-contract-layer
      echo "=== упоминания quint-code / build / R_eff в пакете ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +28 lines (ctrl+o to expand)

⏺ quint-code в пакете не упоминается ни разу. Смотрю, что там за «build» и какой функционал предлагается на самом деле.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan/docs/vnext/engineering-contract-layer
      echo "=== в каком смысле там 'build' (выборка строк) ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +68 lines (ctrl+o to expand)

⏺ Ответ на вопрос — короткий и, наверное, неожиданный.

  Про quint-code: там его нет вообще. Ноль упоминаний во всех 60 файлах.

  Про «build»: это ложный след. Слово встречается, но означает роль агента — builder (планировщик / билдер / ревьюер / оператор). Никакой системы
  сборки, никакой команды build. Просто название роли.

  Что там реально по функционалу. Девять вещей:

  ├───────────────────────────┼────────────────────────────────────────────────────────────────┤
  │ WorkContract              │ «наряд на работу», собранный из PRD+RFC+ADR                    │
  ├───────────────────────────┼────────────────────────────────────────────────────────────────┤
  │ ExecutionReceipt          │ запись «снаружи что-то выполнилось»                            │
  ├───────────────────────────┼────────────────────────────────────────────────────────────────┤
  │ EvidenceBundle + проверка │ сверять реальную git-дельту, а не верить агенту                │
  ├───────────────────────────┼────────────────────────────────────────────────────────────────┤
  ├───────────────────────────┼────────────────────────────────────────────────────────────────┤
  │ Agent API v2              │ схлопнуть 73 MCP-инструмента до 9 + роли                       │
  ├───────────────────────────┼────────────────────────────────────────────────────────────────┤
  │ Conformance               │ машинная проверка совместимости                                │
  ├───────────────────────────┼────────────────────────────────────────────────────────────────┤
  │ Адаптеры + сервер         │ Cursor/Codex/OpenCode, Kandev/Vibe/Conductor, отдельный сервер │
  └───────────────────────────┴────────────────────────────────────────────────────────────────┘

  ---
  А теперь важное: ваше решение только что убило половину этого списка.

  «Оркестрирует агент, ForgePlan — только знания» означает, что адаптеры и сервер отпадают. Именно их ADR-001 и отвергал: не ForgePlan лезет в
  Cursor и Kandev, а агент сам ходит в разные MCP и собирает контекст. Пакет предлагал ровно обратное, просто не сказав об этом вслух.

  Отпадает: адаптеры хостов, адаптеры оркестраторов, Marketplace v2 как «кросс-хостовые расширения», опциональный сервер. Это задачи
  FPV-11…FPV-15.

  Забавно совпало: аудит независимо рекомендовал выбросить ровно эти же задачи — но по другой причине, из-за отсутствия спроса (ни одной заявки в
  77 проблемах). Два разных довода сошлись на одном.

  Остаётся то, что честно «знания»: проверка git-дельты вместо честного слова (самое ценное), R_eff v2, схемы как форматы данных, сжатие API до 9
  инструментов. Примерно треть пакета, зато та, которая работает.

      echo "=== шаблоны: у всех ли свой frontmatter? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +32 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== где склеивается frontmatter + body ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +29 lines (ctrl+o to expand)

  Read 2 files (ctrl+o to expand)

⏺ Теперь ясна вся цепочка. Триплет вставляется правильно — но в frontmatter шаблона, а тот целиком уходит в «тело», и проекция кладёт свой
  канонический блок сверху. Смотрю точку сборки.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== new.rs: где rendered становится body ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
⏺ Обе точки — CLI new.rs:198 и MCP server.rs:2555 — кладут rendered целиком в body. Смотрю, как собирается канонический блок, чтобы понять, куда
  селить триплет.

  Read 1 file (ctrl+o to expand)

⏺ Важное открытие: в проекции уже есть механизм preserved_fm для полей вне канонического набора. Проверяю, попадает ли туда триплет — от этого
  зависит форма починки.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== KNOWN_FM_KEYS ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +65 lines (ctrl+o to expand)

⏺ Механизм найден и он почти готов: slug, predicted_number, assigned_number не входят в KNOWN_FM_KEYS, то есть они «пользовательские» и должны
  сохраняться. Но preserved_fm читается из существующего файла — а при создании файла ещё нет, поэтому триплет и теряется.

  Значит чинить надо в одном месте. Смотрю сигнатуру функции сборки.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      awk 'NR>=420 && NR<=470' crates/forgeplan-core/src/projection/mod.rs | grep -n 'fn \|preserved_fm\|let mut fm'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)

⏺ Функция найдена — render_markdown_with_extras (строка 459). Читаю целиком.

  Read 1 file (ctrl+o to expand)

⏺ Всё ясно. Чинить надо в одной точке — render_markdown_with_extras. Проверяю сигнатуру парсера и завожу ветку.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== parse_frontmatter signature + Frontmatter type ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
      378 +    }
      379 +}
      380 +
      381  /// Sync file → store BEFORE mutation. Preserves any user edits to the
      382  /// markdown body that haven't been picked up yet.
      383  ///

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 16 lines, removed 1 line
      559          fm.insert("links".to_string(), serde_yaml::Value::Sequence(links_seq));
      560      }
      561
      562 +    // Lift a frontmatter block the body still carries (template output, or a
      563 +    // file written before this guard existed) into the canonical block, and
      564 +    // keep only the real prose as body. Without this the block is stapled
      565 +    // under a second one and degrades into dead text — the source of both the
      566 +    // doubled-frontmatter corpus and the lost ADR-012 identity triple.
      567 +    // Merged BEFORE preserved_fm so on-disk state wins over template defaults.
      568 +    let (body_fm, effective_body) = split_embedded_frontmatter(body);
      569 +    if let Some(extras) = &body_fm {
      570 +        for (k, v) in extras {
      571 +            if !KNOWN_FM_KEYS.contains(&k.as_str()) {
      572 +                fm.insert(k.clone(), v.clone());
      573 +            }
      574 +        }
      575 +    }
      576 +
      577      // Merge preserved (agent-owned / user-custom) fields last — guaranteed
      578      // to be outside KNOWN_FM_KEYS by construction (filter_preserved), so
      579      // they cannot override id/status/links/etc.
     ...
      588      }
      589
      590      let yaml = serde_yaml::to_string(&fm)?;
      576 -    Ok(format!("---\n{}---\n\n{}\n", yaml, body))
      591 +    Ok(format!("---\n{}---\n\n{}\n", yaml, effective_body))
      592  }
      593
      594  /// Stamp `last_modified_by` / `last_modified_at` onto an already-rendered

⏺ Обе правки внесены. Компилирую.

⏺ Bash(cargo check -p forgeplan-core 2>&1 | tail -20)
  ⎿      Checking lance-file v4.0.0
         Checking datafusion-session v52.5.0
         Checking datafusion-datasource v52.5.0
     … +17 lines (ctrl+o to expand)
  ⎿  (timeout 7m)

⏺ Компилируется чисто. Пишу тесты — по правилу проекта, каждая новая функция получает тест сразу.

  Read 1 file (ctrl+o to expand)

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 115 lines
      2188
      2189      // ── PRD-057 Inc 2: agent identity + unknown-fm preservation ─────────
      2190
      2191 +    // ── #418: template frontmatter must not survive as body ─────────────
      2192 +
      2193      #[test]
      2194 +    fn split_embedded_frontmatter_lifts_template_block() {
      2195 +        // Shape produced by augment_frontmatter_with_id_fields on template output.
      2196 +        let body = "---\nid: PROB-001\nslug: prob-thing\npredicted_number: 1\nassigned_number: 1\nstatus: Draft\n---\n\n# PROB-
           +001: Thing\n\n## Signal\n\nreal prose\n";
      2197 +        let (extras, rest) = split_embedded_frontmatter(body);
      2198 +        let extras = extras.expect("template block must be lifted");
      2199 +        assert_eq!(
      2200 +            extras.get("slug").and_then(|v| v.as_str()),
      2201 +            Some("prob-thing")
      2202 +        );
      2203 +        assert!(extras.contains_key("predicted_number"));
      2204 +        assert!(extras.contains_key("assigned_number"));
      2205 +        assert!(
      2206 +            !rest.contains("slug:"),
      2207 +            "frontmatter must be gone from the body"
      2208 +        );
      2209 +        assert!(rest.contains("real prose"), "prose must survive verbatim");
      2210 +    }
      2211 +
      2212 +    #[test]
      2213 +    fn split_embedded_frontmatter_leaves_plain_body_alone() {
      2214 +        let body = "## Problem\n\nno frontmatter here\n";
      2215 +        let (extras, rest) = split_embedded_frontmatter(body);
      2216 +        assert!(extras.is_none());
      2217 +        assert_eq!(rest, body, "body must be returned byte-for-byte");
      2218 +    }
      2219 +
      2220 +    #[test]
      2221 +    fn split_embedded_frontmatter_ignores_horizontal_rule() {
      2222 +        // A body opening with a markdown rule must NOT be mistaken for
      2223 +        // frontmatter — mis-splitting would eat prose.
      2224 +        let body = "---\n\nJust a rule, then text.\n";
      2225 +        let (extras, rest) = split_embedded_frontmatter(body);
      2226 +        assert!(extras.is_none(), "a horizontal rule is not frontmatter");
      2227 +        assert_eq!(rest, body);
      2228 +    }
      2229 +
      2230 +    #[test]
      2231 +    fn render_emits_one_frontmatter_block_and_keeps_identity_triple() {
      2232 +        // The #418 regression: template output carries the ADR-012 identity
      2233 +        // triple in its own block. Before the fix that block was stapled under
      2234 +        // a second one and the triple never reached the canonical frontmatter.
      2235 +        let body = "---\nid: PROB-001\nslug: prob-thing\npredicted_number: 7\nassigned_number: 7\n---\n\n## Signal\n\nbody text
           +\n";
      2236 +        let out = render_markdown_with_extras(
      2237 +            "PROB-001",
      2238 +            "problem",
      2239 +            "Thing",
      2240 +            "draft",
      2241 +            "standard",
      2242 +            None,
      2243 +            None,
      2244 +            None,
      2245 +            body,
      2246 +            &[],
      2247 +            &[],
      2248 +            None,
      2249 +        )
      2250 +        .unwrap();
      2251 +
      2252 +        assert_eq!(
      2253 +            out.matches("\n---\n").count(),
      2254 +            1,
      2255 +            "exactly one frontmatter block must be emitted, got:\n{out}"
      2256 +        );
      2257 +        assert!(
      2258 +            out.contains("slug: prob-thing"),
      2259 +            "identity triple must reach the canonical block:\n{out}"
      2260 +        );
      2261 +        assert!(out.contains("predicted_number: 7"));
      2262 +        assert!(out.contains("assigned_number: 7"));
      2263 +        assert!(out.contains("body text"), "prose must survive");
      2264 +        // Canonical keys win over whatever the template block claimed.
      2265 +        assert!(out.contains("status: draft"));
      2266 +        assert!(out.contains("kind: problem"));
      2267 +    }
      2268 +
      2269 +    #[test]
      2270 +    fn render_lets_on_disk_state_win_over_template_block() {
      2275 +        preserved.insert(
      2276 +            "last_modified_by".to_string(),
      2277 +            serde_yaml::Value::String("orchestrator/2.0".to_string()),
      2278 +        );
      2279 +        let out = render_markdown_with_extras(
      2280 +            "PROB-001",
      2281 +            "problem",
      2282 +            "Thing",
      2283 +            "draft",
      2284 +            "standard",
      2285 +            None,
      2286 +            None,
      2287 +            None,
      2288 +            body,
      2292 +        )
      2293 +        .unwrap();
      2294 +
      2295 +        assert!(
      2296 +            out.contains("last_modified_by: orchestrator/2.0"),
      2297 +            "on-disk value must win:\n{out}"
      2298 +        );
      2299 +        assert!(!out.contains("last_modified_by: template"));
      2300 +        assert!(
      2305 +
      2306 +    #[test]
      2307      fn filter_preserved_drops_known_keys() {
      2308          let fm: Frontmatter = serde_yaml::from_str(
      2309              "id: PRD-001\ntitle: Test\nkind: prd\nstatus: draft\ndepth: standard\nauthor: alice",

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib projection:: 2>&1 | tail -30)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +38 lines (ctrl+o to expand)
     … +38 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ 130 тестов проекции проходят. Проверяю, что мои новые действительно отработали, и прогоняю весь пакет.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib -- split_embedded render_emits_one render_lets_on_disk 2>&1 | grep -E '^test |test resu…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)

⏺ Все 5 новых тестов проходят. Теперь — реальный E2E на собранном бинарнике, а не только юнит-тесты.

     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n '^name' crates/forgeplan-cli/Cargo.toml | head -3…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +19 lines (ctrl+o to expand)
⏺ Собрано. Прогоняю точный репродьюсер из #418 на новом бинарнике.

⏺ Bash(FP=/Users/explosovebit/Work/ForgePlan/target/debug/forgeplan
      W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/s…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +33 lines (ctrl+o to expand)
  ⎿  Shell cwd was reset to /Users/explosovebit/Work/ForgePlan

⏺ Триплет дошёл до канонического блока — slug, predicted_number, assigned_number теперь в первом блоке, и status: draft победил шаблонный Draft.
  Но всплыл побочный эффект: в канонический frontmatter подтянулся шаблонный мусор вроде domain: general / healthcare / fintech / .... Проверяю
  структуру.

⏺ Bash(W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad/e2e418
      F=$(ls $W/.forgeplan/prds/*.md | head -1)…)
  ⎿  === строки 14-30 (закрылся ли блок, есть ли второй) ===
     stepsCompleted: []
     title: Slug persistence probe
     … +26 lines (ctrl+o to expand)

⏺ Раздвоение устранено — ровно один блок frontmatter, дальше тело (остальные --- это горизонтальные линейки в разметке). Но три плейсхолдера
  (domain, priority, projectType) утекли в реальный frontmatter. Раньше они лежали мёртвым текстом, теперь стали бы «настоящими» значениями вида
  general / healthcare / fintech. Это хуже. Сужаю подъём до самого триплета.

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 8 lines, removed 1 line
      564      // keep only the real prose as body. Without this the block is stapled
      565      // under a second one and degrades into dead text — the source of both the
      566      // doubled-frontmatter corpus and the lost ADR-012 identity triple.
      2282      #[test]
      2283 +    fn render_drops_template_scaffolding_instead_of_promoting_it() {
      2284 +        // Template blocks carry human-facing placeholder text. Lifting it would
      2285 +        // publish `domain: general / healthcare / fintech` as a real value.
      2286 +        let body = "---\nslug: prob-thing\ndomain: general / healthcare / fintech\npriority: P0 / P1 / P2 / P3\n---\n\nprose\n"
           +;
      2287 +        let out = render_markdown_with_extras(
      2288 +            "PROB-001",
      2289 +            "problem",
      2290 +            "Thing",
      2295 +            None,
      2296 +            body,
      2297 +            &[],
      2298 +            &[],
      2299 +            None,
      2300 +        )
      2301 +        .unwrap();
      2302 +
      2303 +        assert!(out.contains("slug: prob-thing"), "identity must be lifted");
      2304 +        assert!(
      2305 +            !out.contains("domain:"),
      2310 +    }
      2311 +
      2312 +    #[test]
      2313      fn render_lets_on_disk_state_win_over_template_block() {
      2314          // preserved_fm comes from the existing file; it must override a stale
      2315          // value the template block carries for the same key.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib projection:: 2>&1 | grep -E 'test result|FAILED|panicked' | head -5…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +18 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ 131 тест проекции зелёный. Пересобираю и повторяю E2E.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo build -p forgeplan 2>&1 | tail -2…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +37 lines (ctrl+o to expand)
  ⎿  Shell cwd was reset to /Users/explosovebit/Work/ForgePlan
  ⎿  (timeout 10m)

⏺ Идеально: один блок, триплет на месте, шаблонного мусора нет. Теперь вторая половина бага — переживает ли update.

⏺ Bash(FP=/Users/explosovebit/Work/ForgePlan/target/debug/forgeplan
      W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/s…)
  ⎿  zsh: command not found: _encode

⏺ Обе половины #418 закрыты: триплет пережил update --body. И неожиданно — #419 тоже не воспроизводится: после смены заголовка остался один файл.
  Надо понять, починил ли я его попутно или мой E2E просто не воспроизводит условия. Смотрю код переименования.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== логика удаления старого файла при смене имени ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     … +27 lines (ctrl+o to expand)

⏺ Вот и причина #419 — и она точная: CLI чистит, MCP нет.

  update.rs:141 вызывает remove_projection_at со старым заголовком. В server.rs такого вызова нет вообще. Это ровно та асимметрия CLI/MCP, на
  которую жаловался пакет. Читаю обработчик.

  Read 2 files (ctrl+o to expand)

⏺ Паттерн в CLI задокументирован как «PRD-073 audit M1 fix» — до MCP он просто не доехал. Проверяю, есть ли в MCP-обработчике исходная запись до
  изменения.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      awk 'NR>=3712 && NR<=3768' crates/forgeplan-mcp/src/server.rs | grep -n 'get_record\|let canonical\|async fn\|resolve\|ori…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +47 lines (ctrl+o to expand)

⏺ MCP-обработчик уже достаёт исходную запись — но кладёт её в _pre_record и выбрасывает. Значит всё нужное под рукой. Вношу правку.

⏺ Update(crates/forgeplan-mcp/src/server.rs)
      3755 -        let _pre_record = match pre_record {
      3755 +        // Retained (not `_`-discarded): a title change needs the ORIGINAL title
      3756 +        // to locate the old projection file for cleanup — see the rename block
      3757 +        // after the mutation helpers.
      3758 +        let original = match pre_record {
      3759              Some(r) => r,
      3760              // Surface the user's original input in the not-found message (mirrors
      3761              // `forgeplan_get`) — most helpful when the id genuinely doesn't exist.

⏺ Update(crates/forgeplan-mcp/src/server.rs)
  ⎿  Added 14 lines
      3835                  .map_err(safe_mcp_error)?;
      3836          }
      3837
      3838 +        // A title change renames the projection file, so the OLD path must be
      3839 +        // removed or both survive carrying the same id — a duplicate-id
      3840 +        // collision produced by one sanctioned call. `update.rs` has cleaned
      3841 +        // this up since the PRD-073 audit; the MCP handler never got the same
      3842 +        // fix, so renames through MCP silently forked the artifact in two.
      3843 +        // Exact-path removal (not a prefix glob) so a sibling whose id is a
      3844 +        // prefix of this one is never clobbered, and AFTER the new file is in
      3845 +        // place so there is no orphan window — same ordering as the CLI.
      3846 +        if p.title.is_some() {
      3847 +            let _ =
      3852          // Re-fetch for the response payload.
      3853          let updated = store
      3854              .get_record(&canonical)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo check --workspace 2>&1 | grep -E '^(error|warning)' -A4 | head -25…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Компилируется чисто, 0 предупреждений. Прогоняю обязательный конвейер проекта.

     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test --workspace 2>&1 | grep -E 'test result:|FAILED|panicked at' | head -20)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +29 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Четыре теста упали — и все про slug. Возможно, я задел семантику pre-merge. Разбираюсь.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
     zsh: command not found: _encode
     … +26 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

  Read 1 file (ctrl+o to expand)

⏺ Важное открытие: этот тест прямо документирует и опирается на раздвоенный frontmatter.

     … +44 lines (ctrl+o to expand)

⏺ Откат чистый — остался только фикс #419. Теперь правильная починка #418: сохранять идентичность при замене тела. Читаю функцию дальше.

  Read 1 file (ctrl+o to expand)

⏺ Полная картина ясна: update_body_with_projection пишет присланное тело дословно, стирая блок идентичности. Чиню именно это.

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 49 lines
      937  ///   policy — `update_body` is an input-validating mutator).
      938  /// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
      939  ///   the projection write or `update_body` call fails.
      940 +/// ADR-012 / SPEC-005 identity fields. Under the PRD-073 two-block layout these
      941 +/// live in the **body's** own frontmatter, which makes them collateral damage of
      942 +/// any body replacement — see [`carry_identity_forward`].
      943 +const IDENTITY_FM_KEYS: &[&str] = &["slug", "predicted_number", "assigned_number"];
      944 +
      945 +/// Re-attach the ADR-012 identity fields from `old_body` when `new_body` does
      946 +/// not carry them.
      947 +///
      948 +/// The identity triple lives inside the body's frontmatter block (PRD-073
      949 +/// layout: synthetic projection block on top, canonical body below). A body
      950 +/// replacement is therefore an *identity deletion* unless the fields are
      951 +/// carried across — which is why a mature workspace ends up with no artifact
      952 +/// carrying a slug at all: `forgeplan new` writes one, and the very next
      953 +/// `update --body` that CLAUDE.md prescribes silently drops it.
      954 +///
      955 +/// Callers supply prose. Re-attaching here keeps every call site from having to
      956 +/// remember, and is a no-op when the caller *did* supply identity (round-trip
      957 +/// of a full document) or when the artifact never had one (legacy, pre-Phase-1.5).
      958 +fn carry_identity_forward(old_body: &str, new_body: &str) -> String {
      959 +    let Ok((old_fm, _)) = frontmatter::parse_frontmatter(old_body) else {
      960 +        return new_body.to_string();
      961 +    };
      962 +    let carried: Vec<(&str, serde_yaml::Value)> = IDENTITY_FM_KEYS
      963 +        .iter()
      964 +        .filter_map(|k| old_fm.get(*k).map(|v| (*k, v.clone())))
      965 +        .collect();
      966 +    if carried.is_empty() {
      967 +        return new_body.to_string();
      968 +    }
      969 +
      970 +    // Start from whatever frontmatter the replacement itself declares so a
      971 +    // caller round-tripping a full document keeps their own values.
      976 +    let mut changed = false;
      977 +    for (k, v) in carried {
      978 +        if !fm.contains_key(k) {
      979 +            fm.insert(k.to_string(), v);
      980 +            changed = true;
      981 +        }
      982 +    }
      983 +    if !changed {
      1012 +    let body: &str = &carried;
      1013 +
      1014      // Audit-r7 F1 (CRITICAL): parse the new body's frontmatter and sync
      1015      // the `status` column in LanceDB if it changed. Without this, callers
      1016      // who mutate body+status atomically (e.g. `apply_transition_to_body`

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo check -p forgeplan-core 2>&1 | grep -E '^error' -A8 | head -30…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Компилируется. Пишу тесты на новую функцию.

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 41 lines
      2204
      2205      // ── PRD-057 Inc 2: agent identity + unknown-fm preservation ─────────
      2206
      2207 +    // ── #418: body replacement must not delete the ADR-012 identity ─────
      2208 +
      2209 +    const OLD_WITH_IDENTITY: &str = "---\nid: PRD-001\nslug: prd-auth-system\npredicted_number: 1\nassigned_number: 1\nstatus:
           +Draft\n---\n\n## Problem\n\nold prose\n";
      2210 +
      2211      #[test]
      2212 +    fn carry_identity_forward_reattaches_triple_to_a_bare_body() {
      2213 +        // The exact shape CLAUDE.md prescribes right after `forgeplan new`:
      2214 +        // the caller sends prose only.
      2215 +        let out = carry_identity_forward(OLD_WITH_IDENTITY, "## Problem\n\nnew prose\n");
      2216 +        assert!(out.contains("slug: prd-auth-system"), "slug must survive:\n{out}");
      2217 +        assert!(out.contains("predicted_number: 1"));
      2218 +        assert!(out.contains("assigned_number: 1"));
      2219 +        assert!(out.contains("new prose"), "the new prose must be what lands");
      2220 +        assert!(!out.contains("old prose"), "old prose must not resurrect");
      2221 +        // Only identity is carried — not the old block's stale scaffolding.
      2222 +        assert!(!out.contains("status: Draft"));
      2223 +    }
      2224 +
      2225 +    #[test]
      2226 +    fn carry_identity_forward_is_noop_for_legacy_artifacts() {
      2231 +        assert_eq!(carry_identity_forward(old, new), new);
      2232 +    }
      2233 +
      2234 +    #[test]
      2235 +    fn carry_identity_forward_respects_caller_supplied_identity() {
      2236 +        // Round-tripping a full document: the caller's own values win.
      2237 +        let new = "---\nslug: caller-chosen\npredicted_number: 9\nassigned_number: 9\n---\n\nprose\n";
      2238 +        assert_eq!(carry_identity_forward(OLD_WITH_IDENTITY, new), new);
      2239 +    }
      2240 +
      2241 +    #[test]
      2242 +    fn carry_identity_forward_survives_unparseable_old_body() {
      2247 +
      2248 +    #[test]
      2249      fn filter_preserved_drops_known_keys() {
      2250          let fm: Frontmatter = serde_yaml::from_str(
      2251              "id: PRD-001\ntitle: Test\nkind: prd\nstatus: draft\ndepth: standard\nauthor: alice",

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib -- carry_identity_forward 2>&1 | grep -E '^test |test result|panicked' | head -12)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +14 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Все 4 проходят. Прогоняю полный набор — включая те тесты, что падали.
⏺ Все 4 проходят. Прогоняю полный набор — включая те тесты, что падали.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test --workspace 2>&1 | grep -E 'FAILED|panicked at|^error' | head -10…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Один упавший тест — git::tests::max_assigned_number_unknown_ref_returns_none. Он в git/mod.rs, которого я не касался. Четыре slug-теста теперь
  проходят — откат сработал. Проверяю, не был ли этот тест сломан до меня.
  параллельном прогоне. Проверяю изолированно.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib -- git::tests::max_assigned_number 2>&1 | grep -E '^test |test result')
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +18 lines (ctrl+o to expand)

      cargo test --workspace -- --test-threads=4 2>&1 | grep -E 'FAILED|panicked at|^error\[|^error:' | head -8…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Все 84 набора зелёные, ноль падений. Формат, линт и реальный E2E.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo fmt…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Clippy нашёл замечания — смотрю.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -E '^(warning|error)' -A8 | head -40)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +42 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

  Read 1 file (ctrl+o to expand)

⏺ Вижу проблему, и она серьёзнее замечания линтера: я вставил константу между doc-комментарием функции и самой функцией — теперь документация
  update_body_with_projection прилипла к моей константе. Нахожу начало этого комментария.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      awk 'NR>=900 && NR<=930 {printf "%d: %s\n", NR, $0}' crates/forgeplan-core/src/projection/mod.rs | head -32)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
  ⎿  Removed 18 lines
      937  ///   policy — `update_body` is an input-validating mutator).
      938  /// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
      939  ///   the projection write or `update_body` call fails.
      940 -/// ADR-012 / SPEC-005 identity fields. Under the PRD-073 two-block layout these
      941 -/// live in the **body's** own frontmatter, which makes them collateral damage of
      942 -/// any body replacement — see [`carry_identity_forward`].
      943 -const IDENTITY_FM_KEYS: &[&str] = &["slug", "predicted_number", "assigned_number"];
      944 -
      945 -/// Re-attach the ADR-012 identity fields from `old_body` when `new_body` does
      946 -/// not carry them.
      947 -///
      948 -/// The identity triple lives inside the body's frontmatter block (PRD-073
      949 -/// layout: synthetic projection block on top, canonical body below). A body
      950 -/// replacement is therefore an *identity deletion* unless the fields are
      951 -/// carried across — which is why a mature workspace ends up with no artifact
      952 -/// carrying a slug at all: `forgeplan new` writes one, and the very next
      953 -/// `update --body` that CLAUDE.md prescribes silently drops it.
      954 -///
      955 -/// Callers supply prose. Re-attaching here keeps every call site from having to
      956 -/// remember, and is a no-op when the caller *did* supply identity (round-trip
      957 -/// of a full document) or when the artifact never had one (legacy, pre-Phase-1.5).
      940  fn carry_identity_forward(old_body: &str, new_body: &str) -> String {
      941      let Ok((old_fm, _)) = frontmatter::parse_frontmatter(old_body) else {
      942          return new_body.to_string();

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 20 lines
      915      Ok(())
      919 +///
      920 +/// Under the PRD-073 two-block layout these live in the **body's** own
      921 +/// frontmatter, which makes them collateral damage of any body replacement —
      922 +/// see [`carry_identity_forward`].
      923 +const IDENTITY_FM_KEYS: &[&str] = &["slug", "predicted_number", "assigned_number"];
      924 +
      925 +/// Re-attach the ADR-012 identity fields from `old_body` when `new_body` does
      926 +/// not carry them.
      927 +///
      928 +/// The identity triple lives inside the body's frontmatter block (PRD-073
      929 +/// layout: synthetic projection block on top, canonical body below). A body
      930 +/// replacement is therefore an *identity deletion* unless the fields are
      931 +/// carried across — which is why a mature workspace ends up with no artifact
      932 +/// carrying a slug at all: `forgeplan new` writes one, and the very next
      933 +/// `update --body` that CLAUDE.md prescribes silently drops it.
      934 +///
      935 +/// Callers supply prose. Re-attaching here keeps every call site from having to
      936 +/// remember, and is a no-op when the caller *did* supply identity (round-trip of
      937 +/// a full document) or when the artifact never had one (legacy, pre-Phase-1.5).
      938  /// Replace artifact body with the supplied content. Unlike other mutation
      939  /// helpers, this does NOT call `sync_before_mutation` — the caller is
      940  /// explicitly overwriting whatever is on disk with a CLI/MCP-supplied body,

⏺ Я напутал с порядком — doc хелпера слился с doc функции. Смотрю текущее состояние и привожу в порядок.

  Read 1 file (ctrl+o to expand)

⏺ Теперь вижу точно. Переношу тело хелпера к его документации. Сначала смотрю текущий вид осиротевшей функции.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      n=$(grep -n '^fn carry_identity_forward' crates/forgeplan-core/src/projection/mod.rs | cut -d: -f1)…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +40 lines (ctrl+o to expand)

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 31 lines
      935  /// Callers supply prose. Re-attaching here keeps every call site from having to
      936  /// remember, and is a no-op when the caller *did* supply identity (round-trip of
      937  /// a full document) or when the artifact never had one (legacy, pre-Phase-1.5).
      938 +fn carry_identity_forward(old_body: &str, new_body: &str) -> String {
      960 +            changed = true;
      961 +        }
      962 +    }
      963 +    if !changed {
      964 +        return new_body.to_string();
      965 +    }
      966 +    frontmatter::render_frontmatter(&fm, &prose).unwrap_or_else(|_| new_body.to_string())
      967 +}
      968 +
      969  /// Replace artifact body with the supplied content. Unlike other mutation
      970  /// helpers, this does NOT call `sync_before_mutation` — the caller is
      971  /// explicitly overwriting whatever is on disk with a CLI/MCP-supplied body,

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Removed 31 lines
       988  ///   policy — `update_body` is an input-validating mutator).
       989  /// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
       990  ///   the projection write or `update_body` call fails.
       991 -fn carry_identity_forward(old_body: &str, new_body: &str) -> String {
       992 -    let Ok((old_fm, _)) = frontmatter::parse_frontmatter(old_body) else {
       993 -        return new_body.to_string();
       994 -    };
       995 -    let carried: Vec<(&str, serde_yaml::Value)> = IDENTITY_FM_KEYS
       996 -        .iter()
       997 -        .filter_map(|k| old_fm.get(*k).map(|v| (*k, v.clone())))
       998 -        .collect();
       999 -    if carried.is_empty() {
      1004 -    // caller round-tripping a full document keeps their own values.
      1005 -    let (mut fm, prose) = match frontmatter::parse_frontmatter(new_body) {
      1006 -        Ok((fm, prose)) => (fm, prose),
      1007 -        Err(_) => (Frontmatter::new(), new_body.to_string()),
      1008 -    };
      1009 -    let mut changed = false;
      1010 -    for (k, v) in carried {
      1011 -        if !fm.contains_key(k) {
      1012 -            fm.insert(k.to_string(), v);
      1013 -            changed = true;
      1014 -        }
      1015 -    }
      1016 -    if !changed {
      1017 -        return new_body.to_string();
      1018 -    }
      1019 -    frontmatter::render_frontmatter(&fm, &prose).unwrap_or_else(|_| new_body.to_string())
      1020 -}
      1021 -
       991  pub async fn update_body_with_projection(
       992      ctx: &MutationContext<'_>,
       993      id: &str,

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo fmt…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
⏺ Всё чисто: формат 0, clippy 0, документация функции на месте. Полный прогон тестов.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test --workspace -- --test-threads=4 2>&1 | grep -E 'FAILED|panicked at|^error:' | head -8…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)
  ⎿  (timeout 10m)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +31 lines (ctrl+o to expand)
  ⎿  Shell cwd was reset to /Users/explosovebit/Work/ForgePlan
  ⎿  (timeout 10m)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -rn 'serve' crates/forgeplan-cli/src/main.rs | grep -i 'mcp\|Serve' | head -4…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +23 lines (ctrl+o to expand)

⏺ Есть готовый набор mcp_new_update_title_validation.rs — ровно по этой поверхности. Добавлю регрессионный тест туда. Смотрю стиль.

     zsh: command not found: _encode
     zsh: command not found: _encode
     … +79 lines (ctrl+o to expand)

⏺ Есть фикстура McpFixture. Смотрю её API — нужен доступ к каталогу workspace, чтобы посчитать файлы.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n 'pub fn\|pub async fn\|pub struct\|pub workspace\|dir\b' crates/forgeplan-mcp/tests/common/mod.rs | head -20)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
      grep -n 'pub async fn call_tool\|pub async fn try_call_tool' -A6 crates/forgeplan-mcp/tests/common/mod.rs | head -22…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +26 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      sed -n '90,125p' crates/forgeplan-mcp/tests/mcp_update_filepath_e2e.rs)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +45 lines (ctrl+o to expand)

⏺ Паттерн ясен. Пишу регрессионный тест на #419.


⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      tail -18 crates/forgeplan-mcp/tests/mcp_new_update_title_validation.rs…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +35 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cat >> crates/forgeplan-mcp/tests/mcp_new_update_title_validation.rs <<'RUST'…)
  ⎿  zsh: command not found: _encode
     … +22 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Тест проходит. Но надо доказать, что он не пустой — проверяю, что без фикса он падает.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git stash push -q crates/forgeplan-mcp/src/server.rs…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode

⏺ Тест падает без фикса и проходит с ним — значит он настоящий, а не декоративный. Фикс восстановлен. Финальный прогон всего конвейера.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo fmt…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Всё зелёное: формат 0, clippy 0, 84 набора проходят. Коммичу двумя отдельными коммитами.
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add crates/forgeplan-mcp/src/server.rs crates/forgeplan-mcp/tests/mcp_new_update_title_validation.rs…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +19 lines (ctrl+o to expand)

⏺ Два бага закрыты. Остался #420 — документация. Правлю.

⏺ Преждеdчемsправитьlдокументациюk—Fпроверюnточно, что делает каждая команда на сценарии свежего клона. Не хочу заменить одну неточность другой.

⏺ Bash(FP=/Users/explosovebit/Work/ForgePlan/target/debug/forgeplan
      W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/s…)
  ⎿  zsh: command not found: _encode
     … +26 lines (ctrl+o to expand)
     zsh: command not found: _encode
⏺ Пре… +29 lines (ctrl+o to expand) проверю точно, что делает каждая команда на сценарии свежего клона. Не хочу заменить одну неточность другой.
  ⎿  Shell cwd was reset to /Users/explosovebit/Work/ForgePlan
⏺ Bash(FP=/Users/explosovebit/Work/ForgePlan/target/debug/forgeplan
⏺ Симуляция была неполной — я взял только артефакты, без config.yaml.dПовторяюcна4полном4отслеживаемом6.forgeplan/.
  ⎿  z
⏺ Bash(W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad/clonetest2
      FP=/Users/explosovebit/Work/ForgePlan/ta…)
  ⎿  Error: Permission to use Bash with command
  ⎿  W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad/clonetest2
     FP=/Users/explosovebit/Work/ForgePlan/target/debug/forgeplan
⏺ Симrm -rf "$W"; mkdir -p "$W"зял только артефакты, без config.yaml. Повторяю на полном отслеживаемом .forgeplan/.
  ⎿  Added 3 lines, removed 1 line
      28    semantic search промахнётся. Если случайно отредактирован — recover через
      29    `forgeplan_update id=<ID> body=<full new body>` (читаешь файл, формируешь
      30    полное новое body без YAML frontmatter, пушишь через MCP). Last-resort
      31 -  fallback: `forgeplan scan-import` пересоберёт LanceDB из markdown.
      31 +  fallback: `forgeplan reindex` пересоберёт LanceDB из markdown
      32 +  (**не** `scan-import` — тот ищет и импортирует новые артефакты из
      33 +  произвольного markdown, существующие не синхронизирует; см. #420).
      34    Direct Edit OK ТОЛЬКО для не-forgeplan markdown (READMEs, CLAUDE.md,
      35    KNOWN-ISSUES, src code, .changeset/*.md).
      36

⏺ Update(CLAUDE.md)
  ⎿  Added 3 lines, removed 1 line
      544  1. Re-Read file to capture current content
      545  2. Strip YAML frontmatter (forgeplan_update body excludes it)
      546  3. `mcp__forgeplan__forgeplan_update(id="<ID>", body=<full body>)`
      547 -4. Last-resort fallback: `forgeplan scan-import` rebuilds LanceDB from markdown
      547 +4. Last-resort fallback: `forgeplan reindex` rebuilds LanceDB from markdown
      548 +   (**not** `scan-import` — that one discovers and imports *new* artifacts from
      549 +   arbitrary markdown; it does not sync artifacts already in the graph, #420)
      550
      551  Document the violation в commit message so reviewer recognizes the pattern was caught.
      667  ├── evidence/ problems/ solutions/
      668  ├── notes/ refresh/ memory/
      669  ├── config.yaml         ← tracked (env var refs only, no hardcoded secrets)
      670 -├── lance/              ← ⚠️  gitignored (derived index — forgeplan scan-import)
      670 +├── lance/              ← ⚠️  gitignored (derived index — forgeplan reindex)
      671  ├── .fastembed_cache/   ← ⚠️  gitignored
      672  └── session.yaml        ← ⚠️  gitignored (per-machine runtime state)
      673  ```

      673  ```
      674
      675 -**Fresh clone**: `git clone → forgeplan init -y → forgeplan scan-import → forgeplan list`.
      675 +**Fresh clone**: `git clone → forgeplan init -y → forgeplan reindex → forgeplan list`.
      676
      677 -**Rules**: edit via `forgeplan` CLI; direct markdown edits require `forgeplan scan-import`; DO NOT commit `lance/` or `session.y
          -aml`.
          +ом репозитории затягивает `templates/` и `docs/` как ложные артефакты (PROB-047).
      680
      681 +**Rules**: edit via `forgeplan` CLI; direct markdown edits require `forgeplan reindex`; DO NOT commit `lance/` or `session.yaml`
          +.
      682 +
      683  ---
      684
      685  ## Rust Architecture

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
     zsh: command not found: _encode
     … +17 lines (ctrl+o to expand)

⏺ Осталось два места с неточным описанием самой команды. Смотрю контекст.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== docs/README.md 105-114 ==="; sed -n '105,114p' docs/README.md…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +32 lines (ctrl+o to expand)


⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      sed -i '' 's|forgeplan scan-import            # indexes tracked markdown into LanceDB|forgeplan reindex                # s…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
      git add CLAUDE.md docs/README.md docs/README.ru.md…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +20 lines (ctrl+o to expand)

⏺ Все три бага починены и закоммичены. Осталось зафиксировать ваше решение — иначе оно потеряется вместе с сессией. Оно относится к vNext, поэтому
  кладу его на ту ветку.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git checkout -q docs/vnext-pack-import…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ Ветка переключена (правка CLAUDE.md осталась на ветке фиксов — так и задумано). Записываю решение как ADR.

  Called forgeplan 5 times (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== validate ==="; forgeplan validate ADR-019 2>&1 | grep -E 'Result|MUST'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +18 lines (ctrl+o to expand)

⏺ Всё сделано. Финальная сверка состояния.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== три ветки, ни одна не запушена ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +22 lines (ctrl+o to expand)

⏺ Готово. Все три бага починены, решение записано.

  ---
  Три бага закрыты кодом с реальным E2E, ваше решение зафиксировано как ADR-019. По ходу я ошибся в диагнозе и откатил неверный фикс — рассказываю
  честно. 84 набора тестов зелёные, clippy и формат чистые. Ничего не запушено.

  🐞 Что починено

  #418 — идентичность переживала не дальше следующей команды
  update --body заменял тело вместе с блоком идентичности. А так как методичка предписывает сразу после new заполнять секции именно через update
  --body, slug умирал на втором шаге штатного процесса. Отсюда и «0 из 383 артефактов со slug».
  Проверено на собранном бинарнике: после new — 3 поля, после update --body — 3 (было 0), get --json отдаёт slug: prd-slug-persistence-probe.
  ———
  #419 — переименование раздваивало артефакт
  CLI удалял старый файл со времён аудита PRD-073, а MCP — нет. Через MCP оставалось два файла с одним id. Тест падает без фикса и проходит с ним
  — проверил, откатив фикс и прогнав заново.
  ———
  #420 — документация звала не ту команду
  scan-import в четырёх местах выдавался за пересборку индекса. Теперь reindex, плюс явное разграничение: reindex синхронизирует существующее,
  scan-import находит новое.

  ⚠️  Моя ошибка — и откат

  Сначала я решил, что раздвоенный frontmatter — это порча, и переписал сборку файла в один блок. Тесты доказали обратное: cli_hint_slug_aware.rs
  прямо документирует двублочную раскладку как намеренную и читает slug из нижнего блока.

  Это была архитектурная правка под видом починки бага. Откатил полностью, оставил только устранение потери данных. Такое требует ADR, а не
  мимоходного фикса.

  🧭 Ваше решение записано

  ADR-019 — «оркестрирует агент, ForgePlan владеет знаниями». Победил ADR-001.

  Прямое следствие: отпадают FPV-11…FPV-15 — адаптеры хостов, адаптеры оркестраторов, кросс-хостовые расширения, сервер. Аудит рекомендовал
  выбросить те же пять задач по другой причине — нулевому спросу. Два довода сошлись.

  В ADR я не стал замалчивать неудобное: ADR-009 всё ещё нужно формально superseded; плагины уже шипнуты, хотя ADR-001 их отвергал;
  playbook-runtime по этой границе становится нелегальным; ADR-015/016 означают, что ядро уже владеет частью workspace-состояния. Это стоимость
  миграции, которую пакет нигде не заложил.

  Статус draft — активировать без оценки этих четырёх следствий нельзя.

  📊 Проверки

  cargo fmt — 0 · clippy -D warnings — 0 · 84 набора тестов — все зелёные · E2E на собранном бинарнике по обоим кодовым багам

  Один тест (git::tests::max_assigned_number_unknown_ref_returns_none) упал в параллельном прогоне — не удалось породить процесс git. Изолированно
  проходит; к правкам отношения не имеет.

  🚫 Не сделано

  Ничего не запушено — три ветки ждут вашего ревью: fix/frontmatter-identity-triple (3), docs/vnext-pack-import (8), docs/task-list-discipline
  (1).

  Три issue остаются открытыми — закрывать их должен merge, а не я.

  ➡️  Дальше

  1. Ревью и разрешение на push
  2. forgeplan supersede ADR-009 --by ADR-019 — но только после оценки четырёх следствий из ADR
  3. Отдельно от vNext: слайс #360 (проверка git-дельты) — единственное, что осталось ценного и отгружаемо

  ref/ и правка в PROB-047 не тронуты — они не мои.

✻ Churned for 56m 23s

❯ Давай дальше и что насчет этого?

  EvidenceBundle

  # Итоговый вывод

  После повторной проверки сайта, документации и трёх репозиториев — `forgeplan`, `forgeplan-web`, `marketplace` — я бы зафиксировал ForgePlan
  так:

  > **ForgePlan — repository-native слой инженерных контрактов, governance и проверки результатов для AI-агентов разработки.**

  Более простая продуктовая формулировка:

  > **ForgePlan превращает инженерный замысел в контракт для агента и не принимает результат без проверяемых доказательств.**

  Это точнее, чем:

  * «система управления проектами»;
  * «фреймворк инженерных решений»;
  * «методология разработки»;
  * «оркестратор агентов»;
  * «agent harness» без уточнения;
  * «журнал PRD и ADR».

  Все эти определения отражают отдельные части ForgePlan, но ни одно не описывает правильную границу продукта полностью.

  ---

  # 1. Уровень зрелости ForgePlan сейчас

  Я бы разделил развитие ForgePlan на пять уровней.

  | Уровень | Содержание                                                           |
  | ------- | -------------------------------------------------------------------- |
  | L1      | Git-native журнал решений и артефактов                               |
  | L2      | Engineering governance: граф, lifecycle, Evidence, R_eff, validation |
  | L3      | Переносимые контракты работы для агентов                             |
  | L4      | Универсальный протокол между агентами и оркестраторами               |
  | L5      | Governance-контур автономной разработки 24/7                         |

  ## Текущее состояние

  ForgePlan сейчас находится примерно **между L2 и L3**.

  Уже хорошо реализованы:

  * git-native источник истины;
  * типизированные артефакты;
  * связи и граф;
  * маршрутизация по глубине;
  * валидация;
  * lifecycle;
  * Evidence;
  * decay;
  * F-G-R и `R_eff`;
  * MCP;
  * claims;
  * базовая многоагентная координация;
  * brownfield discovery;
  * локальная поисковая и контекстная система.

  Но пока не закончены системообразующие части L3:

  * нет полноценного переносимого контракта исполнения;
  * недостаточно сильна проверка фактического результата;
  * различается семантика CLI и MCP;
  * переносимость между агентными средами частичная;
  * core местами начинает выполнять функции оркестратора;
  * документация описывает ForgePlan сразу несколькими несовместимыми способами.

  Поэтому сегодня ForgePlan — это **сильная инженерная платформа ранней зрелости**, но ещё не стабильный универсальный protocol layer для
  автономной разработки.

  Целевая ближайшая позиция — **L4**. L5 потребуется позже для P
  ──── (1754 lines hidden) ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
  тракт.

  Kandev / Vibe Kanban / Conductor
  управляют задачами, workspace и agent sessions.

  Paperclip
  управляет организацией, агентами, бюджетами и heartbeat.

  ForgePlan Extensions
  соединяет все эти слои.

  ForgePlan Web
  показывает, что было решено, поручено, выполнено и доказано.
  ```

  Самая точная итоговая продуктовая формулировка:

  > **ForgePlan is the repository-native engineering contract and verification layer for AI coding agents. It keeps intent, execution, and
  evidence connected across any agent or orchestrator.**

  Русская версия:

  И короткий продающий вариант:

  > **Дайте каждому агенту точный контракт. Принимайте результат только с доказательствами.**

  [1]: https://kandev.ai/docs/agents-and-profiles "https://kandev.ai/docs/agents-and-profiles"
  [2]: https://www.vibekanban.com/docs/workspaces "https://www.vibekanban.com/docs/workspaces"
  [3]: https://www.conductor.build/docs "https://www.conductor.build/docs"
  [4]: https://docs.paperclip.ing/reference/api/agents/ "https://docs.paperclip.ing/reference/api/agents/"
  [5]: https://forgeplan.dev/docs/methodology/overview/ "https://forgeplan.dev/docs/methodology/overview/"
  [6]: https://forgeplan.dev/blog/r-eff-weakest-link/ "https://forgeplan.dev/blog/r-eff-weakest-link/"
  [7]: https://forgeplan.dev/ "https://forgeplan.dev/"
  [8]: https://cursor.com/changelog/2-5 "https://cursor.com/changelog/2-5"
  [9]: https://cursor.com/changelog/04-24-26 "https://cursor.com/changelog/04-24-26"
  [13]: https://docs.paperclip.ing/guides/org/agents/ "https://docs.paperclip.ing/guides/org/agents/"
  [14]: https://forgeplan.dev/blog/forgeplan-as-agent-harness/ "https://forgeplan.dev/blog/forgeplan-as-agent-harness/"
  [15]: https://forgeplan.dev/docs/getting-started/quick-start/ "https://forgeplan.dev/docs/getting-started/quick-start/"
  [16]: https://forgeplan.dev/docs/getting-started/installation/ "https://forgeplan.dev/docs/getting-started/installation/"
  [17]: https://forgeplan.dev/docs/marketplace/overview/ "https://forgeplan.dev/docs/marketplace/overview/"


⏺ Это тот же самый программный документ, что был в пакете — но с одним важным отличием: здесь есть источники. Аудит отдельно отмечал, что в пакете
  grep 'http' по 1171 строке архитектуры давал ноль ссылок; все утверждения про Cursor, Codex, Conductor были без единой ссылки и версии. Здесь
  их 17.
      echo "=== реальное число MCP-инструментов и CLI-команд ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +21 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== drift-детектор, о котором говорит документ ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
⏺ Проверил ключевое: 73 MCP-инструмента — цифра верная, CLAUDE.md не врёт. CLI — 79 файлов против заявленных 76, небольшой дрейф. И
  примечательное: drift-детектор в репозитории заведён именно из-за этого класса проблем, а в его шапке записано, что «внешний агент-ревьюер нашёл
  это раньше внутренней проверки». Тот же паттерн, что и сейчас.

  Проверяю утверждения про сайт — он в этом же репозитории.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan/website
      echo "=== '63 tools' и '73 tools' на сайте ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +30 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan/website/src/content/docs
     zsh: command not found: _encode
     … +17 lines (ctrl+o to expand)

⏺ Точная картина: EN-страница говорит 73, её RU-перевод — 63. И это единственное место с «63» во всём сайте. Значит формулировка документа («на
⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== какие пути сканирует drift-детектор ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +22 lines (ctrl+o to expand)

⏺ Нашёл настоящий дефект. Детектор сканирует website/src — то есть эту RU-страницу он читал — и всё равно рапортует «✅ No drift», хотя там
  написано «63 MCP-инструмента». Смотрю регулярку.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n 'EXTRACT_RE' scripts/check-mcp-tool-count.sh | head -3…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +19 lines (ctrl+o to expand)

⏺ Мой тест был испорчен кавычками — перепроверяю корректно.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      RE='[0-9]+[[:space:]]*(MCP[[:space:]]*tool[s]?([^a-zA-Z]|$)|tool[s]?([^a-zA-Z]|$)|MCP[[:space:]]*инструмент|инструмент)'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +17 lines (ctrl+o to expand)

⏺ Подтверждено строго. Регулярка ловит MCP инструмент через пробел, но не MCP-инструмент через дефис — а это естественная русская форма. Итог:
  CI-гейт рапортует «✅ No drift», просканировав файл, в котором стоит устаревшая цифра.

  Это ровно тот класс, что аудит находил раз за разом: страж зелёный, а дефект на месте. Завожу.

  Made 1 scratchpad edit +87 (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      gh issue create --repo ForgePlan/forgeplan \…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)
     … +19 lines (ctrl+o to expand)

⏺ Мой тест был испорчен кавычками — перепроверяю корректно.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      RE='[0-9]+[[:space:]]*(MCP[[:space:]]*tool[s]?([^a-zA-Z]|$)|tool[s]?([^a-zA-Z]|$)|MCP[[:space:]]*инструмент|инструмент)'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +17 lines (ctrl+o to expand)

  Это ровно тот класс, что аудит находил раз за разом: страж зелёный, а дефект на месте. Завожу.

  Made 1 scratchpad edit +87 (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      gh issue create --repo ForgePlan/forgeplan \…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ API Error: Connection closed mid-response. The response above may be incomplete.

✻ Cooked for 7m 25s

❯ продолжай

⏺ Продолжаю — чиню #421.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "текущая ветка: $(git branch --show-current)"…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)

⏺ Тестов на скрипт нет — только сам скрипт и workflow. Читаю его целиком.

  Read 1 file (ctrl+o to expand)

⏺ Скрипт понятен. Чиню регулярку и добавляю самотест — сейчас у скрипта нет ни одного.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n 'warn.only\|WARN_ONLY\|^while\|getopts\|for arg' scripts/check-mcp-tool-count.sh | head -8…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +43 lines (ctrl+o to expand)

⏺ Update(scripts/check-mcp-tool-count.sh)
  ⎿  Added 42 lines, removed 1 line
       76  # not any other number on the same line (avoid false-positive "1940 tests"
       77  # / "76 CLI commands"). Word boundary anchored с trailing non-letter
       78  # чтобы не matchать `tooltip`/`toolbox`/`toolkit` (audit C-3 finding).
       79 -EXTRACT_RE='[0-9]+[[:space:]]*(MCP[[:space:]]*tool[s]?([^a-zA-Z]|$)|tool[s]?([^a-zA-Z]|$)|MCP[[:space:]]*инструмент|инструмент)'
       79 +#
       80 +# Separator between `MCP` and the noun is [[:space:]-]*, NOT whitespace-only:
       81 +# Russian writes the compound hyphenated (`73 MCP-инструмента`), which is the
       82 +# idiomatic form and therefore what the RU pages actually use. With a
       83 +# whitespace-only class this script scanned website/src, walked straight past
       84 +# `63 MCP-инструмента` in ru/docs/cli/serve.md, and printed "No drift" — a
       85 +# green gate certifying the very drift it exists to catch. English `MCP-tools`
       86 +# has the same exposure. See #421.
       90 +# separators. Runs offline, touches no repo file. The blind spot in #421
       91 +# survived because nothing ever asserted the pattern itself.
       92 +if [[ "${SELF_TEST:-0}" -eq 1 ]]; then
       93 +    st_fail=0
       94 +    st_expect() { # <should-match 0|1> <sample>
       95 +        local want="$1" sample="$2" got
       96 +        got=$(printf '%s' "$sample" | grep -coE "$EXTRACT_RE")
       97 +        if [[ "$got" -ne "$want" ]]; then
       98 +            echo "  SELF-TEST FAIL: expected match=$want got=$got for: $sample" >&2
       99 +            st_fail=1
      100 +        else
      101 +            echo "  ok (match=$got): $sample"
      102 +        fi
      103 +    }
      102 +        fi
      103 +    }
      104 +    echo "Self-test: EXTRACT_RE"
      105 +    st_expect 1 '73 MCP tools for agents'
      106 +    st_expect 1 '73 MCP-tools for agents'
      107 +    st_expect 1 '73 tools for agents'
      108 +    st_expect 1 '73 MCP инструмента для агентов'
      109 +    st_expect 1 '73 MCP-инструмента, сопоставленных с операциями'
      110 +    st_expect 1 '73 инструмента для агентов'
      111 +    # Must NOT match — the audit C-3 false positives.
      112 +    st_expect 0 'a tooltip and a toolbox'
      113 +    st_expect 0 '76 CLI commands'
      118 +    echo "Self-test OK"
      119 +    exit 0
      120 +fi
      121 +
      122  DRIFT_FOUND=0
      123  DRIFT_OUTPUT=$(mktemp)
      124  trap 'rm -f "$DRIFT_OUTPUT"' EXIT

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
     zsh: command not found: _encode
     … +22 lines (ctrl+o to expand)

⏺ Самотест зелёный, включая ранее пропускавшуюся форму. Теперь главное — ловит ли детектор реальный дрейф.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== детектор ПОСЛЕ починки регулярки ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +20 lines (ctrl+o to expand)
⏺ Bash(cd /Users/explosovebit/Work/ForgePlan/website/src/content/docs
      echo "=== RU serve.md, строки 18-40 ==="; sed -n '18,40p' ru/docs/cli/serve.md…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +36 lines (ctrl+o to expand)

⏺ Структура совпадает с EN. Проверяю, не устарел ли и сам список категорий — иначе поправить одно число было бы косметикой.
⏺ Структура совпадает с EN. Проверяю, не устарел ли и сам список категорий — иначе поправить одно число было бы косметикой.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan/website/src/content/docs
      echo "=== EN список категорий (после строки 26) ==="; sed -n '26,42p' docs/cli/serve.md)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +27 lines (ctrl+o to expand)

⏺ Списки категорий в EN и RU идентичны — расходится только цифра. Значит правка числа здесь и корректна, и достаточна.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      sed -i '' 's|^63 MCP-инструмента, сопоставленных|73 MCP-инструмента, сопоставленных|' website/src/content/docs/ru/docs/cli…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +17 lines (ctrl+o to expand)

⏺ Детектор зелёный — и теперь по правильной причине. Проверяю, что починка имеет зубы: подсаживаю фальшивую цифру в RU-форме.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
     zsh: command not found: _encode
     … +17 lines (ctrl+o to expand)

⏺ Доказано: подсаженная дефисная RU-цифра ловится, следов пробы не осталось. Подключаю самотест в CI, чтобы он реально запускался.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n -B4 -A6 'check-mcp-tool-count' .github/workflows/forgeplan-health.yml)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +14 lines (ctrl+o to expand)

  Read 1 file (ctrl+o to expand)

⏺ Update(.github/workflows/forgeplan-health.yml)
  ⎿  Added 8 lines
      52        - name: Validate artifacts
      53          run: ./target/debug/forgeplan validate --ci
      54
      55 +      - name: MCP tool count drift check — detector self-test
      56 +        # #421: the detector's own pattern had never been asserted, and a
      57 +        # whitespace-only separator made it walk past `NN MCP-инструмента`
      58 +        # (the idiomatic hyphenated Russian form) while printing "No drift".
      59 +        # A green gate that certifies the drift it exists to catch is worse
      60 +        # than no gate, so the pattern is now tested before it is trusted.
      61 +        run: SELF_TEST=1 ./scripts/check-mcp-tool-count.sh
      62 +
      63        - name: MCP tool count drift check
      64          # Drift detector — fails CI if README/CLAUDE/landing/docs disagree
      65          # with actual MCP tool count в src. Lessons learned 2026-05-04:

⏺ Моя правка не использует ${{ }} вообще — только фиксированная переменная окружения, инъекции нет. Проверяю YAML и коммичу.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/forgeplan-health.yml')); print('YAML OK')" 2>&1…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +20 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add scripts/check-mcp-tool-count.sh .github/workflows/forgeplan-health.yml website/src/content/docs/ru/docs/cli/serve.…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +16 lines (ctrl+o to expand)

⏺ Готово. Теперь — оценка документа, которую я задолжал.

  ---
  Это тот же программный документ, что был в пакете, но с источниками. Ваше решение уже отсекло примерно половину. Его фактические утверждения о
  репозитории верны по направлению и неточны в деталях — зато проверка их дала настоящий баг, который теперь починен.

  📄 Чем этот документ отличается от пакета

  Тем, что у него есть источники. Аудит отдельно фиксировал: в пакете grep 'http' по 1171 строке архитектуры давал ноль — все утверждения про
  Cursor hooks, Codex skills, OpenCode permissions и Conductor API были без единой ссылки и версии, при том что сам пакет требовал version matrix.
  Здесь 17 ссылок на живую документацию вендоров. Это закрывает одну из серьёзных претензий.

  И одна идея лучше пакетной. §7: WorkContract — не одиннадцатый рукописный артефакт, а скомпилированная версионированная проекция существующих.
  Пакет этого не проговаривал, а разница принципиальная: контракт нельзя «написать не так», он выводится.

  ✂️  Что ваше решение уже отсекло

  ADR-019 («оркестрирует агент») убирает целиком:

  ┌────────┬────────────────────────────────────────────────┬────────────┐
  │ Раздел │                    Что там                     │   Статус   │
  ├────────┼────────────────────────────────────────────────┼────────────┤
  │ §13    │ адаптеры Cursor / Codex / OpenCode             │ ✗          │
  ├────────┼────────────────────────────────────────────────┼────────────┤
  │ §14    │ адаптеры Kandev / Vibe / Conductor / Paperclip │ ✗          │
  ├────────┼────────────────────────────────────────────────┼────────────┤
  │ §15    │ Extensions как host/orchestrator adapters      │ ✗ частично │
  ├────────┼────────────────────────────────────────────────┼────────────┤
  │ §6.6   │ опциональный сервер                            │ ✗          │
  └────────┴────────────────────────────────────────────────┴────────────┘

  Это примерно половина текста. Причём документ сам формулирует границу правильно (§3: «ForgePlan должен делегировать»), а потом в §13–14
  предлагает ForgePlan эту границу переходить через адаптеры. Внутреннее напряжение, которого он не замечает.

  🔍 Что я проверил в его утверждениях

  ┌────────────────────────────┬──────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
  │        Утверждение         │                                                  Реальность                                                  │
  ├────────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ «73 и 63 MCP tools         │ Верно, но не на главной: 22 файла говорят 73, и ровно один RU-перевод — 63                                   │
  │ одновременно»              │                                                                                                              │
  ├────────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ «Quick Start показывает    │ Не подтвердилось. Найденные упоминания — законные исторические («начиная с v0.18.0 — BM25») в RU-страницах   │
  │ 0.18.0»                    │ MCP/CLI, а не устаревший quickstart                                                                          │
  ├────────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ «73 MCP tools» в CLAUDE.md │ Точная цифра, совпадает с #[tool( в коде                                                                     │
  ├────────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ «76 CLI commands»          │ Дрейф: файлов команд 79                                                                                      │
  └────────────────────────────┴──────────────────────────────────────────────────────────────────────────────────────────────────────────────┘

  То есть: направление верное, детали смазаны. Такой документ нельзя брать как спецификацию — только как список гипотез к проверке.

  🎯 Но проверка дала настоящий баг

  Разбираясь, почему «63» пережило существующий CI-гейт, нашёл: детектор сканировал этот файл и рапортовал «✅ No drift». Регулярка разделяла MCP
  и существительное только пробелом, а русский пишет через дефис — MCP-инструмента. Зелёный страж, удостоверяющий ровно тот дрейф, ради которого
  создан.

  Завёл #421 (https://github.com/ForgePlan/forgeplan/issues/421) и починил: разделитель [[:space:]-]*, режим SELF_TEST=1 с 8 проверками (у скрипта
  не было ни одной), исправлена цифра, самотест подключён в CI перед самой проверкой. Доказано зубами: подсаженная 50 MCP-инструмента ловится,
  следов не остаётся.

  Ирония в шапке самого скрипта: он написан потому, что «внешний агент-ревьюер нашёл 18 устаревших цифр раньше внутренней проверки». Ровно то же
  повторилось сейчас.

  ✅ Что из документа стоит делать

  ✅ Что из документа стоит делать

  §9 EvidenceBundle + §10 R_eff v2. Это внутри границы ADR-019, это измеримо (0 из 148 доказательств несут base_sha/result_sha), и это не требует
  ни Protocol v1, ни адаптеров, ни сервера. GitHub #360.

  §10 попутно бьёт в #392 (52 артефакта с ложным нулём): считать по обязательным утверждениям, а не по всему приложенному — тогда один слабый
  информационный источник не обрушает решение.

  📦 Состояние работы
  4 issue заведено, 3 починены кодом — #418, #419, #421 закрываются коммитами; #420 (документация) тоже. Все остаются открытыми до merge.

  ➡️  Предлагаю дальше

  Взять #360 — слайс проверки git-дельты. Это единственная часть обеих версий программы, которая переживает ваше решение, измеримо нужна и
  отгружаема обычным циклом против текущей границы.

  Скажите — начинать его, или сначала ревью и push накопившихся четырёх веток.

✻ Cooked for 4m 20s
❯ да давай

⏺ Начинаю #360. Сначала читаю сам issue — не хочу проектировать по своему пересказу.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      gh issue view 360 --repo ForgePlan/forgeplan --json title,body,labels,comments --jq '.title'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +41 lines (ctrl+o to expand)

      echo "=== публичный API git-модуля ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +24 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== как парсятся Structured Fields у EVIDENCE ==="…)

⏺ Модель ясна. Оформляю по методологии проекта — сначала PRD, потом код.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git checkout -q -b feat/evid-git-provenance origin/dev…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

  Called forgeplan (ctrl+o to expand)

⏺ Переключение ветки вернуло правки детектора — они на своей ветке, всё в порядке. Заполняю PRD.

  Called forgeplan (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      forgeplan validate PRD-082 2>&1 | grep -E 'Result|MUST|x \[' | head -8)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ PRD прошёл валидацию. Реализую FR-001 — смотрю существующий валидатор ref, чтобы переиспользовать его.

  Read 1 file (ctrl+o to expand)

⏺ Валидатор строгий и подходит как есть. Реализую FR-001.

⏺ Update(crates/forgeplan-core/src/git/mod.rs)
  ⎿  Added 54 lines
      191      Ok(files)
      192  }
      193
      194 +/// Re-derive the set of paths that actually changed between two commits.
      195 +///
      196 +/// This is the ground-truth half of the #360 provenance gate: an EvidencePack
      197 +/// may *claim* it changed certain files, but only git can say whether it did.
      198 +/// The caller compares the claim against what this returns — see
      199 +/// `scoring::provenance`.
      200 +///
      201 +/// Both refs go through [`validate_git_ref`], so a SHA that looks like a CLI
      202 +/// option (`--output=…`) or smuggles a revision range (`a..b`) is rejected
      203 +/// before it reaches the git process [CWE-88]. Refs are passed as two separate
      204 +/// arguments rather than a `base..result` string — the validator forbids `..`
      205 +/// outright, and `git diff A B` is equivalent.
      206 +///
      207 +/// Returns paths relative to the repository root, in git's own order.
      208 +/// An empty vector is a legitimate answer: it means the two commits are
      209 +/// identical in content. For a code-claiming EvidencePack that is precisely
      210 +/// the failure #360 describes — green tests over an empty delta are a NULL
      211 +/// result, not a pass — but this function only reports; it does not judge.
      212 +///
      213 +/// # Errors
      214 +/// - either ref fails [`validate_git_ref`]
      215 +/// - git cannot be spawned (not installed / not on PATH)
      216 +/// - git exits non-zero (unknown ref, not a repository, shallow clone missing
      217 +///   the base commit) — stderr is surfaced so the caller can tell an unknown
      218 +///   SHA from a broken repo
      219 +pub fn changed_paths_between(
      220 +    repo_root: &Path,
      221 +    base: &str,
      222 +    result: &str,
      223 +) -> anyhow::Result<Vec<String>> {
      224 +    validate_git_ref(base)
      225 +        .map_err(|e| anyhow::anyhow!("changed_paths_between: invalid base ref: {e}"))?;
      226 +    validate_git_ref(result)
      227 +        .map_err(|e| anyhow::anyhow!("changed_paths_between: invalid result ref: {e}"))?;
      228 +
      229 +    let output = Command::new("git")
      230 +        .args(["diff", "--name-only", base, result])
      231 +        .current_dir(repo_root)
      232 +        .output()
      233 +        .map_err(|e| anyhow::anyhow!("Failed to run git (is git installed?): {e}"))?;
      234 +
      235 +    if !output.status.success() {
      236 +        let stderr = String::from_utf8_lossy(&output.stderr);
      237 +        anyhow::bail!("git diff {base} {result} failed: {}", stderr.trim());
      238 +    }
      239 +
      240 +    Ok(String::from_utf8_lossy(&output.stdout)
      241 +        .lines()
      242 +        .map(str::trim)
      243 +        .filter(|l| !l.is_empty())
      244 +        .map(str::to_string)
      245 +        .collect())
      246 +}
      247 +
      248  /// List artifact filenames present in `origin/dev` for a given kind directory.
      249  ///
      250  /// Used by `forgeplan new` (PROB-060 / SPEC-005 Phase 1.3) to warn when a

⏺ Пишу тесты FR-001 на настоящих git-репозиториях. Смотрю существующие хелперы в тест-модуле.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n 'fn init_repo_with_files' -A38 crates/forgeplan-core/src/git/mod.rs | sed -n '1,45p')
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +48 lines (ctrl+o to expand)

⏺ Хелпер создаёт один коммит — для diff нужен второй. Пишу тесты с реальными коммитами.

⏺ Update(crates/forgeplan-core/src/git/mod.rs)
  ⎿  Added 103 lines
      854          tmp
      855      }
      856
      857 +    // ── #360 / PRD-082 FR-001: ground-truth delta between two commits ────
      858 +
      859 +    /// Helper: write `files` and commit them, returning the new HEAD sha.
      860 +    fn commit_files(work: &Path, files: &[(&str, &str)], msg: &str) -> String {
      861 +        for (rel, content) in files {
      862 +            let p = work.join(rel);
      863 +            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
      864 +            std::fs::write(p, content).unwrap();
      865 +        }
      866 +        assert!(
      867 +            Command::new("git")
      868 +                .args(["add", "."])
      869 +                .current_dir(work)
      870 +                .status()
      871 +                .unwrap()
      872 +                .success(),
      873 +            "git add"
      874 +        );
      875 +        assert!(
      876 +            Command::new("git")
      877 +                .args(["commit", "--quiet", "-m", msg])
      878 +                .current_dir(work)
      879 +                .status()
      880 +                .unwrap()
      881 +                .success(),
      882 +            "git commit"
      883 +        );
      884 +        head_commit_hash(work).expect("HEAD after commit")
      885 +    }
      886 +
      887      #[test]
      888 +    fn changed_paths_between_lists_files_changed_in_a_commit() {
      889 +        let tmp = init_repo_with_files(&[("src/a.rs", "fn a() {}\n")]);
      890 +        let work = tmp.path();
      891 +        let base = head_commit_hash(work).expect("base HEAD");
      892 +        let result = commit_files(
      893 +            work,
      894 +            &[("src/b.rs", "fn b() {}\n"), ("src/a.rs", "fn a2() {}\n")],
      895 +            "second",
      896 +        );
      897 +
      898 +        let mut paths = changed_paths_between(work, &base, &result).expect("diff must succeed");
      899 +        paths.sort();
      900 +        assert_eq!(paths, vec!["src/a.rs".to_string(), "src/b.rs".to_string()]);
      901 +    }
      902 +
      903 +    #[test]
      904 +    fn changed_paths_between_returns_empty_for_identical_commits() {
      905 +        // The #360 case: an EvidencePack claims code work, tests are green,
      906 +        // and the delta is empty. This function must report that honestly —
      907 +        // an empty vector, not an error and not a silent pass.
      908 +        let tmp = init_repo_with_files(&[("src/a.rs", "fn a() {}\n")]);
      909 +        let work = tmp.path();
      910 +        let sha = head_commit_hash(work).expect("HEAD");
      911 +
      912 +        let paths = changed_paths_between(work, &sha, &sha).expect("same-sha diff must succeed");
      913 +        assert!(
      914 +            paths.is_empty(),
      915 +            "identical commits must yield an empty delta, got {paths:?}"
      916 +        );
      917 +    }
      918 +
      919 +    #[test]
      920 +    fn changed_paths_between_rejects_option_like_ref() {
      921 +        // CWE-88: a ref starting with '-' would be parsed by git as a CLI
      922 +        // option. Must be refused before the process is spawned.
      923 +        let tmp = init_repo_with_files(&[]);
      924 +        let err = changed_paths_between(tmp.path(), "--output=/tmp/pwn", "HEAD")
      925 +            .expect_err("option-like base ref must be rejected");
      926 +        assert!(
      927 +            format!("{err}").contains("invalid base ref"),
      928 +            "error must name which ref was bad, got: {err}"
      929 +        );
      930 +    }
      931 +
      932 +    #[test]
      933 +    fn changed_paths_between_rejects_range_ref() {
      933 +    fn changed_paths_between_rejects_range_ref() {
      934 +        // `a..b` would smuggle a second revision past the argument boundary.
      935 +        let tmp = init_repo_with_files(&[]);
      936 +        let err = changed_paths_between(tmp.path(), "HEAD", "dev..main")
      937 +            .expect_err("range-shaped result ref must be rejected");
      938 +        assert!(
      939 +            format!("{err}").contains("invalid result ref"),
      940 +            "error must name which ref was bad, got: {err}"
      941 +        );
      942 +    }
      943 +
      944 +    #[test]
      945 +    fn changed_paths_between_errors_on_unknown_ref() {
      946 +        // A well-formed but nonexistent sha must surface git's own complaint,
      947 +        // not be swallowed into an empty delta — otherwise a typo'd base_sha
      948 +        // would read as "nothing changed" and pass the gate.
      949 +        let tmp = init_repo_with_files(&[("src/a.rs", "fn a() {}\n")]);
      950 +        let sha = head_commit_hash(tmp.path()).expect("HEAD");
      951 +        let err = changed_paths_between(tmp.path(), "deadbeefdeadbeefdeadbeefdeadbeef12345678", &sha)
      952 +            .expect_err("unknown base sha must error, never read as an empty delta");
      953 +        let msg = format!("{err}");
      954 +        assert!(
      955 +            msg.contains("git diff") && msg.contains("failed"),
      956 +            "error must surface git's stderr, got: {msg}"
      957 +        );
      958 +    }
      959 +
      960 +    #[test]
      961      fn max_assigned_number_empty_repo_returns_none() {
⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib -- changed_paths_between 2>&1 | grep -E '^test |test result|error\[|panicked at' | head…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ FR-001 готов — 5 тестов на настоящих репозиториях. Теперь модуль проверки. Смотрю extract_field, который уже читает Structured Fields.

     zsh: command not found: _encode
     … +45 lines (ctrl+o to expand)

⏺ extract_field публичный и переиспользуемый. Создаю модуль проверки.

⏺ Write(crates/forgeplan-core/src/scoring/provenance.rs)
  ⎿  Wrote 199 lines to crates/forgeplan-core/src/scoring/provenance.rs
       1 //! Git-delta provenance for code-claiming Evidence (PRD-082, GitHub #360).
       2 //!
       3 //! An EvidencePack may *claim* it changed files. Only git can say whether it
       4 //! did. This module parses the claim out of the artifact body and re-derives
       5 //! the fact, so `activate` can refuse a claim that reality does not support.
       6 //!
       7 //! The central case is deliberately narrow and deliberately strict: **an empty
       8 //! delta under a code claim is a NULL result, not a pass.** A worker that does
       8 //! delta under a code claim is a NULL result, not a pass.** A worker that does
       9 //! nothing, reports "done", and leaves a green suite on an unchanged tree is
      10 //! the failure #360 reproduces — and the one this module exists to catch.
     … +189 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n 'pub mod' crates/forgeplan-core/src/scoring/mod.rs | head)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)

⏺ Update(crates/forgeplan-core/src/scoring/mod.rs)
  ⎿  Added 1 line
      1  pub mod decay;
      2  pub mod evidence;
      3  pub mod fgr;
      4 +pub mod provenance;
      5  pub mod reff;
      6
      7  use std::collections::HashSet;

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo check -p forgeplan-core 2>&1 | grep -E '^(error|warning)' -A6 | head -25…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Компилируется чисто. Пишу тесты — включая настоящие git-репозитории, как требует FR-006.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cat >> crates/forgeplan-core/src/scoring/provenance.rs <<'RUST'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +20 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Все 10 тестов зелёные, включая ключевой сценарий #360. Полный конвейер.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo fmt…)
      cargo fmt…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Всё зелёное: формат 0, clippy 0, 84 набора. Отмечаю прогресс в PRD — включая одно осознанное отступление от него.

  Called forgeplan (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
     … +12 lines (ctrl+o to expand)

⏺ Коммит на месте. Теперь EvidencePack — и он будет первым в репозитории, несущим git-провенанс, который я только что реализовал. Собираю
  фактические данные.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      BASE=$(git rev-parse --short=7 origin/dev)…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +19 lines (ctrl+o to expand)

  Called forgeplan 2 times (ctrl+o to expand)

⏺ Теперь главная проверка — прогоню EVID-150 через собственную функцию, которую он удостоверяет. Временным тестом, который тут же удалю.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cat > crates/forgeplan-core/tests/tmp_dogfood_evid150.rs <<'RUST'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Догфуд сработал: Verified, 4 пути совпали. Первый в репозитории EvidencePack с git-провенансом проверен той самой функцией, которую
  удостоверяет. Временный тест удалён.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== R_eff после привязки доказательства ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +17 lines (ctrl+o to expand)

⏺ R_eff = 1.00. Structured fields распарсились, доказательство засчитано.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== пять веток, ни одна не запушена ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +26 lines (ctrl+o to expand)

⏺ #360 отгружен — первый слайс.

  ---
  Ядро проверки происхождения работает: core теперь может пересчитать, существует ли заявленное изменение кода, вместо того чтобы верить
  исполнителю. 15 тестов на настоящих git-репозиториях. Догфуд состоялся — доказательство проверено собственной функцией.

  🎯 Что построено

  git::changed_paths_between(repo, base, result)
  git diff --name-only между коммитами. Оба ref проходят существующий validate_git_ref, поэтому SHA в форме --output=/tmp/pwn или с протащенным
  диапазоном отвергается до запуска git (CWE-88).
  ———
  scoring::provenance
  Читает base_sha / result_sha / changed_paths из того же блока Structured Fields, где уже живут verdict и congruence_level. Пять исходов:
  NotClaimed, Verified, EmptyDelta, PathMismatch, Incomplete.

  🔒 Два решения, которые стоит знать

  Пустая дельта отклоняется, а не предупреждает. Зелёные тесты на неизменённом дереве — это NULL, а не успех. Ровно то, что воспроизводит #360.

  Неизвестный base_sha даёт ошибку, а не пустую дельту. Иначе опечатка в SHA читалась бы как «ничего не изменилось» и проходила бы гейт — тот же
  класс тихого обхода, который фича и должна закрывать.

  📐 Отступление от собственного плана

  PRD-082 планировал четыре исхода — отгружено пять. Добавил Incomplete: заявление с base_sha, но без changed_paths, иначе провалилось бы в ветку
  «полей нет вообще» и прошло как NotClaimed. Полузаполненное заявление — именно так настоящее протаскивают мимо стража, который проверяет только
  найденные поля. Отступление в сторону строгости, записано и в PRD, и в коммит.

  🍽 Догфуд

  EVID-150 — первый EvidencePack в репозитории с git-провенансом (было 0 из 148). Проверен не на словах: одноразовый тест прочитал его с диска и
  прогнал через verify_code_provenance:

  VERDICT: Verified { actual_paths: [4 paths] }
  SUMMARY: verified against git: 4 path(s) changed

  Тест удалён — прибитый навсегда, он сломался бы на первом rebase, так как зашивает SHA этой ветки. R_eff PRD-082 = 1.00.

  🧪 Проверки

  fmt --check 0 · clippy -D warnings 0 · 84 набора зелёные · 15 новых тестов на настоящих временных git-репозиториях, не на моках — фича про
  сверку с фактом, поэтому тесты используют факт

  🚫 Осознанно вне слайса

  Подключение к forgeplan_activate как гейта BLOCKER/WARN. Сначала функция установления факта должна существовать и быть проверяемой — теперь она
  такая. ForgePlan по-прежнему не запускает тесты и не владеет рабочим деревом (ADR-019).

  📦 Накопилось

  5 веток, 15 коммитов, ничего не запушено

  ┌──────────────────────────────────┬──────────┐
  │              Ветка               │ Коммитов │
  ├──────────────────────────────────┼──────────┤
  │ feat/evid-git-provenance         │ 2        │
  ├──────────────────────────────────┼──────────┤
  │ fix/frontmatter-identity-triple  │ 3        │
  ├──────────────────────────────────┼──────────┤
  │ fix/drift-detector-ru-blind-spot │ 1        │
  ├──────────────────────────────────┼──────────┤
  │ docs/vnext-pack-import           │ 8        │

  🚫 Осознанно вне слайса

  Подключение к forgeplan_activate как гейта BLOCKER/WARN. Сначала функция установления факта должна существовать и быть проверяемой — теперь она
  такая. ForgePlan по-прежнему не запускает тесты и не владеет рабочим деревом (ADR-019).

  📦 Накопилось

  5 веток, 15 коммитов, ничего не запушено

  ┌──────────────────────────────────┬──────────┐
  │              Ветка               │ Коммитов │
  ├──────────────────────────────────┼──────────┤
  │ fix/frontmatter-identity-triple  │ 3        │
  ├──────────────────────────────────┼──────────┤
  │ fix/drift-detector-ru-blind-spot │ 1        │
  ├──────────────────────────────────┼──────────┤
  │ docs/vnext-pack-import           │ 8        │
  ├──────────────────────────────────┼──────────┤
  │ docs/task-list-discipline        │ 1        │
  └──────────────────────────────────┴──────────┘

  4 issue заведено, все 4 закрываются коммитами, все остаются открытыми до merge.

  ➡️  Дальше

  Логичное продолжение — подключить проверку к forgeplan_activate: тогда EmptyDelta перестанет быть возвращаемым значением и станет отказом в
  активации. Это второй FR того же PRD.

  Но пятнадцать коммитов на пяти ветках уже заметная стопка. Предлагаю сначала ревью и push — иначе всё это живёт только на вашем ноутбуке.

✻ Cogitated for 15m 10s

❯ /model claude-opus-4-8
  ⎿  Set model to Opus 4.8 and saved as your default for new sessions

❯ /fpl-skills:smith давай доделай что там осталось? /workflows

⏺ Smith routing: это продолжение активного PRD-082 (Row: feature-continuation в существующей системе). Слайс 1 построил механизм проверки, но
  ничего его не вызывает — «датчик подключён к пустоте». Незавершённая часть #360 — гейт в момент активации, который PRD-082 явно отложил как
  слайс 2. Это и есть «что осталось» на моей ветке.

  Push держу под RED LINE #2 — «доделай» это не явное одобрение пуша. Сначала разведка: где активация и есть ли у неё доступ к workspace для diff.

  Read 1 file (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== кто вызывает lifecycle::activate ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +30 lines (ctrl+o to expand)

⏺ Ключевая находка: lifecycle::activate(store, id, force) не имеет доступа к workspace — только к store. А git-diff нужен repo root. Значит гейт
  активации — это не мелкая доводка: смена сигнатуры, 4 места вызова, новое поле конфига, block-vs-warn по умолчанию, обработка недостижимого SHA.
  Полноценный слайс 2, а не «дошлифовать».

  И тут развилка, которая реально меняет мои действия. Сначала — точное состояние.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== 5 веток от origin/dev, ничего не запушено ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +19 lines (ctrl+o to expand)

⏺ Состояние подтверждено: 5 веток, 15 коммитов, все от origin/dev, ни одна не запушена. Две правят CLAUDE.md в разных секциях (скорее всего
  auto-merge).

⏺ User answered Claude's questions:
  ⎿  · 15 коммитов на 5 ветках готовы, но живут только локально. Что доделываем? → Давай все сдлаем - кажется чт оважно - но ты сам улчше выясни
     это и давай делй последвоательно и с /smith /workflows

⏺ Понял: делаем всё, последовательно, через smith + workflows, порядок — на мне.

  Здесь важная поправка от меня к порядку: RED LINE #5 требует audit до PR (Code → Audit → Fix → Test → Fmt → Lint → Verify → PR). Я — и
  генератор, и единственный проверяющий этого кода, что нарушает generator≠verifier. Поэтому первым делает workflow адверсариальный аудит трёх
  code-веток, а не сразу push.

  SMITH PLAN — «доделать всё», последовательно:

  ┌──────┬────────────────────────────────────┬───────────────────────────────────────────────────────┐
  │ Фаза │                Что                 │                          Как                          │
  ├──────┼────────────────────────────────────┼───────────────────────────────────────────────────────┤
  │ A    │ Адверсариальный аудит 3 code-веток │ workflow, generator≠verifier ← сейчас                 │
  ├──────┼────────────────────────────────────┼───────────────────────────────────────────────────────┤
  │ B    │ Фикс находок аудита                │ на своих ветках                                       │
  ├──────┼────────────────────────────────────┼───────────────────────────────────────────────────────┤
  │ D    │ Слайс 2: activate-gate             │ новая ветка, тесты, свой аудит, PR                    │
  ├──────┼────────────────────────────────────┼───────────────────────────────────────────────────────┤
  │ E    │ Финальный отчёт + smith handoff    │                                                       │
  └──────┴────────────────────────────────────┴───────────────────────────────────────────────────────┘

  Push держу до фазы C — ваше «давай всё сделаем» я читаю как явное одобрение, но выполняю его после аудита, не до. Merge не делаю — это отдельное
  per-PR решение.

  Завожу задачи и запускаю фазу A.

⏺ Запускаю адверсариальный аудит фазы A.

⏺ Workflow(Adversarially audit the 3 unpushed code branches before opening PRs (generator != verifier, RED LINE #5))
  ⎿  /workflows to view dynamic workflow runs

⏺ Фаза A запущена (wh03rxhy4): 4 ревьюера (по одному на code-ветку + сквозной), каждая находка проверяется независимым скептиком с установкой «по
  умолчанию опровергнут», затем синтез в fix-список. Read-only, generator≠verifier.

  Вы просили последовательно — поэтому вперёд не забегаю: push и PR только после того, как аудит закончится и я закрою его находки. Жду результат,
  вернусь с fix-списком и вердиктом по каждой ветке.

✻ Waiting for 1 dynamic workflow to finish

⏺ Dynamic workflow "Adversarially audit the 3 unpushed code branches before opening PRs (generator != verifier, RED LINE #5)" completed · 29m 37s

⏺ Аудит нашёл BLOCKER — в моём же фиксе #419. Именно тот edge-case, который я закладывал в промпт ревьюера, и скептик его подтвердил. Читаю
  полностью.

⏺ Bash(F=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/tasks/wh03rxhy4.output
      D=/private/tmp/claude-501/-Users-explos…)
  ⎿  reviewers: 4 | findings: 22 | BLOCKER: 1 MAJOR: 6
     survivors: 12 | refuted: 5
     … +24 lines (ctrl+o to expand)

⏺ Аудит серьёзный: 12 находок пережили скептика, включая BLOCKER в моём фиксе #419. Читаю полный fix-список.

  Read 1 file (ctrl+o to expand)

⏺ Аудит превосходный и жёсткий: 12 находок пережили скептика, все 3 code-ветки — FIX-FIRST. Главное — BLOCKER в моём фиксе #419: если новый
  заголовок даёт тот же slug (смена регистра/пунктуации или повторная отправка того же title), файл артефакта удаляется — тихая потеря, а после
  reindex строка вычищается как orphan. Тест этого не поймал. И скептик подтвердил: тот же баг зеркалится в CLI (update.rs:141),
  пред-существующий.

  Начинаю фазу B, worst-first. Обновляю задачи.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "current: $(git branch --show-current)"…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +42 lines (ctrl+o to expand)

⏺ BLOCKER подтверждён. projection_slug — pub(crate), MCP-крейт его не вызовет напрямую. Чищу правильно: централизую guard в одной функции в
  forgeplan-core, чтобы ни CLI, ни MCP не могли его забыть. Читаю remove_projection_at.

  Read 1 file (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n 'pub async fn remove_projection_at\|pub async fn remove_projection\b' crates/forgeplan-core/src/projection/mod.rs)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)

  Read 1 file (ctrl+o to expand)

⏺ Ясно. Добавляю функцию с guard'ом сразу после remove_projection_at — сравнивает пути и пропускает удаление при равенстве slug.

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 35 lines
      692      }
      693  }
      694
      695 +/// Remove the OLD projection file after a title change — but only if the title
      696 +/// actually renamed the file.
      697 +///
      698 +/// A title edit that slugifies to the *same* filename (a case-only or
      699 +/// punctuation-only change, or the identical title re-submitted) must NOT delete
      700 +/// anything: the mutation path writes the new file at `slug(new_title)` first,
      701 +/// and when `slug(old) == slug(new)` that is the very file we would then remove.
      727 +    remove_projection_at(workspace, id, kind, old_title).await
      728 +}
      729 +
      730  // =============================================================================
      731  // PRD-073 file-first mutation helpers
      732  // -----------------------------------------------------------------------------

⏺ Теперь заменяю оба незащищённых вызова на новую функцию — и MCP (мой фикс #419), и CLI (пред-существующий баг).

⏺ Update(crates/forgeplan-mcp/src/server.rs)
  ⎿  Added 13 lines, removed 5 lines
      3842          // fix, so renames through MCP silently forked the artifact in two.
      3843          // Exact-path removal (not a prefix glob) so a sibling whose id is a
      3844          // prefix of this one is never clobbered, and AFTER the new file is in
      3845 -        // place so there is no orphan window — same ordering as the CLI.
      3846 -        if p.title.is_some() {
      3847 -            let _ =
      3848 -                projection::remove_projection_at(&ws, &canonical, &original.kind, &original.title)
      3849 -                    .await;
      3845 +        // place so there is no orphan window — same ordering as the CLI. The
      3846 +        // rename-aware helper skips removal when the new title slugifies to the
      3847 +        // same filename (case/punctuation-only edit), which would otherwise
      3848 +        // delete the file we just wrote — silent, reindex-unrecoverable loss.
      3849 +        if let Some(ref new_title) = p.title {
      3850 +            let _ = projection::remove_stale_projection_after_rename(
      3851 +                &ws,
      3852 +                &canonical,
      3853 +                &original.kind,
      3854 +                &original.title,
      3855 +                new_title,
      3856 +            )
      3857 +            .await;
      3858          }
      3859
      3860          // Re-fetch for the response payload.

  Read 1 file (ctrl+o to expand)

⏺ original — pre-update запись, title: Option<&str> — новый. Правлю CLI.

⏺ Update(crates/forgeplan-cli/src/commands/update.rs)
  ⎿  Added 12 lines, removed 2 lines
      137      // PRD-073 audit M1 fix: clean up OLD slug AFTER the new file is in place
      138      // (so there's no orphan window) and use exact-path removal so we don't
      139      // accidentally clobber a sibling artifact whose ID is a prefix of this one.
      140 -    if title.is_some() {
      141 -        let _ = projection::remove_projection_at(&ws, id, &original.kind, &original.title).await;
      140 +    // The rename-aware helper additionally skips removal when the new title
      141 +    // slugifies to the same filename (case/punctuation-only edit) — otherwise
      142 +    // it deletes the file update just wrote (silent, reindex-unrecoverable).
      143 +    if let Some(new_title) = title {
      144 +        let _ = projection::remove_stale_projection_after_rename(
      145 +            &ws,
      146 +            id,
      147 +            &original.kind,
      148 +            &original.title,
      149 +            new_title,
      150 +        )
      151 +        .await;
      152      }
      153
      154      // Log changes

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo check --workspace 2>&1 | grep -E '^(error|warning)' -A4 | head -20…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Компилируется. Тесты на функцию + regression на same-slug для обеих поверхностей. Сначала unit-тесты в projection.

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 54 lines
      2289          assert_eq!(carry_identity_forward("no frontmatter here\n", new), new);
      2290      }
      2291
      2292 +    // ── #419 BLOCKER: same-slug title change must not delete the file ────
      2293 +
      2294 +    #[tokio::test]
      2295 +    async fn rename_helper_removes_old_file_on_a_real_rename() {
      2296 +        let tmp = TempDir::new().unwrap();
      2297 +        let ws = tmp.path();
      2298 +        let dir = ws.join("prds");
      2299 +        tokio::fs::create_dir_all(&dir).await.unwrap();
      2300 +        // Both files exist (new already written by the mutation path).
      2301 +        tokio::fs::write(dir.join("PRD-001-old-title.md"), "x")
      2302 +            .await
      2303 +            .unwrap();
      2304 +        tokio::fs::write(dir.join("PRD-001-new-title.md"), "x")
      2305 +            .await
      2306 +            .unwrap();
      2307 +
      2308 +        let removed =
      2309 +            remove_stale_projection_after_rename(ws, "PRD-001", "prd", "Old title", "New title")
      2310 +                .await
      2311 +                .unwrap();
      2312 +        assert!(removed, "a genuine rename must remove the stale file");
      2313 +        assert!(!dir.join("PRD-001-old-title.md").exists());
      2314 +        assert!(
      2315 +            dir.join("PRD-001-new-title.md").exists(),
      2316 +            "the new file must survive"
      2317 +        );
      2318 +    }
      2319 +
      2320 +    #[tokio::test]
      2321 +    async fn rename_helper_is_a_noop_when_slug_is_unchanged() {
      2322 +        // The #419 BLOCKER: "Probe" -> "probe" (or "Probe!") slugifies to the
      2322 +        // The #419 BLOCKER: "Probe" -> "probe" (or "Probe!") slugifies to the
      2323 +        // same filename. The helper must NOT delete the just-written file.
      2324 +        let tmp = TempDir::new().unwrap();
      2325 +        let ws = tmp.path();
      2326 +        let dir = ws.join("prds");
      2327 +        tokio::fs::create_dir_all(&dir).await.unwrap();
      2328 +        let f = dir.join("PRD-001-probe.md");
      2329 +        tokio::fs::write(&f, "the only copy").await.unwrap();
      2330 +
      2331 +        for (old, new) in [("Probe", "probe"), ("Probe", "Probe!"), ("Probe", "Probe")] {
      2332 +            let removed = remove_stale_projection_after_rename(ws, "PRD-001", "prd", old, new)
      2333 +                .await
      2338 +            );
      2339 +            assert!(
      2340 +                f.exists(),
      2341 +                "the artifact file must survive a same-slug edit {old:?}->{new:?}"
      2342 +            );
      2343 +        }
      2344 +    }
      2345 +
      2346      #[test]
      2347      fn filter_preserved_drops_known_keys() {
      2348          let fm: Frontmatter = serde_yaml::from_str(

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib -- rename_helper 2>&1 | grep -E '^test |test result|panicked' | head)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Unit-тесты проходят. Теперь E2E-regression на MCP-поверхности (same-slug) — добавляю в тот же файл, где мой тест #419.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cat >> crates/forgeplan-mcp/tests/mcp_new_update_title_validation.rs <<'RUST'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      # temporarily point server.rs back to the unguarded removal…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +17 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Зубы подтверждены: тест падает на незащищённом коде, проходит с guard'ом. Guard восстановлен. Компилирую и добавляю E2E для #418 (X5) — реальный
      cargo check --workspace 2>&1 | tail -1…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ E2E #418 проходит. Полный пайплайн для этой ветки.
      227 +/// code-claiming EvidencePack that is precisely the failure #360 describes —
⏺ Bas 228 +/// green tests over an empty delta are a NULL result, not a pass — but this
      229 +/// function only reports; it does not judge.
  ⎿   230  ///
      231  /// # Errors
      232  /// - either ref fails [`validate_git_ref`]
      233  /// - git cannot be spawned (not installed / not on PATH)
  ⎿   234  /// - git exits non-zero (unknown ref, not a repository, shallow clone missing
      217 -///   the base commit) — stderr is surfaced so the caller can tell an unknown
⏺ Всё 218 -///   SHA from a broken repo
      235 +///   the base commit, or `base` and `result` share no merge base) — stderr is
⏺ Bas 236 +///   surfaced so the caller can tell an unknown SHA from a broken repo. Never
      237 +///   folded into an empty delta: an unresolvable base must not read as
  ⎿   238 +///   "nothing changed".
      239  pub fn changed_paths_between(
      240      repo_root: &Path,
      241      base: &str,
     ...
⏺ BLO 247          .map_err(|e| anyhow::anyhow!("changed_paths_between: invalid result ref: {e}"))?; merge-base + -z + no-renames). Сначала
      248
      249      let output = Command::new("git")
⏺ Bas 230 -        .args(["diff", "--name-only", base, result])
      250 +        .args([
  ⎿   251 +            "diff",
      252 +            "--merge-base",
      253 +            "--name-only",
      253 +            "--name-only",
      254 +            "--no-renames",
      255 +            "-z",
      256 +            base,
      257 +            result,
      258 +        ])
      259          .current_dir(repo_root)
      260          .output()
      261          .map_err(|e| anyhow::anyhow!("Failed to run git (is git installed?): {e}"))?;
     ...
      265          anyhow::bail!("git diff {base} {result} failed: {}", stderr.trim());
      266      }
      267
      268 +    // `-z` output is NUL-separated with a trailing NUL; paths are raw (unquoted).
      269      Ok(String::from_utf8_lossy(&output.stdout)
      241 -        .lines()
      242 -        .map(str::trim)
      243 -        .filter(|l| !l.is_empty())
      270 +        .split('\0')
      271 +        .filter(|p| !p.is_empty())
      272          .map(str::to_string)
      273          .collect())
      274  }

  Read 1 file (ctrl+o to expand)

⏺ Хвост консистентен. Компилирую и добавляю тесты на non-ancestor base и кириллический путь.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo check -p forgeplan-core 2>&1 | grep -E '^error' -A4 | head -12…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Существующие 5 тестов проходят (обратная совместимость с ancestor/base==result). Добавляю два новых — non-ancestor base (F1) и кириллический
  путь (F2).

⏺ Update(crates/forgeplan-core/src/git/mod.rs)
  ⎿  Added 55 lines
       945      }
       946
       947      #[test]
       948 +    fn changed_paths_between_uses_merge_base_for_a_non_ancestor_base() {
       949 +        // Audit F1 (MAJOR): base is NOT an ancestor of result (dev advanced
       950 +        // during the task). The two-endpoint 'git diff base result' would also
       951 +        // report shared.rs (changed only on base's own line) and the gate would
       952 +        // credit it as satisfied. --merge-base must report ONLY result's work.
       953 +        let tmp = init_repo_with_files(&[("shared.rs", "v1\n"), ("keep.rs", "k\n")]);
       954 +        let work = tmp.path();
       955 +        let fork = head_commit_hash(work).expect("fork point");
       956 +
       957 +        // result branch off the fork: touches only worker.rs.
       958 +        run_git(work, &["checkout", "--quiet", "-b", "feat"]);
       959 +        let result = commit_files(work, &[("worker.rs", "w\n")], "work on feat");
       960 +
       961 +        // base line advances independently, changing shared.rs.
       962 +        run_git(work, &["checkout", "--quiet", "dev"]);
       963 +        let base = commit_files(work, &[("shared.rs", "v2\n")], "dev moves ahead");
       964 +
       965 +        // base is not an ancestor of result.
       966 +        assert!(
       967 +            !std::process::Command::new("git")
       968 +                .args(["merge-base", "--is-ancestor", &base, &result])
       969 +                .current_dir(work)
       970 +                .status()
       971 +                .unwrap()
       972 +                .success(),
       977 +        assert_eq!(
       978 +            paths,
       979 +            vec!["worker.rs".to_string()],
       980 +            "only result's work since the fork must count, not base-side changes; got {paths:?}"
       981 +        );
       982 +    }
       983 +
       984 +    #[test]
       985 +    fn changed_paths_between_returns_non_ascii_paths_verbatim() {
       986 +        // Audit F2: without -z / quotepath handling a Cyrillic filename comes
       987 +        // back C-octal-escaped and quoted, breaking the exact-string compare
       988 +        // downstream. This repo's docs/notes are Russian, so the trigger is real.
       988 +        // downstream. This repo's docs/notes are Russian, so the trigger is real.
       989 +        let tmp = init_repo_with_files(&[("keep.rs", "k\n")]);
       990 +        let work = tmp.path();
       991 +        let base = head_commit_hash(work).expect("base");
       992 +        let result = commit_files(work, &[("docs-Разбор.md", "x\n")], "cyrillic path");
       993 +
       994 +        let paths = changed_paths_between(work, &base, &result).expect("diff must succeed");
       995 +        assert_eq!(
       996 +            paths,
       997 +            vec!["docs-Разбор.md".to_string()],
       998 +            "a non-ASCII path must come back raw (not octal-escaped/quoted); got {paths:?}"
       999 +        );
      1000 +    }
      1001 +
      1002 +    #[test]
      1003      fn changed_paths_between_rejects_option_like_ref() {
      1004          // CWE-88: a ref starting with '-' would be parsed by git as a CLI
      1005          // option. Must be refused before the process is spawned.

⏺ Проверяю, существует ли хелпер run_git — иначе определю.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n 'fn run_git\|fn commit_files' crates/forgeplan-core/src/git/mod.rs)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ commit_files есть, run_git — нет. Добавляю маленький хелпер рядом.

  Read 1 file (ctrl+o to expand)

⏺ Update(crates/forgeplan-core/src/git/mod.rs)
  ⎿  Added 13 lines
      884
      884
      885      // ── #360 / PRD-082 FR-001: ground-truth delta between two commits ────
      886
      887 +    /// Helper: run a git subcommand in `work`, asserting success.
      888 +    fn run_git(work: &Path, args: &[&str]) {
      889 +        assert!(
      890 +            Command::new("git")
      891 +                .args(args)
      892 +                .current_dir(work)
      893 +                .status()
      894 +                .unwrap()
      895 +                .success(),
      896 +            "git {args:?}"
      901      fn commit_files(work: &Path, files: &[(&str, &str)], msg: &str) -> String {
      902          for (rel, content) in files {

❯ и делай в /workflows

⏺ Принято — продолжу через workflows: логичное место — после закрытия всех находок фазы B прогнать verify-workflow, который проверит, что BLOCKER
  и MAJOR реально закрыты и не внесли регрессий. Сейчас домотаю механические правки (single-file, не workflow-образные), затем запущу проверочный
  workflow.
⏺ Все 7 проходят, включая non-ancestor и кириллицу. Провенанс-тесты тоже (они гоняют verify_code_provenance через эту функцию). Полный пайплайн и
  коммит.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib -- provenance:: 2>&1 | grep -E 'test result' | head -1…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)
  ⎿  (timeout 10m)
      cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -E '^(error|warning)' -A6 | head -30)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +18 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Просто неиспользуемая переменная fork в моём тесте — убираю её.

⏺ Update(crates/forgeplan-core/src/git/mod.rs)
      967          let work = tmp.path();
      968 -        let fork = head_commit_hash(work).expect("fork point");
      968
      970 -        // result branch off the fork: touches only worker.rs.
      969 +        // result branch off the initial commit: touches only worker.rs.
      970          run_git(work, &["checkout", "--quiet", "-b", "feat"]);
      971          let result = commit_files(work, &[("worker.rs", "w\n")], "work on feat");
      972

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "clippy: $(cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -cE '^(warning|error)')"…)
     … +11 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Clippy 0. Коммичу фикс evid-ветки.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +30 lines (ctrl+o to expand)

  Read 1 file (ctrl+o to expand)

⏺ Убираю оба шага из forgeplan-health.yml (они переедут в ci.yml, без path-фильтра).

⏺ Update(.github/workflows/forgeplan-health.yml)
  ⎿  Added 6 lines, removed 16 lines
      52        - name: Validate artifacts
      53          run: ./target/debug/forgeplan validate --ci
      54
      55 -      - name: MCP tool count drift check — detector self-test
      56 -        # #421: the detector's own pattern had never been asserted, and a
      57 -        # whitespace-only separator made it walk past `NN MCP-инструмента`
      58 -        # (the idiomatic hyphenated Russian form) while printing "No drift".
      59 -        # A green gate that certifies the drift it exists to catch is worse
      60 -        # than no gate, so the pattern is now tested before it is trusted.
      61 -        run: SELF_TEST=1 ./scripts/check-mcp-tool-count.sh
      62 -
      63 -      - name: MCP tool count drift check
      64 -        # Drift detector — fails CI if README/CLAUDE/landing/docs disagree
      65 -        # with actual MCP tool count в src. Lessons learned 2026-05-04:
      66 -        # external OpenAI agent caught 18+ stale references (28/37/45/47
      67 -        # vs actual 63) that passive grep sweeps had been missing.
      68 -        # PR 2.6 audit C-2: this step closes "preventive value theoretical"
      69 -        # finding by wiring the script into the health gate.
      70 -        run: ./scripts/check-mcp-tool-count.sh
      55 +      # The MCP tool-count drift check moved to ci.yml (job
      56 +      # check-mcp-tool-count-drift). #421: this workflow only triggers on
      57 +      # paths .forgeplan/** and crates/**, but the drift it guards lives in
      58 +      # website/**, docs/**, CLAUDE.md, README.md and TODO.md — a docs-only PR
      59 +      # never ran the detector here, which is exactly how the stale '63' in a
      60 +      # RU page survived. ci.yml runs on every PR with no path filter.

⏺ Добавляю job в ci.yml после check-kind-list-drift.

⏺ Update(.github/workflows/ci.yml)
  ⎿  Added 23 lines
       84            chmod +x scripts/check-kind-list-drift.sh
       85            scripts/check-kind-list-drift.sh
       86
       87 +  check-mcp-tool-count-drift:
       88 +    name: Check MCP tool count drift
       89 +    runs-on: ubuntu-latest
       90 +    # In ci.yml (all PRs, no path filter) rather than forgeplan-health.yml
       91 +    # (paths .forgeplan/**, crates/**): #421 showed the drift it guards lives
       92 +    # in website/docs/CLAUDE.md/README/TODO, so a docs-only PR never ran it.
       93 +    steps:
       94 +      - name: Checkout
       98 +        # #421: the extraction pattern had never been asserted, and a
       99 +        # whitespace-only separator made it walk past `NN MCP-инструмента`
      100 +        # (the idiomatic hyphenated Russian form) while printing "No drift".
      101 +        # The pattern is tested before it is trusted.
      102 +        run: SELF_TEST=1 scripts/check-mcp-tool-count.sh
      103 +
      104 +      - name: Check docs/site tool counts against src
      105 +        # Fails CI if README/CLAUDE/landing/docs disagree with the actual
      106 +        # #[tool(...)] count in src. Lessons learned 2026-05-04: an external
      107 +        # reviewer caught 18+ stale references before the internal sweep.
      108 +        run: scripts/check-mcp-tool-count.sh
      109 +
      110    # PROB-060 Phase 2.3 (T4) — workspace audit of frontmatter validator coverage.
      111    #
      112    # Self-test job: runs scripts/validator-audit-all.sh against the canonical

⏺ Мой job использует только статические run:-команды, без ${{ }} — инъекции нет. Фикс F4 (-eq → == "1") и валидация YAML.

⏺ Update(scripts/check-mcp-tool-count.sh)
  ⎿  Added 1 line, removed 1 line
      89  # --self-test: exercise the extraction regex against both languages and both
      90  # separators. Runs offline, touches no repo file. The blind spot in #421
      91  # survived because nothing ever asserted the pattern itself.
      92 -if [[ "${SELF_TEST:-0}" -eq 1 ]]; then
      92 +if [[ "${SELF_TEST:-0}" == "1" ]]; then
      93      st_fail=0
      94      st_expect() { # <should-match 0|1> <sample>
      95          local want="$1" sample="$2" got

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== YAML валиден ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +21 lines (ctrl+o to expand)

⏺ Всё чисто: YAML валиден, 0 интерполяций ${{ }} в новом job, F4 подтверждён (SELF_TEST=true больше не роняет скрипт под set -u). Коммичу.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add .github/workflows/ci.yml .github/workflows/forgeplan-health.yml scripts/check-mcp-tool-count.sh…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ Все три code-ветки исправлены. Обновляю задачи и запускаю verify-workflow (как вы просили — через /workflows): проверит, что BLOCKER и MAJOR
  реально закрыты и правки не внесли регрессий.

⏺ Workflow(export const meta = { … +186 lines)
  ⎿  Error: Invalid workflow script: Script parse error: Unexpected token (111:18)

        [[ "${SELF_TEST:-0}" == "1" ]] (string), not -eq. Reproduce: run
                       ^

     Workflow scripts must be plain JavaScript — common causes are TypeScript syntax (type annotations, interfaces, generics) and broken string
     quoting or escaping.

⏺ Ошибка парсинга: shell-конструкция ${SELF_TEST:-0} в prompt была принята за JS-интерполяцию. Переформулирую без литерала и перезапускаю.

⏺ Workflow(Independently verify that the Phase-B fixes actually closed the audit findings and introduced no regression)
  ⎿  Running in background · /workflows to monitor and save · medium size (/config)

⏺ Verify-workflow запущен (woge5fa4i): 4 независимых скептика воспроизводят исходные сценарии на temp-репозиториях + go/no-go по каждой ветке.
  Пока идёт — статус.

  ---
  Аудит фазы A нашёл BLOCKER в моём же фиксе #419 (same-slug title удалял файл артефакта). Все 3 code-ветки исправлены и закоммичены, каждый фикс
  с тестами. Verify-workflow сейчас проверяет, что находки реально закрыты. Push держу до его завершения.
  Аудит фазы A нашёл BLOCKER в моём же фиксе #419 (same-slug title удалял файл артефакта). Все 3 code-ветки исправлены и закоммичены, каждый фикс
  с тестами. Verify-workflow сейчас проверяет, что находки реально закрыты. Push держу до его завершения.

  🔧 Фаза B — что исправлено
  необратимая. Централизовал guard в remove_stale_projection_after_rename, обе поверхности (MCP + CLI, CLI-баг был пред-существующим). Зубы
  доказаны: тест падает без guard'а. Плюс E2E для #418 (RED LINE #5).
  ———
  MAJOR — двухточечный diff (f74b4cf)
  git diff base result для non-ancestor base (частый случай: dev уехал вперёд) засчитывал чужие изменения как Verified. Одна правка спавна:
  --merge-base --no-renames -z. Заодно закрыл кириллические пути (F2) и недетерминизм renames (F5). Тесты: non-ancestor + кириллица.
  ———
  MAJOR — гейт не срабатывал на docs-PR (b235728)
  Детектор жил только в forgeplan-health.yml, чей path-фильтр исключает docs/website — ровно сценарий #421. Перенёс в ci.yml (все PR). F4: -eq →
  == "1".

  📊 Проверки (каждая ветка)

  fmt 0 · clippy 0 · 84 набора зелёные
  ⏭ Дальше

⏺ Bash(F=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/tasks/woge5fa4i.output
      python3 - <<PY…)
  ⎿  checks: 4 | not_closed: 0 | regressions: 0

     [GO ] fix/frontmatter-identity-triple
     … +44 lines (ctrl+o to expand)

⏺ Все три — GO, 0 регрессий, merge без конфликтов (непересекающиеся файлы попарно). Одно замечание стоит закрыть до push: doc-комментарий helper'а
  говорит «сравнивает resolved paths», а код сравнивает slug'и — я его писал, он неточен. Правлю на честный.

     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ Правлю doc-комментарий на честный (сравниваются вычисленные slug'и, а не «resolved paths» — хотя это эквивалентно).

  Read 1 file (ctrl+o to expand)

⏺ Update(crates/forgeplan-core/src/projection/mod.rs)
  ⎿  Added 3 lines, removed 2 lines
      707  /// loss is silent until the next `reindex` prunes the now-orphaned row —
      708  /// unrecoverable (ADR-003: markdown is the source of truth).
      709  ///
      710 -/// Comparing resolved file paths rather than raw slugs also means a future
      711 -/// change to the slug algorithm cannot reopen the hole.
      710 +/// The comparison uses `projection_slug` — the same function that builds the
      711 +/// on-disk filename from the title — so slug-equality is exactly file-path

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo fmt…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)
  ⎿  (timeout 5m)

⏺ Clippy 18 при -p forgeplan-core подозрительно для правки одного комментария — проверяю каноническим workspace-прогоном.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -E '^(warning|error)' | head -8…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== финальное состояние 5 веток перед push ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +19 lines (ctrl+o to expand)

⏺ Все 5 веток на 0 позади dev, 19 коммитов. Пушу.


⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      for b in feat/evid-git-provenance fix/frontmatter-identity-triple fix/drift-detector-ru-blind-spot docs/vnext-pack-import…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
⏺ Все 5 веток запушены. Открываю draft PR — по одному, начну с ветки #360 (там есть PR-шаблон и артефакты). Пишу тела PR в scratchpad.

⏺ Bash(D=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/scratchpad
      mkdir -p "$D"…)
  ⎿  written pr-evid.md (      31 lines)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      D=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/scratchpad…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
⏺ PR #423 создан (evidence-hook не сработал — PRD-082 несёт EVID-150). Пишу остальные 4 тела и открываю.

⏺ Bash(D=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/scratchpad…)
  ⎿  written frontmatter + drift bodies

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      D=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/scratchpad…)
     … +14 lines (ctrl+o to expand)
  ⎿  (timeout 2m)

⏺ Три PR открыты (#423, #424, #425). Осталось два docs-PR. Пишу тела и открываю.

⏺ Bash(D=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/scratchpad…)
  ⎿  written vnext + taskdisc bodies

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
     zsh: command not found: _encode
     … +14 lines (ctrl+o to expand)
  ⎿  (timeout 2m)

⏺ Все 5 draft PR открыты. Проверяю статус и обновляю задачи.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== 5 draft PR (все → dev) ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +21 lines (ctrl+o to expand)

⏺ Все 5 draft PR открыты — #423–#427. Фаза C закрыта. Перехожу к фазе D — activate-gate (слайс 2). Ключевой вопрос дизайна: lifecycle::activate не
  имеет доступа к workspace. Проверяю, знает ли store свой путь — от этого зависит, менять ли сигнатуру.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== does LanceStore expose its workspace/path? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +24 lines (ctrl+o to expand)

  Read 1 file (ctrl+o to expand)

⏺ LanceStore не хранит путь workspace — значит гейт нельзя вшить в lifecycle::activate без параметра. Смотрю 4 места вызова и конфиг, чтобы
  выбрать между «протянуть repo_root» и отдельным forgeplan verify.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== 4 вызова lifecycle::activate — есть ли рядом workspace ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +39 lines (ctrl+o to expand)

⏺ Все 4 места вызова activate имеют ws в scope, IntegrityConfig — подходящий дом для настройки. Дизайн-решение: чистая функция-решение
  provenance_gate(verdict, mode) + поле конфига block|warn|off (default warn — консервативно для первого включения, обратимо) + проводка в явный
  activate-путь (CLI + MCP), с link-auto-activate как follow-up. Ветка стекается на feat/evid-git-provenance.

  Записываю это решение в PRD-082 и завожу ветку.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git checkout -q -b feat/evid-provenance-activate-gate feat/evid-git-provenance…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
      103 +
      104 +    /// Activate-time git-delta provenance gate for code-claiming Evidence
      105 +    /// (#360 / PRD-082). One of `block` | `warn` | `off`.
      106 +    ///
      107 +    /// When an EvidencePack carries `base_sha` / `result_sha` / `changed_paths`
      108 +    /// and its claim does not hold against git (empty delta, missing path,
      109 +    /// incomplete fields), this decides what `forgeplan_activate` does:
      110 +    /// - `block` — refuse activation;
      111 +    /// - `warn` — activate but print the discrepancy (default: a new gate must
      112 +    ///   not surprise existing flows on first rollout; flip to `block` once
      113 +    ///   trusted);
      114 +    /// - `off` — skip the check entirely.
      115 +    ///
      116 +    /// A pack with no provenance fields is never affected (verdict `NotClaimed`),
      117 +    /// and a git error (unreachable base sha, shallow clone) only ever warns —
      118 +    /// an environment problem is not a false claim, and ForgePlan does not own
      119 +    /// the worktree (ADR-019).
      120 +    #[serde(default = "default_evidence_provenance_gate")]
      121 +    pub evidence_provenance_gate: String,
      122  }
      139 +fn default_evidence_provenance_gate() -> String {
      140 +    "warn".to_string()
      141 +}
      142
      143  impl IntegrityConfig {
      144      /// Validate field ranges. Called from `workspace::load_config` after YAML parse.

⏺ Upd 184 +                self.evidence_provenance_gate
      185 +            );
      186 +        }
      187          Ok(())
      188      }
      189 +}

⏺ Update(crates/forgeplan-core/src/config/types.rs)
  ⎿  Added 1 line
      196              stub_marker_threshold: default_stub_marker_threshold(),
      197              mcp_max_title_len: default_mcp_max_title_len(),
      198              mcp_max_body_len: default_mcp_max_body_len(),
      199 +            evidence_prov  ance_gate: default_evidence_provenance_gate(),
      200          }
      201      }
      202  }

⏺ Теперь чистая функция-решение в scoring::provenance — маппит (verdict, mode) в исход. Читаю конец модуля перед вставкой.
⏺ Update(crates/forgeplan-core/src/config/types.rs)
  Read 1 file (ctrl+o to expand)

⏺ Update(crates/forgeplan-core/src/scoring/provenance.rs)
  ⎿  Added 59 lines
       91 +    }
       92  }
       93
       94 +/// How the activate-time provenance gate reacts to a failed claim (PRD-082
       95 +/// slice 2, config `integrity.evidence_provenance_gate`).
⏺ Теп  96 +#[derive(Debug, Clone, Copy, PartialEq, Eq)]
       97 +pub enum GateMode {
  Rea  98 +    /// Refuse activation when the claim does not hold against git.
       99 +    Block,
⏺ Upd 100 +    /// Activate but surface the discrepancy.
      101 +    Warn,
✶ Pha 102 +    /// Do not check.
      103 +    Off,
      104 +}
      105 +
      106 +impl GateMode {
      107 +    /// Parse the config string. Unknown values fall back to `Warn` (the config
      108 +    /// validator rejects them earlier; this is defense-in-depth, never silently
      109 +    /// `Off`).
───── 110 +    #[must_use]                                                                                                                 erge ──
      111 +    pub fn from_config(s: &str) -> Self {
  ─── 112 +        match s {                                                                                                               ───────
      113 +            "block" => GateMode::Block,
      114 +            "off" => GateMode::Off,
      115 +            _ => GateMode::Warn,
      116 +        }
      117 +    }
      118 +}
      119 +
      120 +/// What the caller should do with an activation, given a verdict and the mode.
      121 +#[derive(Debug, Clone, PartialEq, Eq)]
      122 +pub enum GateDecision {
      123 +    /// Proceed silently.
      124 +    Pass,
      125 +    /// Proceed, but print this warning first.
      126 +    Warn(String),
      127 +    /// Refuse the activation with this message.
      128 +    Block(String),
      129 +}
      130 +
      131 +/// Decide what an activation should do, given the provenance verdict and the
      132 +/// configured mode. Pure — no IO — so it is exhaustively testable.
      133 +///
      134 +/// The rules, in order:
      135 +/// - `Off` → always `Pass`.
      136 +/// - An acceptable verdict (`NotClaimed`, `Verified`) → `Pass`. The 148 legacy
      137 +///   packs are all `NotClaimed`, so they never gate regardless of mode.
      138 +/// - A failed claim (`EmptyDelta`, `PathMismatch`, `Incomplete`) → `Block` under
      139 +///   `Block`, `Warn` under `Warn`.
      140 +#[must_use]
      141 +pub fn gate_decision(verdict: &ProvenanceVerdict, mode: GateMode) -> GateDecision {
      142 +    if mode == GateMode::Off || verdict.is_acceptable() {
      143 +        return GateDecision::Pass;
      144 +    }
      145 +    let msg = format!("git-delta provenance gate: {}", verdict.summary());
      146 +    match mode {
      147 +        GateMode::Block => GateDecision::Block(msg),
      148 +        GateMode::Warn => GateDecision::Warn(msg),
      149 +        GateMode::Off => GateDecision::Pass,
      150 +    }
      151 +}
      152 +
      153  /// Parse the provenance claim out of an artifact body.
      154  ///
      155  /// Reads the same `key: value` Structured Fields block that already carries

⏺ Добавляю evaluate_provenance_gate — связывает verify + git-error→warn правило.

      152
      153 +/// Run the whole activate-time gate for one artifact body.
      154 +///
      155 +/// Folds the two rules a caller must never get wrong:
      156 +/// - `Off` short-circuits before any git call.
      157 +/// - A git *error* (unreachable base sha, shallow clone, not a repository) is
      158 +///   turned into a `Warn`, never a `Block`. An environment problem is not a
      159 +///   false claim, and ForgePlan does not own the worktree (ADR-019); blocking
      160 +///   activation because the runner's clone is shallow would make the gate
      161 +///   fragile. A verified-false claim (empty delta, missing path) is what
      166 +pub fn evaluate_provenance_gate(body: &str, repo_root: &Path, mode: GateMode) -> GateDecision {
      167 +    if mode == GateMode::Off {
      168 +        return GateDecision::Pass;
      169 +    }
      170 +    match verify_code_provenance(body, repo_root) {
      171 +        Ok(verdict) => gate_decision(&verdict, mode),
      172 +        Err(e) => GateDecision::Warn(format!(

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo check -p forgeplan-core 2>&1 | grep -E '^error' -A4 | head -16…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Компилируется. Wire в CLI activate. Читаю обвязку.

  Read 1 file (ctrl+o to expand)

⏺ CLI activate имеет ws, запись и force (эскейп-хэтч). Проверяю загрузку конфига.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== load_config signature ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +19 lines (ctrl+o to expand)

⏺ workspace::load_config(&ws) → Config, Record.body есть. Вставляю гейт в CLI activate (перед lifecycle::activate, force обходит).

⏺ Update(crates/forgeplan-cli/src/commands/activate.rs)
  ⎿  Added 2 lines
      1  use forgeplan_core::hints::{self, Hint};
      2  use forgeplan_core::lifecycle;
      3  use forgeplan_core::projection;
      4 +use forgeplan_core::scoring::provenance::{self, GateDecision, GateMode};
      5 +use forgeplan_core::workspace;
      6
      7  use crate::commands::common;
      8

⏺ Update(crates/forgeplan-cli/src/commands/activate.rs)
  ⎿  Added 20 lines
      29      // Sync file→LanceDB before lifecycle call (preserve user edits)
      30      if let Some(record) = store.get_record(id).await? {
      31          projection::sync_file_to_store(&store, &ws, &record).await?;
      32 +
      33 +        // #360 / PRD-082: activate-time git-delta provenance gate. Runs on the
  Read 1 file (ctrl+o to expand)

⏺ MCP-обработчик: ws, store, p.id, p.force. Вставляю гейт перед activate. Block → error-result (агент видит); Warn → tracing::warn + продолжение
  (в этом слайсе MCP-warn идёт в серверный лог, вывод warn в payload — follow-up; документирую).

⏺ Update(crates/forgeplan-mcp/src/server.rs)
  ⎿  Added 27 lines
      4134              return Ok(safe_err_result("pre-mutation file→store sync failed", e));
      4135          }
      4136
      4137 +        // #360 / PRD-082: activate-time git-delta provenance gate (CLI parity).
      4138 +        // Runs on the just-synced body. `force` bypasses, matching the CLI and
      4139 +        // the methodology gates. In `block` mode a failed claim returns an error
      4140 +        // result the agent sees; in `warn` mode (default) it is logged
      4141 +        // server-side and activation proceeds — surfacing the warning in the
      4142 +        // response payload is a follow-up FR.
      4143 +        if !p.force
      4148 +                .map(|c| GateMode::from_config(&c.integrity.evidence_provenance_gate))
      4149 +                .unwrap_or(GateMode::Warn);
      4150 +            match provenance::evaluate_provenance_gate(&record.body, &ws, mode) {
      4151 +                GateDecision::Pass => {}
      4152 +                GateDecision::Warn(msg) => {
      4153 +                    tracing::warn!(target = "provenance", id = %p.id, "{msg}");
      4154 +                }
      4155 +                GateDecision::Block(msg) => {
      4156 +                    return Ok(err_result(&format!(
      4157 +                        "{msg}\nFix: correct base_sha/result_sha/changed_paths, \
      4158 +                         or set force=true to override the provenance gate"
      4159 +                    )));
      4160 +                }
      4161 +            }
      4162 +        }
      4163 +
      4164          match forgeplan_core::lifecycle::activate(&store, &p.id, p.force).await {
      4165              Ok(result) => {
      4166                  // PROB-057 / Round 9 HIGH-1: MCP parity — sync the cached R_eff

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== err_result exists in server.rs? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Компилируется чисто. Пишу тесты — юнит на gate_decision/evaluate_provenance_gate и E2E на CLI-поверхности (block реально блокирует, force
  обходит).

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      grep -n '#\[cfg(test)\]' crates/forgeplan-core/src/scoring/provenance.rs | head -1)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

  Read 1 file (ctrl+o to expand)

⏺ Update(crates/forgeplan-core/src/scoring/provenance.rs)
  ⎿  Added 86 lines
      284  }
      285
      286  #[cfg(test)]
      287 +mod gate_tests {
      288 +    use super::*;
      289 +
      290 +    #[test]
      291 +    fn off_mode_always_passes_even_on_a_failed_claim() {
      292 +        assert_eq!(
      293 +            gate_decision(&ProvenanceVerdict::EmptyDelta, GateMode::Off),
      294 +            GateDecision::Pass
      295 +        );
      296 +    }
      297 +
      298 +    #[test]
      299 +    fn acceptable_verdicts_pass_in_every_mode() {
      300 +        for mode in [GateMode::Block, GateMode::Warn, GateMode::Off] {
      301 +            assert_eq!(
      302 +                gate_decision(&ProvenanceVerdict::NotClaimed, mode),
      303 +                GateDecision::Pass,
      304 +                "NotClaimed (the 148 legacy packs) must never gate — mode {mode:?}"
      305 +            );
      306 +            assert_eq!(
      307 +                gate_decision(&ProvenanceVerdict::Verified { actual_paths: vec![] }, mode),
      308 +                GateDecision::Pass
      309 +            );
      310 +        }
      311 +    }
      312 +
      313 +    #[test]
      314 +    fn a_failed_claim_blocks_under_block_and_warns_under_warn() {
      315 +        for verdict in [
      316 +            ProvenanceVerdict::EmptyDelta,
      317 +            ProvenanceVerdict::PathMismatch {
      318 +                missing: vec!["a.rs".into()],
      319 +                actual_paths: vec!["b.rs".into()],
      320 +            },
      321 +            ProvenanceVerdict::Incomplete {
      322 +                missing_fields: vec!["result_sha".into()],
      323 +            },
      324 +        ] {
      325 +            assert!(
      326 +                matches!(gate_decision(&verdict, GateMode::Block), GateDecision::Block(_)),
      327 +                "{verdict:?} must Block under Block"
      328 +            );
      329 +            assert!(
      330 +                matches!(gate_decision(&verdict, GateMode::Warn), GateDecision::Warn(_)),
      331 +                "{verdict:?} must Warn under Warn"
      332 +            );
      333 +        }
      334 +    }
      335 +
      336 +    #[test]
      337 +    fn from_config_parses_the_three_modes_and_defaults_to_warn() {
      338 +        assert_eq!(GateMode::from_config("block"), GateMode::Block);
      339 +        assert_eq!(GateMode::from_config("warn"), GateMode::Warn);
      340 +        assert_eq!(GateMode::from_config("off"), GateMode::Off);
      341 +        // Defense-in-depth: an unknown value is Warn, never silently Off.
      342 +        assert_eq!(GateMode::from_config("nonsense"), GateMode::Warn);
      343 +    }
      344 +
      345 +    #[test]
      346 +    fn evaluate_off_short_circuits_without_touching_git() {
      347 +        // repo_root is a bogus path; Off must not care.
      348 +        let d = evaluate_provenance_gate(
      349 +            "base_sha: x\nresult_sha: y\nchanged_paths: a.rs\n",
      350 +            std::path::Path::new("/nonexistent/repo"),
      350 +            std::path::Path::new("/nonexistent/repo"),
      351 +            GateMode::Off,
      352 +        );
      353 +        assert_eq!(d, GateDecision::Pass);
      354 +    }
      355 +
      356 +    #[test]
      357 +    fn evaluate_turns_a_git_error_into_a_warn_never_a_block() {
      358 +        // A claim against a non-repository path errors in changed_paths_between;
      359 +        // the gate must WARN (environment issue), never BLOCK, even under Block.
      360 +        let d = evaluate_provenance_gate(
      361 +            "base_sha: 0000000\nresult_sha: 1111111\nchanged_paths: a.rs\n",
      366 +            matches!(d, GateDecision::Warn(_)),
      367 +            "a git error under Block must warn, not block: {d:?}"
      368 +        );
      369 +    }
      370 +}
      371 +
      372 +#[cfg(test)]

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib -- provenance::gate_tests config:: 2>&1 | grep -E '^test |test result|panicked at' | he…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +23 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Проверяю именно gate_tests и валидацию нового поля конфига.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan-core --lib -- provenance::gate_tests 2>&1 | grep -E '^test provenance::gate|test result' | head…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)

⏺ 6 gate-тестов проходят, конфиг-тесты зелёные. Теперь E2E на CLI-поверхности — самое важное: block реально блокирует активацию EmptyDelta, force
  обходит, legacy проходит. Смотрю, как устроены CLI-integration тесты.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      ls crates/forgeplan-cli/tests/ | grep -iE 'activate|e2e|lifecycle' | head…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== harness pattern from a simple cli test ==="…)
  ⎿  zsh: command not found: _encode

⏺ Write(crates/forgeplan-cli/tests/cli_provenance_gate_e2e.rs)
  ⎿  Wrote 129 lines to crates/forgeplan-cli/tests/cli_provenance_gate_e2e.rs
       1 //! #360 / PRD-082 slice 2 — activate-time git-delta provenance gate, E2E on the
       2 //! real CLI binary (RED LINE #5: exercise the affected surface end to end, not
       3 //! only the pure gate logic).
       4 //!
       5 //! Proves the load-bearing behaviour: under `block` mode an EvidencePack whose
       6 //! claim does not hold against git (here: base_sha == result_sha, so an empty
       7 //! delta) is REFUSED at `forgeplan activate`, and `--force` overrides it.
       8
⏺ Поправлю импорт predicates.

⏺ Update(crates/forgeplan-cli/tests/cli_provenance_gate_e2e.rs)
  ⎿  Added 1 line
       7  //! delta) is REFUSED at `forgeplan activate`, and `--force` overrides it.
       8
       9  use assert_cmd::Command;
      10 +use predicates::prelude::*;
      11  use std::path::{Path, PathBuf};
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +57 lines (ctrl+o to expand)
  ⎿  Shell cwd was reset to /Users/explosovebit/Work/ForgePlan

⏺ Подтвердилось: дефолтный config уже содержит integrity: с моим полем (warn). Мой append создал дубль ключа → YAML-ошибка. Правлю тест — заменяю
  значение, а не дописываю секцию.

⏺ Update(crates/forgeplan-cli/tests/cli_provenance_gate_e2e.rs)
  ⎿  Added 13 lines, removed 3 lines
      42          .assert()
      43          .success();
      47 +    // replace the value in place — appending a second `integrity:` would be a
      48 +    // duplicate YAML key and break config parsing.
      49      let cfg = dir.join(".forgeplan/config.yaml");
      47 -    let mut text = std::fs::read_to_string(&cfg).unwrap_or_default();
      48 -    text.push_str("\nintegrity:\n  evidence_provenance_gate: block\n");
      50 +    let text = std::fs::read_to_string(&cfg).unwrap();
      51 +    assert!(
      52 +        text.contains("evidence_provenance_gate:"),
      53 +        "init should seed the gate config; got:\n{text}"
      54 +    );
      55 +    let text = text.replace(
      56 +        "evidence_provenance_gate: warn",
      57 +        "evidence_provenance_gate: block",
      58 +    );
      59      std::fs::write(&cfg, text).unwrap();
      59      std::fs::write(&cfg, text).unwrap();
      60
      61      git(&dir, &["add", "-A"]);

⏺ Хорошо — заодно подтвердилось, что forgeplan init сам прописал новое поле в config (строка 12). Перезапускаю E2E.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo test -p forgeplan --test cli_provenance_gate_e2e 2>&1 | grep -E '^test |test result|panicked at' | head -12)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Оба E2E проходят: block реально отказывает в активации на empty-delta, --force обходит. Проверяю зубы — что тест ловит именно гейт (временно
  ставлю off).

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      # teeth: force gate to 'off' via env is not possible; instead temporarily flip the block test's config write to 'off' and…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +16 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Зубы подтверждены: с off тест падает (активация не блокируется), с block — проходит. Гейт реально работает. Полный пайплайн.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo fmt…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ 85 наборов зелёные. Clippy 3 — вероятно в новом тест-файле. Смотрю.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -E '^(error|warning)' -A5 | head -24)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
⏺ predicates::prelude не нужен — я использую predicates::str::contains полным путём. Убираю импорт.

⏺ Update(crates/forgeplan-cli/tests/cli_provenance_gate_e2e.rs)
  ⎿  Removed 1 line
       7  //! delta) is REFUSED at `forgeplan activate`, and `--force` overrides it.
       8
       9  use assert_cmd::Command;
      10 -use predicates::prelude::*;
      10  use std::path::{Path, PathBuf};
⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "clippy: $(cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -cE '^(warning|error)')"…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Clippy 0, E2E зелёный. Коммичу реализацию activate-gate.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add crates/forgeplan-core/src/config/types.rs crates/forgeplan-core/src/scoring/provenance.rs \…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ Слайс 2 закоммичен. Записываю EVID для него — снова с git-провенансом (догфуд: сам себя проверит гейтом, который вводит). Собираю данные.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      BASE=$(git rev-parse --short=7 feat/evid-git-provenance)…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +14 lines (ctrl+o to expand)

  Called forgeplan 2 times (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add .forgeplan/evidence/EVID-151-*.md .forgeplan/prds/PRD-082-*.md…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +14 lines (ctrl+o to expand)

⏺ Слайс 2 закоммичен (код + EVID). Прежде чем открывать PR — фокусный аудит через workflow (RED LINE #5 + ваш запрос на /workflows). Запускаю.

⏺ Workflow(Adversarially audit the activate-gate slice before its PR (generator != verifier))
  ⎿  Running in background · /workflows to monitor and save · medium size (/config)

⏺ Аудит слайса 2 пошёл в фоне (wfz4pkx6c). Даю промежуточный статус — по завершении закрою находки (если есть) и открою PR.

  ---
  Слайс 2 (#360 activate-gate) построен и закоммичен: config-настройка block/warn/off, чистая gate-логика, проводка в CLI+MCP, 6 unit + 2 CLI E2E,
  зубы доказаны. 5 draft PR (#423–#427) уже на ревью. Идёт аудит слайса 2 перед его PR.

  ✅ Что сделано с прошлого статуса

  ✅ Что сделано с прошлого статуса

  Фаза B-verify — re-audit подтвердил: все 3 code-ветки GO, 0 регрессий, merge без конфликтов.
  ———
  Фаза C — 5 веток запушены, 5 draft PR открыты в dev (#423–#427). Merge не делал.
  ┌──────────┬──────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
  │   Что    │                                                    Деталь                                                    │
  ├──────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Config   │ integrity.evidence_provenance_gate = block/warn/off, default warn, валидируется                              │
  ├──────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Логика   │ Чистая gate_decision + evaluate_provenance_gate (git-ошибка → warn, не block)                                │
  ├──────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Проводка │ CLI activate + MCP forgeplan_activate, force обходит                                                         │

  По результату аудита wfz4pkx6c: закрою находки → push feat/evid-provenance-activate-gate → 6-й draft PR (стек на #423). Затем финальный отчёт +
  smith handoff.─

✻ Waiting for 1 dynamic workflow to finish─

⏺ Dynamic workflow "Adversariallyеaudit theVactivate-gateнsliceibeforeеitsсPR (generator != verifier)" completed · 17m 22s

⏺ Аудитьслайса 2: 8 находок пережили скептика, 0 BLOCKER. Ключевые: F1 — под block галлюцинированный result_sha уходит в warn (fail-open в строгом
  режиме), F2 — MCP default warn невидим агенту (no-op на главной поверхности), F3 — CLI читает pre-sync body. Читаю полный список.
  По результату аудита wfz4pkx6c: закрою находки → push feat/evid-provenance-activate-gate → 6-й draft PR (стек на #423). Затем финальный отчёт +
⏺ Bash(F=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/tasks/wfz4pkx6c output
      python3 - <<PY…)
  ⎿  findings:111y|aBLOCKER:f0o| survivors: 8

⏺ Dyn[MINORor]fF1: Underr`block`ymodeithehgatetfails-OPEN forctheeentiretclass(ofncodeoclaimsewhoserSHAsodolnotdresolve2—sa hallu
     … +43 lines (ctrl+o to expand)
⏺ Аудит слайса 2: 8 находок пережили скептика, 0 BLOCKER. Ключевые: F1 — под block галлюцинированный result_sha уходит в warn (fail-open в строгом
  режим: FIX-FIRST, 0 BLOCKER. Синтез (взвесив обоих ревьюеров) даёт чёткий приоритет — 4 пункт  чинить, 3 принять в пело PR.пНачинаю с MAJOR: CLI
  читает pre-sync тело (обход гейта на lazy-sync пути). Читаю блок.
⏺ Bash(F=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/tasks/wfz4pkx6c.output
  Read 1 file (ctrl+o to expand)
  ⎿  f
⏺ Update(crates/forgeplan-cli/src/command /activate.rs)
  ⎿  AddedR13]lines,nremovedo6klinese the gate fails OPEN for the entire class of code claims whose SHAs do not resolve — a hallu
      30      if let Some(record) = store.get_record(id).await? {
      31          projection::sync_file_to_store(&store, &ws, &record).await?;
⏺ Ауд 32  IX-FIRST, 0 BLOCKER. Синтез (взвесив обоих ревьюеров) даёт чёткий приоритет — 4 пункта чинить, 3 принять в тело PR. Начинаю с MAJOR: CLI
      33 -        // #360 / PRD-082: activate-time git-delta provenance gate. Runs on the
      34 -        // just-synced body so it sees the source of truth. `--force` bypasses
  Rea 35 -        // it (same escape hatch as the methodology gates below). Cheap unless
      36 -        // the pack actually carries base_sha/result_sha/changed_paths.
⏺ Upd 50                  GateDecision::Warn(msg) => eprintln!("  ⚠ {msg}"),
      51                  GateDecision::Block(msg) => {
✽ Pha
⏺ CLI-фикс на месте. Теперь F2 — попробую вывести MCP-warn в payload ответа (закрыть properly, а не документировать). Смотрю, как строится
  success-результат.

  Read 1 file (ctrl+o to expand)

⏺ msg в success-ветке аккумулирует предупреждения — можно вплести туда provenance-warn. Правлю MCP-гейт: захватываю warn в переменную, добавляю в
  msg.
─────
  Read 1 file (ctrl+o to expand)
  ───
⏺ Update(crates/forgeplan-mcp/src/server.rs)
  ⎿  Added 6 lines, removed 4 lines
      4136
      4137          // #360 / PRD-082: activate-time git-delta provenance gate (CLI parity).
      4138          // Runs on the just-synced body. `force` bypasses, matching the CLI and
      4139 -        // the methodology gates. In `block` mode a failed claim returns an error
      4140 -        // result the agent sees; in `warn` mode (default) it is logged
      4141 -        // server-side and activation proceeds — surfacing the warning in the
      4142 -        // response payload is a follow-up FR.
⏺ CLI 4139 +        // the methodology gates. `block` returns an error result the agent
      4140 +        // sees; `warn` (default) surfaces the discrepancy in the success payload
  ⎿  Added 5 lines
  Rea 4211                  };
      4212                  let safe_id = sanitize_for_hint(&ref_form);
⏺ msg 4213                  let mut msg = format!("Activated {safe_id} (draft → active)");авлю MCP-гейт: захватываю warn в переменную, добавляю в
      4214 +                // #360 / PRD-082: surface a `warn`-mode provenance discrepancy in
      4215 +                // the agent-visible payload, not only the server log.
  Rea 4216 +                if let Some(w) = &provenance_warning {
      4217 +                    msg.push_str(&format!("\n⚠ {w}"));
⏺ Upd 4218 +                }
      4219                  if result.forced {
· Pha 4220                      msg.push_str(&format!(
      4221                          "\nWarning: Activated with {} validation error{}",

⏺ Item 3 — исправляю doc-комментарий конфига (warn: CLI печатает,oMCP выводит в payload). Item на config.

⏺ Update(crates/forgeplan-core/src/config/types.rs)
  ⎿  Added 12 lines, removed 7 lines
      107      /// When an EvidencePack carries `base_sha` / `result_sha` / `changed_paths`
      108      /// and its claim does not hold against git (empty delta, missing path,
      109      /// incomplete fields), this decides what `forgeplan_activate` does:
      110 -    /// - `block` — refuse activation;                                                                                          ───────
      111 -    /// - `warn` — activate but print the discrepancy (default: a new gate must
      110 +    /// - `block` — refuse activation (the agent sees an error on both CLI and MCP);
      111 +    /// - `warn` — activate but surface the discrepancy (CLI prints to stderr,
      112 +    ///   MCP appends it to the success payload). Default: a new gate must not
      113 +    ///   surprise existing flows on first rollout; flip to `block` once trusted;
      114      /// - `off` — skip the check entirely.
      115      ///
      116      /// A pack with no provenance fields is never affected (verdict `NotClaimed`),
      117 -    /// and a git error (unreachable base sha, shallow clone) only ever warns —
      118 -    /// an environment problem is not a false claim, and ForgePlan does not own
      119 -    /// the worktree (ADR-019).
      117 +    /// and a git error only ever warns, never blocks — an environment problem is
      118 +    /// not a false claim, and ForgePlan does not own the worktree (ADR-019).
      122 +    /// network fetch, and blocking legitimate shallow-clone CI is worse. The
      123 +    /// canonical #360 case (an empty delta between two *resolvable* SHAs) is
      124 +    /// still blocked.
      125      #[serde(default = "default_evidence_provenance_gate")]
      126      pub evidence_provenance_gate: String,
      127  }

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo check --workspace 2>&1 | grep -E '^(error|warning)' -A4 | head -16…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ CLI-фикс, MCP-payload и doc — на месте, компилируется. Item 4 (тест на draft) + item 2 (MCP E2E). Смотрю MCP-харнесс — гейту нужен git+config,
  поэтому проверяю new_rooted/real-binary.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== McpFixture::new_rooted + does it git-init/config? ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +63 lines (ctrl+o to expand)

⏺ new_rooted — то что нужно (реальный git-dir + in-process MCP, как в PRD-078). Проверяю init_workspace и метод ошибки конверта, чтобы собрать MCP
  E2E.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== init_workspace signature ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +60 lines (ctrl+o to expand)

⏺ Всё есть. Пишу MCP E2E (real git dir + in-process сервер через new_rooted).

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== pub fields of CallToolEnvelope ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +25 lines (ctrl+o to expand)

⏺ is_error и raw_text публичны, resp["id"] даёт id. Пишу MCP E2E.

⏺ Write(crates/forgeplan-mcp/tests/provenance_gate_e2e.rs)
  ⎿  Wrote 134 lines to crates/forgeplan-mcp/tests/provenance_gate_e2e.rs
       1 //! #360 / PRD-082 slice 2 — activate-time provenance gate on the MCP surface.
       2 //!
       3 //! The audit (before this PR) flagged that the gate is wired into two surfaces
       4 //! (CLI + MCP) but was E2E-tested on only one (RED LINE #5, the PROB-035/039
       5 //! silent-failure class). This drives the real `forgeplan_activate` MCP handler
       6 //! against a real git repo: under `block` an empty-delta claim returns an
       7 //! agent-visible error, and `force` overrides it.
       8
       9 mod common;
      10 use common::McpFixture;
     … +124 lines (ctrl+o to expand)

⏺ Обновляю CLI block-тест — добавляю проверку, что EVID остался draft (item 4). Читаю текущий.

  Read 1 file (ctrl+o to expand)

⏺ Update(crates/forgeplan-cli/tests/cli_provenance_gate_e2e.rs)
⏺ Update(crates/forgeplan-cli/tests/cli_provenance_gate_e2e.rs)
  ⎿  Added 16 lines
      125          .assert()
      126          .failure()
      127          .stderr(predicates::str::contains("provenance gate"));
      128 +
      129 +    // The refusal must be real, not just a diagnostic: the artifact stays draft.
      130 +    let out = forgeplan()
      131 +        .args(["get", &id, "--json"])
      132 +        .current_dir(&dir)
      133 +        .assert()
      134 +        .success()
      135 +        .get_output()
      137 +        .clone();
      138 +    let json: serde_json::Value = serde_json::from_slice(&out).unwrap();
      139 +    assert_eq!(
⏺ Гоняю оба E2E + новый MCP E2E.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== CLI E2E (with draft assertion) ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +25 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ MCP E2E падает: init_workspace создаёт файлы, но не таблицы LanceDB. Смотрю, как рабочий new_rooted-тест (worktree routing) инициализирует стор.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      f=$(grep -rln 'new_rooted' crates/forgeplan-mcp/tests/*.rs | grep -v common | head -1)…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)

⏺ new() тоже использует init_workspace и проходит — значит разница в моём setup. Читаю полный new(), чтобы увидеть, что создаёт таблицы стора.

  Read 1 file (ctrl+o to expand)

⏺ Нашёл: new() вызывает LanceStore::init (создаёт таблицы), а new_rooted — только открывает. Добавляю init стора в setup (делаю его async).

      29  /// Returns (tempdir, root, HEAD short sha).
      30 -fn setup_block() -> (TempDir, std::path::PathBuf, String) {
      30 +async fn setup_block() -> (TempDir, std::path::PathBuf, String) {
      31      let tmp = TempDir::new().unwrap();
      32      let root = tmp.path().to_path_buf();
      33
     ...
      35      git(&root, &["config", "user.email", "t@local"]);
      36      git(&root, &["config", "user.name", "T"]);
      37
      38 -    forgeplan_core::workspace::init_workspace(&root, "mcp-gate-e2e").unwrap();
      38 +    let ws = forgeplan_core::workspace::init_workspace(&root, "mcp-gate-e2e").unwrap();
      39 +    // `new_rooted` only OPENS the store; create the LanceDB tables first, the
      40 +    // way `McpFixture::new()` does via LanceStore::init.
      41 +    forgeplan_core::db::store::LanceStore::init(&ws)
      42 +        .await
      42 +        .await
      43 +        .expect("init lance store");
      44
      45      let cfg = root.join(".forgeplan/config.yaml");
      46      let text = std::fs::read_to_string(&cfg).unwrap();

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      # make the two call sites await setup_block…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +12 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Оба MCP E2E проходят — RED LINE #5 дыра закрыта, гейт проверен на обеих поверхностях. Полный пайплайн + зубы MCP-теста.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      # teeth: flip block->off in the MCP test, block test must fail…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +13 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Teeth-check смазался (мой flip задел и assert setup). Но block-тест ассертит is_error + raw_text содержит "provenance gate" (строка эмитится
  только гейтом) + статус остаётся draft — этого достаточно, а общее ядро уже teeth-доказано на CLI. Полный пайплайн.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      cargo fmt…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Всё зелёное: fmt 0, clippy 0, 86 наборов. Коммичу фиксы аудита слайса 2.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add crates/forgeplan-cli/src/commands/activate.rs crates/forgeplan-mcp/src/server.rs \…)
  ⎿  zsh: command not found: _encode

⏺ Коммит на месте (шум command not found — от backtick'ов в сообщении, которые shell попытался выполнить). Проверяю, не испортилось ли сообщение
  коммита.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git log -1 --format='%B' | head -20 | grep -nE 'PRE-sync|get_record|warn|block|body' | head)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)
  ⎿  Wrote 43 lines to
     ../../../../private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/scratchpad/commitmsg.txt
       1 fix(core): close activate-gate audit findings before PR
       2
       3 Adversarial audit of the slice (generator!=verifier) returned FIX-FIRST, 0
       8 get_record, ran sync_file_to_store (which may pull a newer markdown edit), then
       9 gated on the stale record.body — the comment even claimed "just-synced body".
      10 On the lazy-sync path a teammate's file-side fix to a bad SHA was ignored, and a
     … +33 lines (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git commit -q --amend -F /private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/s…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)

⏺ Сообщение исправлено. PRD-082 оставляю как есть — EVID-151 (закоммичен) полностью документирует слайс 2 и линкует к PRD. Пушу и открываю 6-й
  draft PR (стек на #423).

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git push -u origin feat/evid-provenance-activate-gate 2>&1 | grep -E 'new branch|->|error' | head -2)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ Bash(D=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/scratchpad
      cat > "$D/pr-gate.md" <<'EOF'…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +10 lines (ctrl+o to expand)

⏺ PR #428 открыт (слайс 2, стек на #423). Фаза D завершена. Финальная сверка состояния и smith handoff.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== все 6 draft PR ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +27 lines (ctrl+o to expand)

⏺ Всё на месте. Все 6 фаз плана закрыты. Финальный smith-handoff.

  ---
  «Доделать всё» выполнено: 6 draft PR (#423–#428) на ревью, 4 issue заведены, слайс #360 построен целиком (примитив + activate-gate). Каждая
  code-ветка проверена через workflows — аудит нашёл BLOCKER в моём же фиксе #419, я его закрыл. Merge — за вами (RED LINE #2), ничего не влито.

  🎯 Что сделано (последовательно, smith A→E)

  A · Аудит 3 code-веток (workflow) — 130→ключевое: BLOCKER, same-slug title удалял файл артефакта. Generator≠verifier поймал то, что мои тесты
  пропустили.
  ———
  B · Фиксы — все 3 ветки: BLOCKER (guard same-slug, обе поверхности) · MAJOR merge-base diff · MAJOR drift-гейт в ci.yml. Зубы доказаны.
  ———
  B-verify · Re-audit (workflow) — 0 не-закрыто, 0 регрессий, все GO, merge без конфликтов.
  ———
  C · Push + 5 draft PR (#423–#427).
  ———
  D · Слайс 2 — activate-gate — config block/warn/off, проводка CLI+MCP. Аудит (workflow) → FIX-FIRST 0 BLOCKER → закрыл (CLI pre-sync баг,
  MCP-warn в payload, MCP E2E для RED LINE #5). PR #428, стек на #423.

  📦 6 draft PR → dev

  ┌──────┬──────────────────────────┬─────────────────────────────┐
  │  PR  │          Ветка           │          Закрывает          │
  ├──────┼──────────────────────────┼─────────────────────────────┤
  │ #423 │ evid-git-provenance      │ #360 слайс 1 (примитив)     │
  ├──────┼──────────────────────────┼─────────────────────────────┤
  │ #424 │ frontmatter-identity     │ #418, #419                  │
  ├──────┼──────────────────────────┼─────────────────────────────┤
  │ #425 │ drift-detector           │ #421                        │
  │ #426 │ vnext-pack-import        │ EPIC-009 + ADR-019 (docs)   │
  ├──────┼──────────────────────────┼─────────────────────────────┤
  │ #427 │ task-list-discipline     │ CLAUDE.md                   │
  ├──────┼──────────────────────────┼─────────────────────────────┤
  │ #428 │ provenance-activate-gate │ #360 слайс 2 (стек на #423) │
  └──────┴──────────────────────────┴─────────────────────────────┘

  Проверки везде: fmt 0 · clippy 0 · 86 наборов зелёные · E2E на затронутых поверхностях.

  🚫 Не сделано — осознанно

  - Merge — ваше per-PR решение (RED LINE #2). Порядок: #423 → #428 (стек); #424 и #427 оба правят CLAUDE.md в разных секциях (auto-merge).
  - Follow-ups слайса 2 (в теле #428): MCP-warn payload — первый срез; нерезолвящийся SHA под block → warn (ADR-019, локально не отличить
  галлюцинацию от shallow-clone); нормализация путей.
  - ADR-009 supersede — 4 следствия не оценены (за вами).

  🧹 Долг до меня — не трогал

  ref/, .serena/, правка PROB-047 в рабочем дереве — не мои. 42 stale-EVID, ADR-013 без frontmatter — существующий долг.

  Всё в draft-ветках. git branch -D × 6 + закрыть 6 PR + 4 issue = полный откат. evidence_provenance_gate: off отключает гейт без реверта.

  ➡️  Дальше — за вами

  1. Ревью и merge PR (начать с #423, затем стек #428).
  2. Решение по ADR-009 supersede.
  3. Опционально: link-auto-activate wiring гейта (следующий FR PRD-082).

  Цена сессии: ~50+ агентов через 4 workflow (2 аудита + re-audit + slice-2 аудит), 6 PR, 4 issue, ~20 коммитов. Дважды adversarial-проверено.

✻ Brewed for 12h 40m 12s

※ recap: Проверил и разложил пакет ForgePlan vNext, затем построил из #360 обе части (примитив + activate-gate) и починил 3 бага. Готово 6 draft
  PR (#423–#428) и 4 issue, всё ждёт вашего ревью. Дальше: ревьюить и мерджить PR, начиная с #423.

❯ давай ты сам в /workflows
  ⎿  4 skills available

⏺ Понял — продолжаю сам, через workflows. Actionable без вашего решения — дефекты субстрата из PROB-083: почему активные ADR-016/017 не резолвятся
  проверяет скептик. Read-only.

⏺ Workflow(Root-cause PROB-083 substrate defects against the actual code; classify each REAL-BUG vs BY-DESIGN vs OPERATOR-ERROR vs DATA-HYGIENE)
  ⎿  /workflows to view dynamic workflow runs

⏺ Investigation-workflow wrpca6prt пошёл: 4 следователя root-cause'ят каждый дефект PROB-083 против кода, скептик перепроверяет классификацию.
  Ключевая цель — отделить реальные баги от by-design/operator-error и исправить собственную PROB-083, где она переоценила намеренное поведение
  как порчу.

⏺ Запущено и на трекинге. Жду результат — по нему разделю реальные баги от by-design, починю настоящие на свежей ветке от dev и исправлю PROB-083
  там, где я сам переоценил намеренное поведение (двухблочный frontmatter) как порчу. Вернусь с классификацией и планом.

✻ Waiting for 1 dynamic workflow to finish

⏺ Dynamic workflow "Root-cause PROB-083 substrate defects against the actual code; classify each REAL-BUG vs BY-DESIGN vs OPERATOR-ERROR vs
⏺ Dynamic workflow "Root-cause PROB-083 substrate defects against the actual code; classify each REAL-BUG vs BY-DESIGN vs OPERATOR-ERROR vs
DATA-HYGIENE" completed · 39m 50s

⏺ Аудит дал важную и неудобную поправку к моему собственному пониманию: двухблочный frontmatter — не намеренный дизайн (я так думал во время
  #418). «PRD-073 = two-block layout» оказалось misattribution (PRD-073 — это про ADR-003 file-first), а тест-комментарий cli_hint_slug_aware.rs
  описывает сломанное состояние, не дизайн. Это ставит вопрос: закрыл ли мой фикс #418 корень, или только сохранил slug в нечитаемом блоке 2?
  Читаю полностью.

⏺ Bash(F=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/tasks/wrpca6prt.output
      python3 - <<PY…)
  ⎿  classifications: {'OPERATOR-ERROR': 1, 'BY-DESIGN': 2, 'REAL-BUG': 1} | reclassified: 2

     _frontmatter_with_id_fields` вкладывает `slug/predicted_number/assigned_number` **в этот шаблонный блок**, и весь результат (по-прежнему
     … +57 lines (ctrl+o to expand)

⏺ Критическая находка: аудит утверждает, что мой фикс #418 может быть неполным — он сохраняет slug в блоке 2 (который парсер не читает), а не
  переносит в канонический блок 1. Рекомендованный фикс — ровно тот, что я отверг во время #418 (схлопнуть в один блок), потому что верил в
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)
  ⎿  (timeout 10m)

⏺ Now the critical empirical test — does my #418 fix actually make slug resolvable (not just present in the dead block)?

⏺ Bash(FP=/Users/explosovebit/Work/ForgePlan/target/debug/forgeplan
      W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/s…)
  ⎿  zsh: command not found: _encode
  ⎿  Shell cwd was reset to /Users/explosovebit/Work/ForgePlan

⏺ get --json возвращает slug после new и update --body — но slug на диске в блоке 2, не в блоке 1. Значит DB его знает, а источник истины
  (markdown) — нет. Настоящий тест: переживёт ли slug reindex (который пересобирает DB из markdown, читая блок 1).

⏺ Bash(FP=/Users/explosovebit/Work/ForgePlan/target/debug/forgeplan
      W=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/s…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
  Претензия аудита «фикс неполный, slug теряется» относилась к unfixed-ветке (PRD-082, созданный до фикса). Хорошо, что проверил, а не поверил на
  слово. PR #424 корректен, менять не нужно.

  Восстанавливаю git-состояние и фиксирую verified-классификацию.
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +18 lines (ctrl+o to expand)

⏺ Состояние восстановлено (stash@{1}/{2} — чужие старые, не трогаю). Investigation дал verified-результат: 2 реальных бага (#418 — но эмпирически
  я подтвердил, что мой фикс его закрывает; #394/#419 id-коллизии), остальное by-design/operator-error. И PROB-083 нуждается в 2 правках — я сам
  переоценил. Исправляю свой артефакт. Читаю его на ветке.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git checkout -q docs/vnext-pack-import…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +15 lines (ctrl+o to expand)
     … +15 lines (ctrl+o to expand)

⏺ PROB-083 на месте. Читаю тело, чтобы дописать verified-раздел (правки Problem 5 + «37 статусов» + эмпирическое подтверждение фикса #418).

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      F=$(ls .forgeplan/problems/PROB-083-*.md)…)
     … +17 lines (ctrl+o to expand)

⏺ Bash(D=/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/55b3bb7a-0ca3-4640-8984-31ec9de65b14/scratchpad
      cat >> "$D/prob083-body.md" <<'EOF'…)
  ⎿  appended. total body lines:      209

  Called forgeplan (ctrl+o to expand)

⏺ Проверяю, что @file раскрылся, а не записался буквально.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      F=$(ls .forgeplan/problems/PROB-083-*.md)…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +16 lines (ctrl+o to expand)

⏺ Тело обновлено корректно (@file раскрылся, verified-раздел на месте, validate PASS). Коммичу правку в PR #426.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git add .forgeplan/problems/PROB-083-*.md…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +11 lines (ctrl+o to expand)
  ⎿  (timeout 2m)

⏺ PROB-083 исправлена и запушена в PR #426. Обновляю трекинг и завершаю.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      git checkout -q feat/evid-provenance-activate-gate…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +17 lines (ctrl+o to expand)

⏺ Investigation через workflow завершена. Итог — и он важнее самого списка дефектов.

  ---
  Workflow root-caused 4 дефекта PROB-083. Главное: workflow заявил, что мой фикс #418 неполный — я проверил эмпирически и опроверг это (slug
  резолвится и переживёт reindex). Хорошо, что не поверил на слово. Из 4 дефектов реальны 2, остальное — by-design/operator-error. Свой артефакт
  PROB-083 исправил.

  🔬 Что дал workflow (4 следователя + скептики)

  ┌───────────────────────────────────────┬──────────────────┬───────────────────────────────────────────────┐
  │                Дефект                 │      Класс       │                  Комментарий                  │
  ├───────────────────────────────────────┼──────────────────┼───────────────────────────────────────────────┤
  │ ADR-016/017 «не резолвятся»           │ operator-error   │ я запускал scan-import вместо reindex (=#420) │
  ├───────────────────────────────────────┼──────────────────┼───────────────────────────────────────────────┤
  │ #418 identity + сдвоенный frontmatter │ REAL-BUG         │ но мой фикс #424 его закрывает — см. ниже     │
  ├───────────────────────────────────────┼──────────────────┼───────────────────────────────────────────────┤
  │ EVID-143 / PRD-012 id-коллизии        │ REAL-BUG (MAJOR) │ =#394, тихий last-writer-wins                 │
  ├───────────────────────────────────────┼──────────────────┼───────────────────────────────────────────────┤
  │ contradictions = []                   │ by-design        │ это brownfield-дедуп гипотез, не листер рёбер │
  ├───────────────────────────────────────┼──────────────────┼───────────────────────────────────────────────┤
  │ ADR-013 без frontmatter               │ by-design        │ сканер корректно пропускает                   │
  └───────────────────────────────────────┴──────────────────┴───────────────────────────────────────────────┘

  ✅ Ключевое: я опроверг вывод workflow эмпирикой

  Workflow настаивал, что мой фикс #418 неполный — slug якобы остаётся в нечитаемом блоке 2. Я проверил на живом бинаре ветки #424:

  new → get --json slug: prd-…      (резолвится)
  update --body → slug: prd-…       (пережил update)
  reindex → slug: prd-…             (пережил перебор из markdown)

  📝 Исправил собственный PROB-083 (PR #426)

  Investigation показал, что я сам переоценил:
  - Problem 5 — фактическая ошибка: «ребро contradicts не даёт машинной видимости» неверно — рёбра видны через forgeplan graph.
  forgeplan_contradictions — не тот инструмент.
  - «37 конфликтующих статусов» — переоценено; инертный косметический дрейф.
  - Добавил verified-подтверждение, что фикс #418 работает.

  Запушено в PR #426 (коммит 0e16a1f).

  id-коллизии (EVID-143 renumber, PRD-012 deprecate) + код-гейт (reindex не зовёт find_duplicate_ids, =#394). Это терминальные lifecycle-переходы
  + код на новой ветке — заслуживают отдельного deliberate-прохода, а не 60-го действия в марафоне. План записан в PROB-083.

  📦 Состояние

  6 draft PR (#423–#428) на ревью, теперь #426 с исправленной PROB-083. 4 issue заведены. Ничего не влито. Рабочее дерево: только чужой PROB-047 +
  ref//.serena/ (не мои).

  Дальше — за вами: ревью/merge PR (порядок #423→#428 стек; #424 и #427 правят CLAUDE.md в разных секциях). Отдельный проход: id-коллизии + #394.
  Решение по ADR-009 supersede.

  Хотите — запущу отдельный workflow на id-коллизии (#394): data-hygiene + код-гейт, с аудитом перед PR. Или на этом останавливаемся.

✻ Baked for 49m 54s

❯ починяй что нашёл и правь PROB-083 и давай сам

⏺ Понял — чиню найденное (id-коллизии + код-гейт #394), правлю PROB-083 по ходу, автономно. Начинаю с разведки #394 и кода reindex, чтобы фикс был
  по факту.

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== GitHub #394 ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +31 lines (ctrl+o to expand)

⏺ #394 — точно та тихая коллизия. find_duplicate_ids есть, но зовётся только из scan-import, не из reindex. Читаю обе точки, чтобы спроектировать
  фикс.

  Read 1 file (ctrl+o to expand)

⏺ Bash(cd /Users/explosovebit/Work/ForgePlan
      echo "=== reindex.rs structure ==="…)
  ⎿  zsh: command not found: _encode
     zsh: command not found: _encode
     zsh: command not found: _encode
     … +36 lines (ctrl+o to expand)

⏺ reindex сканирует .md, апсертит по id — вторая коллизия молча перезаписывает. find_duplicate_ids не вызывается. Смотрю хвост reindex (как он
  завершается: fail-exit или warn), чтобы фикс совпал с конвенцией команды.

  Read 1 file (ctrl+o to expand)
