---
depth: standard
id: EVID-198
kind: evidence
last_modified_at: 2026-10-23T19:23:16.005700+00:00
last_modified_by: documentation-walkthrough/1
links:
- target: RFC-017
  relation: informs
- target: PRD-089
  relation: informs
status: draft
title: Documentation closure production build browser and reference measurements
---

---
assigned_number: 198
created: 2026-10-23
predicted_number: 198
slug: evid-documentation-closure-production-build-browser-and-reference-measurements
updated: 2026-10-23
---

# EVID-198: Documentation closure production/browser/reference measurements

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
review_verdict: CONCERNS

## Measurement

Реальные команды выполнял родительский координатор; независимые статические рецензенты проверили журналы/источники,но НЕ запускали build/runtime заново. Cargo/Rusttestsне выполнялись. Использован готовый0.39.0+constellation.s1.1,binarySHA95ce1953a7d59d92b3806ebca86b8162f0d66362c9a87ee7b9e94a600327e332. Технически успешные наблюдения записаны отдельно от нативного formal gate. Оба постоянных источника и их точныеSHA закрепленыEVID-197.

## Result

- Первыйproductionbuild exit0,566Astroстраниц/608HTML,18.04s. После него completenessпроверка отказала:240CLI+171MCPstale generatedwrapperlabels. Этототказ сохранён,не переписан.
- РеальныеCLI/MCPгенераторы согласовали411страниц. Все410detail authoredобласти иliteralpayload неизменны;другаяизменённаястраница—RUoverviewlabelс прежнимhref. Последующийcompleteness exit0:CLI120/full76/read9×2locales,missing/stale/unmatched0.
- Финальныйproductionbuild exit0,566Astropages/608HTML,17.36s. Vitechunk>500kB иEntrydocs404—наблюдавшиесяпредупреждения. PreviewотдельнонапечаталNodeDEP0205.
- Последнийdeadlinkcheck exit0:124630внутреннихназначенийв608HTML,0битых. Fragments/всенешниеURLэтимнепроверены.
- Driftcheck exit0:236EN/235RU,0missingunexpected/mtimecandidates;generatedENchangelogожидаемоисключён. Mtimeнепровозглашёнсмысловойпроверкой.
- РеальныйproductionChromium:40EN/RUlessonroutes,correctlang/body,одинcanonicalиLearningResourceпоlocale,внутриembeddedbody0scriptтегов. Дваstandaloneindex,по20правильныхsiblinglinks.
- 6интерактивныхуроков×2языка=12реальныхсценариев:поодномуChartнаcanvas,trustPlotly,20динамическихшаговfirst-artifact/reset,depthcosmeticTACTICAL/authCRITICAL(учебнаяшкала),DAG24→18→24видимыхузла. НовыхJSисключенийвэтихсценариях0.
- Menuactualbutton390pxEN/RU:click/Enter/Space/Escape,body/sidebarstate,focusreturn,ENopen→RUreset;1440pxкнопкаскрыта.
- НастоящийextractEmbedприотсутствующемENfileвозвращаетENOENT,неподставляетRU. Все временныеbrowserтабысервисыdev4847/productionpreview4848закрыты.

## Findings

ПослеисправленияbodyScriptduplicationшестьранееошибочныхдиаграммработалиоднократно. На двухstandaloneindexбыли2внешнихCSSrequestfailures ERR_BLOCKED_BY_ORBдляJetBrainsMono;ониявноотделеныотJSисключенийи сохраненывотчёте/#542. Нетзаявления,чтоURLнедоступенвсемклиентам. Старые194проверкинепереименованывновыеtestcounts;codecoverageнеизмеренои неравно0. НетфальшивыхдефектовдляизмерительногоPASS.

## Interpretation

Наблюдения подтверждают перечисленный локальный UI/documentation/runtimeобъём. review_verdictCONCERNS сохраняетнеснятыеG1/G2/G3иprivacyhonest-zeroограничения: это НЕ общийgatePASS. Не доказываетсяRustregressioncoverage,полнаясовместимостьbackend,migratedlayout,performance,productiondeployилиготовностьпилотанашестипроектах. Нативныеправила/памятьнеправились;PRD/RFCнеактивированы.

## Congruence Level Justification

CL3: тотжеснимок/готовыйбинарник/настоящийбраузер,не mock. Исходныйотказadmission,двесборки,controlledregenerationифинальныепроверкипрослеживаются. Сыройlogroot .local/documentation-closure-20261023 сохраняетсякакдополнение;постоянныечисла/история/источникинеживуттольковscratch.

## Related Artifacts

| Artifact | Relation |
|---|---|
| RFC-017 | informs |
| PRD-089 | informs |
