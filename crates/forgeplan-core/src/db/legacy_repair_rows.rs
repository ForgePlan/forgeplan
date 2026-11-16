//! Pure preparation of legacy identity-repair rows. Publication belongs to the
//! guarded recovery transaction, not this module.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use arrow_array::{Array, LargeStringArray, RecordBatch, StringArray};

use super::{convert, migrate, schema, store};
use crate::artifact::identity_repair::{
    JournalStatus, LegacyIdentityMapping, ValidatedLegacyRepair,
};
use migrate::{IdentityV5Document, IdentityV5Marker, IdentityV5SemanticProtocol, IdentityV5Source};

pub(crate) struct PreparedRepairRows {
    pub(crate) rows: [Vec<RecordBatch>; 3],
    pub(crate) marker: IdentityV5Marker,
    pub(crate) had_legacy_vectors: bool,
}

struct DocumentPair {
    before: IdentityV5Document,
    current: IdentityV5Document,
}

fn row_string(row: &RecordBatch, field: &str) -> Result<String> {
    migrate::required_string(row, field)
}

fn set_optional(row: &RecordBatch, name: &str, value: Option<&str>) -> Result<RecordBatch> {
    let index = row.schema().index_of(name)?;
    let mut columns = row.columns().to_vec();
    columns[index] = Arc::new(StringArray::from(vec![value]));
    Ok(RecordBatch::try_new(row.schema(), columns)?)
}

fn set_body(row: &RecordBatch, value: &str) -> Result<RecordBatch> {
    let index = row.schema().index_of("body")?;
    let mut columns = row.columns().to_vec();
    columns[index] = Arc::new(LargeStringArray::from(vec![value]));
    Ok(RecordBatch::try_new(row.schema(), columns)?)
}

fn old_payload_candidate(row: &RecordBatch, before: &IdentityV5Document, v5: bool) -> Result<bool> {
    if row_string(row, "id")? != before.display_id {
        return Ok(false);
    }
    if v5 && store::record_string(row, "canonical_key", 0).as_deref() != Some(&before.canonical_key)
    {
        return Ok(false);
    }
    let body = store::record_large_string(row, "body", 0).context("legacy artifact has no body")?;
    if body == before.body || body == before.full_text {
        return Ok(true);
    }
    if let Ok((_, parsed_body)) = crate::artifact::frontmatter::parse_frontmatter(&body) {
        return Ok(parsed_body == before.body);
    }
    Ok(false)
}

fn old_document_matches(row: &RecordBatch, before: &IdentityV5Document, v5: bool) -> Result<bool> {
    if row.num_rows() != 1 {
        bail!("legacy artifact batch is not a single row");
    }
    if row_string(row, "id")? != before.display_id {
        return Ok(false);
    }
    if v5 && store::record_string(row, "canonical_key", 0).as_deref() != Some(&before.canonical_key)
    {
        return Ok(false);
    }
    for (name, value) in [
        ("kind", &before.kind),
        ("status", &before.status),
        ("title", &before.title),
        ("depth", &before.depth),
    ] {
        if row_string(row, name)? != *value {
            return Ok(false);
        }
    }
    let body =
        store::record_large_string(row, "body", 0).context("legacy artifact body is absent")?;
    if !migrate::indexed_body_matches_source(&body, &before.body)
        && !migrate::indexed_body_matches_source(&body, &before.full_text)
    {
        return Ok(false);
    }
    if store::record_string(row, "author", 0) != before.author
        || store::record_string(row, "valid_until", 0) != before.valid_until
        || convert::extract_tags(row, 0) != before.tags
    {
        return Ok(false);
    }
    Ok(true)
}

fn unique_old_key<'a>(
    pairs: &'a [DocumentPair],
    reference: &str,
    v5: bool,
) -> Result<&'a DocumentPair> {
    let matches: Vec<_> = pairs
        .iter()
        .filter(|pair| {
            if v5 {
                pair.before.canonical_key == reference
            } else {
                pair.before.display_id == reference
            }
        })
        .collect();
    match matches.as_slice() {
        [pair] => Ok(*pair),
        [] => bail!("legacy identity {reference:?} has no source document"),
        _ => bail!("legacy identity {reference:?} is ambiguous across source documents"),
    }
}

fn old_target_matches(
    pairs: &[DocumentPair],
    declared: &str,
    indexed: &str,
    v5: bool,
) -> Result<bool> {
    if declared == indexed {
        return Ok(true);
    }
    if declared.contains('@') || indexed.contains('@') {
        return Ok(false);
    }
    let declared_pairs: Vec<_> = pairs
        .iter()
        .filter(|pair| pair.before.canonical_key == declared || pair.before.display_id == declared)
        .collect();
    let indexed_pairs: Vec<_> = pairs
        .iter()
        .filter(|pair| {
            if v5 {
                pair.before.canonical_key == indexed
            } else {
                pair.before.display_id == indexed
            }
        })
        .collect();
    match (declared_pairs.as_slice(), indexed_pairs.as_slice()) {
        ([left], [right]) => Ok(left.before.relative_path == right.before.relative_path),
        ([], _) | (_, []) => Ok(false),
        _ => bail!("legacy relation target {declared:?} / {indexed:?} is ambiguous"),
    }
}

// The old corpus may be ambiguous by design. Only an exact indexed
// occurrence and the journaled rewrite to one of its old claimants can
// establish which reviewed target was selected.
fn indexed_parent_matches(
    pair: &DocumentPair,
    indexed: Option<&str>,
    old_candidates: &[crate::artifact::canonical_key::ArtifactCandidate],
    candidates: &[crate::artifact::canonical_key::ArtifactCandidate],
    mappings: &BTreeMap<String, LegacyIdentityMapping>,
    forward_from_preimage: bool,
) -> Result<bool> {
    let declared = pair.before.parent_epic.as_deref();
    let resolve = |value: Option<&str>| {
        value
            .map(|value| migrate::resolve_reference(old_candidates, value))
            .transpose()
    };
    match (resolve(indexed), resolve(declared)) {
        (Ok(indexed), Ok(declared)) => Ok(indexed == declared),
        (_, Err(error))
            if forward_from_preimage
                && indexed == declared
                && mappings.contains_key(&pair.before.relative_path) =>
        {
            use crate::artifact::canonical_key::{ResolveError, resolve_structural_target};
            let Some(declared) = declared else {
                return Err(error);
            };
            let claimants = match resolve_structural_target(declared, old_candidates) {
                Err(ResolveError::Ambiguous { candidates, .. })
                | Err(ResolveError::ConflictingCanonicalKey { candidates, .. }) => candidates,
                _ => return Err(error),
            };
            let Some(post_parent) = pair.current.parent_epic.as_deref() else {
                return Err(error);
            };
            if post_parent == declared {
                return Err(error);
            }
            let Some(selected) = resolve_structural_target(post_parent, candidates)? else {
                return Err(error);
            };
            let path = selected
                .relative_path
                .to_str()
                .context("non-UTF-8 parent source path")?;
            let Some(claimant) = claimants
                .iter()
                .find(|candidate| candidate.relative_path == selected.relative_path)
            else {
                return Err(error);
            };
            let matches_identity = match mappings.get(path) {
                Some(target) => {
                    target.old_key == claimant.canonical_key
                        && target.new_key == selected.identity.canonical_key
                }
                None => claimant.canonical_key == selected.identity.canonical_key,
            };
            if !matches_identity {
                return Err(error);
            }
            Ok(true)
        }
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}

fn stable_times(document: &IdentityV5Document, operation_time: &str) -> Result<(String, String)> {
    let (frontmatter, _) = crate::artifact::frontmatter::parse_frontmatter(&document.full_text)?;
    let get = |key: &str| {
        frontmatter
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    let created = get("created_at")
        .or_else(|| get("created"))
        .unwrap_or_else(|| operation_time.to_string());
    let updated = get("updated_at")
        .or_else(|| get("updated"))
        .unwrap_or_else(|| operation_time.to_string());
    Ok((created, updated))
}

fn new_document_row(
    document: &IdentityV5Document,
    candidates: &[crate::artifact::canonical_key::ArtifactCandidate],
    operation_time: &str,
) -> Result<RecordBatch> {
    let parent = document
        .parent_epic
        .as_deref()
        .map(|value| migrate::resolve_reference(candidates, value))
        .transpose()?;
    let artifact = store::NewArtifact {
        canonical_key: document.canonical_key.clone(),
        id: document.display_id.clone(),
        kind: document.kind.clone(),
        status: document.status.clone(),
        title: document.title.clone(),
        body: document.full_text.clone(),
        depth: document.depth.clone(),
        author: document.author.clone(),
        parent_epic: parent,
        valid_until: document.valid_until.clone(),
        tags: document.tags.clone(),
    };
    let (created, updated) = stable_times(document, operation_time)?;
    let row = convert::artifact_to_batch(&artifact, &created)?;
    let row = migrate::replace_string(&row, "updated_at", &updated)?;
    migrate::replace_string(&row, "semantic_state", "pending")
}

fn pairs_for_image(
    source: &IdentityV5Source,
    mappings: &BTreeMap<String, LegacyIdentityMapping>,
    operation_time: &str,
    postimage: bool,
) -> Result<Vec<DocumentPair>> {
    source
        .documents
        .iter()
        .map(|current| {
            let before = match mappings.get(&current.relative_path) {
                Some(mapping) => {
                    let bytes = if postimage {
                        &mapping.postimage
                    } else {
                        &mapping.preimage
                    };
                    migrate::parse_identity_v5_document(
                        &current.relative_path,
                        std::str::from_utf8(bytes)?,
                        operation_time,
                    )?
                }
                None => current.clone(),
            };
            Ok(DocumentPair {
                before,
                current: current.clone(),
            })
        })
        .collect()
}

fn pairs_for_linked_postimage(
    source: &IdentityV5Source,
    predecessor_mappings: &[LegacyIdentityMapping],
    operation_time: &str,
) -> Result<Vec<DocumentPair>> {
    let mut predecessor_by_path = BTreeMap::new();
    for mapping in predecessor_mappings {
        if predecessor_by_path
            .insert(mapping.path.as_str(), mapping)
            .is_some()
        {
            bail!("linked predecessor has duplicate source mapping");
        }
    }
    if predecessor_by_path.keys().any(|path| {
        !source
            .documents
            .iter()
            .any(|document| document.relative_path.as_str() == *path)
    }) {
        bail!("linked predecessor source is absent from the captured corpus");
    }
    source
        .documents
        .iter()
        .map(|current| {
            let before = match predecessor_by_path.get(current.relative_path.as_str()) {
                Some(mapping) => {
                    let document = migrate::parse_identity_v5_document(
                        &current.relative_path,
                        std::str::from_utf8(&mapping.postimage)?,
                        operation_time,
                    )?;
                    if document.display_id != mapping.display_id
                        || document.canonical_key != mapping.new_key
                    {
                        bail!(
                            "linked predecessor postimage identity differs at {}",
                            current.relative_path
                        );
                    }
                    document
                }
                None => current.clone(),
            };
            Ok(DocumentPair {
                before,
                current: current.clone(),
            })
        })
        .collect()
}

fn artifact_rows_match_image(
    rows: &[RecordBatch],
    pairs: &[DocumentPair],
    v5: bool,
) -> Result<bool> {
    for row in rows {
        let mut matched = None;
        for (index, pair) in pairs.iter().enumerate() {
            if old_document_matches(row, &pair.before, v5)? && matched.replace(index).is_some() {
                return Ok(false);
            }
        }
        if matched.is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn attributed_pairs(
    rows: &[RecordBatch],
    source: &IdentityV5Source,
    mappings: &BTreeMap<String, LegacyIdentityMapping>,
    operation_time: &str,
    v5: bool,
    marker: Option<&IdentityV5Marker>,
    predecessor_mappings: Option<&[LegacyIdentityMapping]>,
) -> Result<(Vec<DocumentPair>, bool)> {
    // Linked recovery requires its predecessor's indexed rows. A first repair
    // may legitimately start from a source-only corpus with no indexed rows.
    if predecessor_mappings.is_some() && rows.is_empty() && !source.documents.is_empty() {
        bail!("legacy artifact index is empty and cannot establish row provenance");
    }
    let pre = pairs_for_image(source, mappings, operation_time, false)?;
    let pre_matches = artifact_rows_match_image(rows, &pre, v5)?;
    if marker.is_none() {
        if !pre_matches {
            bail!(
                "legacy artifact rows do not match journal preimages and current canonical source"
            );
        }
        return Ok((pre, false));
    }

    let post = pairs_for_image(source, mappings, operation_time, true)?;
    let post_matches = artifact_rows_match_image(rows, &post, v5)?;
    if let Some(predecessor_mappings) = predecessor_mappings {
        let predecessor = pairs_for_linked_postimage(source, predecessor_mappings, operation_time)?;
        if !pre_matches && !post_matches && artifact_rows_match_image(rows, &predecessor, v5)? {
            return Ok((predecessor, true));
        }
    }
    let mut mapped_row_exists = false;
    for row in rows {
        for (pre_pair, post_pair) in pre.iter().zip(&post) {
            let Some(mapping) = mappings.get(&pre_pair.before.relative_path) else {
                continue;
            };
            if mapping.preimage == mapping.postimage {
                continue;
            }
            if old_document_matches(row, &pre_pair.before, v5)?
                || old_document_matches(row, &post_pair.before, v5)?
            {
                mapped_row_exists = true;
                break;
            }
        }
        if mapped_row_exists {
            break;
        }
    }
    match (pre_matches, post_matches) {
        (true, false) => Ok((pre, false)),
        (false, true) => Ok((post, true)),
        (false, false) => bail!(
            "legacy artifact rows match neither journal preimages nor postimages and current canonical source"
        ),
        (true, true) if !mapped_row_exists => Ok((pre, false)),
        (true, true) => {
            bail!("legacy artifact rows do not uniquely identify their journal source image")
        }
    }
}

pub(crate) async fn prepare(
    proof: &ValidatedLegacyRepair<'_>,
    old_rows: &[Vec<RecordBatch>; 3],
    marker: Option<&IdentityV5Marker>,
    source: &IdentityV5Source,
    operation_time: &str,
) -> Result<PreparedRepairRows> {
    if operation_time.is_empty() {
        bail!("repair operation timestamp is empty");
    }
    proof.revalidate(proof.workspace())?;
    let actual = migrate::read_identity_v5_source_at(proof.workspace(), operation_time).await?;
    if actual.source_digest != source.source_digest
        || actual.documents.len() != source.documents.len()
        || actual
            .documents
            .iter()
            .zip(&source.documents)
            .any(|(a, b)| a.relative_path != b.relative_path || a.full_text != b.full_text)
    {
        bail!("identity source changed before row preparation");
    }
    let source = &actual;
    if migrate::verify_identity_v5_source(source).is_blocked() {
        bail!("identity source is ambiguous or invalid");
    }
    if let Some(marker) = marker
        && (marker.version != 5 || !marker.completed)
    {
        bail!("invalid old identity marker");
    }
    let has_old_vector = old_rows[0].iter().any(|row| {
        row.column_by_name("embedding")
            .is_some_and(|column| (0..row.num_rows()).any(|i| !column.is_null(i)))
    });
    if has_old_vector
        && crate::workspace::load_config(proof.workspace())?
            .embedding
            .is_none()
        && marker.and_then(|value| value.embeddings.as_ref()).is_none()
    {
        bail!(
            "legacy vectors cannot be rebuilt without an embedding configuration or verified profile"
        );
    }
    let v5 = marker.is_some();
    let (status, mappings) = proof.identity_mappings()?;
    let applying = match status {
        JournalStatus::CanonicalCommittedIndexPending => true,
        JournalStatus::RolledBackIndexPending => false,
        _ => bail!("journal is not in an index recovery state"),
    };
    let mappings: BTreeMap<_, _> = mappings.into_iter().map(|m| (m.path.clone(), m)).collect();
    if mappings.keys().any(|path| {
        !source
            .documents
            .iter()
            .any(|doc| &doc.relative_path == path)
    }) {
        bail!("journaled source is absent from the captured corpus");
    }
    for current in &source.documents {
        if let Some(mapping) = mappings.get(&current.relative_path) {
            let expected = if applying {
                &mapping.postimage
            } else {
                &mapping.preimage
            };
            if current.full_text.as_bytes() != expected {
                bail!("source differs from journal at {}", current.relative_path);
            }
        }
    }
    // A reviewed successor may encounter rows from its exact linked
    // predecessor even though rollback restored the source preimages.
    let predecessor_mappings = if applying && v5 {
        proof.linked_predecessor_postimage_mappings()?
    } else {
        None
    };
    let (pairs, old_postimage) = attributed_pairs(
        &old_rows[0],
        source,
        &mappings,
        operation_time,
        v5,
        marker,
        predecessor_mappings.as_deref(),
    )?;
    // `mappings` membership was checked before selecting the source image.
    let candidates = migrate::identity_v5_candidates(source);
    let old_candidates: Vec<_> = pairs
        .iter()
        .map(|pair| crate::artifact::canonical_key::ArtifactCandidate {
            canonical_key: pair.before.canonical_key.clone(),
            display_id: pair.before.display_id.clone(),
            title: pair.before.title.clone(),
            relative_path: pair.before.relative_path.clone().into(),
        })
        .collect();
    let mut mapped = BTreeSet::new();
    let mut artifacts = Vec::new();
    for row in &old_rows[0] {
        let mut matches = Vec::new();
        for (i, pair) in pairs.iter().enumerate() {
            if old_payload_candidate(row, &pair.before, v5)? {
                matches.push(i);
            }
        }
        let [index] = matches.as_slice() else {
            bail!(
                "legacy artifact {:?} cannot be attributed uniquely",
                store::record_string(row, "id", 0)
            );
        };
        if !old_document_matches(row, &pairs[*index].before, v5)? {
            bail!(
                "legacy artifact metadata or preimage differs at {}",
                pairs[*index].before.relative_path
            );
        }
        if !mapped.insert(*index) {
            bail!(
                "more than one legacy row maps to {}",
                pairs[*index].current.relative_path
            );
        }
        let pair = &pairs[*index];
        let target = schema::artifacts_schema();
        for field in row.schema().fields() {
            if target.field_with_name(field.name()).is_err() {
                bail!("unsupported legacy artifact field {}", field.name());
            }
        }
        let mut columns = Vec::new();
        for field in target.fields() {
            if field.name() == "canonical_key" {
                columns.push(
                    Arc::new(StringArray::from(vec![pair.current.canonical_key.as_str()]))
                        as Arc<dyn Array>,
                );
            } else if field.name() == "embedding" {
                let old = row
                    .column_by_name("embedding")
                    .context("legacy artifact missing embedding")?;
                if old.data_type() != field.data_type() {
                    bail!("legacy artifact embedding has incompatible type");
                }
                columns.push(convert::make_null_embedding_col(1));
            } else if field.name().starts_with("semantic_") {
                columns.push(Arc::new(StringArray::from(vec![None::<&str>])) as Arc<dyn Array>);
            } else {
                let column = row
                    .column_by_name(field.name())
                    .with_context(|| format!("legacy artifact missing {}", field.name()))?;
                if column.data_type() != field.data_type() {
                    bail!(
                        "legacy artifact field {} has incompatible type",
                        field.name()
                    );
                }
                columns.push(column.clone());
            }
        }
        let mut updated = RecordBatch::try_new(target, columns)?;
        let body = &pair.current.full_text;
        updated = set_body(&updated, body)?;
        let hash = store::compute_content_hash(&pair.current.title, body);
        updated = set_optional(&updated, "body_hash", Some(&hash))?;
        let parent = pair
            .current
            .parent_epic
            .as_deref()
            .map(|value| migrate::resolve_reference(&candidates, value))
            .transpose()?;
        let old_parent = store::record_string(row, "parent_epic", 0);
        if !indexed_parent_matches(
            pair,
            old_parent.as_deref(),
            &old_candidates,
            &candidates,
            &mappings,
            applying && !old_postimage,
        )? {
            bail!(
                "legacy parent differs from preimage at {}",
                pair.before.relative_path
            );
        }
        updated = set_optional(&updated, "parent_epic", parent.as_deref())?;
        updated = migrate::replace_string(&updated, "semantic_state", "pending")?;
        artifacts.push(updated);
    }
    for (i, pair) in pairs.iter().enumerate() {
        if !mapped.contains(&i) {
            artifacts.push(new_document_row(
                &pair.current,
                &candidates,
                operation_time,
            )?);
        }
    }
    let mut expected_edges = BTreeMap::<(String, String, String), usize>::new();
    for pair in &pairs {
        for (target, relation) in &pair.current.links {
            let target = migrate::resolve_reference(&candidates, target)?;
            *expected_edges
                .entry((pair.current.canonical_key.clone(), target, relation.clone()))
                .or_default() += 1;
        }
    }
    let mut relations = Vec::new();
    let mut emitted_edges = BTreeSet::new();
    for row in &old_rows[1] {
        if row.num_rows() != 1 {
            bail!("legacy relation batch is not a single row");
        }
        let source_id = row_string(row, "source_id")?;
        let target_id = row_string(row, "target_id")?;
        let relation = row_string(row, "relation_type")?;
        let mut matches = BTreeSet::new();
        for pair in &pairs {
            let source_matches = if v5 {
                pair.before.canonical_key == source_id
            } else {
                pair.before.display_id == source_id
            };
            if !source_matches {
                continue;
            }
            if pair.before.links.len() != pair.current.links.len() {
                bail!(
                    "structural link count changed at {}",
                    pair.before.relative_path
                );
            }
            for ((old_target, old_type), (new_target, new_type)) in
                pair.before.links.iter().zip(&pair.current.links)
            {
                if old_type == &relation
                    && new_type == old_type
                    && old_target_matches(&pairs, old_target, &target_id, v5)?
                {
                    matches.insert((
                        pair.current.canonical_key.clone(),
                        migrate::resolve_reference(&candidates, new_target)?,
                    ));
                }
            }
        }
        if matches.len() != 1 {
            bail!("legacy relation {source_id:?} -> {target_id:?} cannot be attributed uniquely");
        }
        let (source_key, target) = matches.into_iter().next().unwrap();
        let edge = (source_key.clone(), target.clone(), relation);
        match expected_edges.get_mut(&edge) {
            Some(count) if *count > 0 => *count -= 1,
            _ => bail!("legacy relation is absent from current source"),
        }
        // Validate every original occurrence above, but publish each resolved
        // logical edge once. Canonical declarations and reference decisions stay intact.
        if emitted_edges.insert(edge) {
            let updated = migrate::replace_string(row, "source_id", &source_key)?;
            relations.push(migrate::replace_string(&updated, "target_id", &target)?);
        }
    }
    for (edge, count) in expected_edges {
        if count == 0 || emitted_edges.contains(&edge) {
            continue;
        }
        let (source_key, target, relation) = edge;
        let document = source
            .documents
            .iter()
            .find(|d| d.canonical_key == source_key)
            .context("relation source disappeared")?;
        let (created, _) = stable_times(document, operation_time)?;
        relations.push(convert::relation_to_batch(
            &source_key,
            &target,
            &relation,
            &created,
        )?);
    }
    let mut evidence = Vec::new();
    let mut ids = BTreeSet::new();
    for row in &old_rows[2] {
        if row.num_rows() != 1 {
            bail!("legacy evidence batch is not a single row");
        }
        let id = row_string(row, "id")?;
        if !ids.insert(id.clone()) {
            bail!("duplicate legacy evidence {id:?}");
        }
        for (index, field) in row.schema().fields().iter().enumerate() {
            if !field.is_nullable() && row.column(index).is_null(0) {
                bail!("legacy evidence {id:?} has null {}", field.name());
            }
        }
        let old_id = row_string(row, "artifact_id")?;
        let pair = unique_old_key(&pairs, &old_id, v5)?;
        evidence.push(migrate::replace_string(
            row,
            "artifact_id",
            &pair.current.canonical_key,
        )?);
    }
    let semantic = Some(IdentityV5SemanticProtocol {
        protocol: 1,
        active_profile_digest: None,
    });
    let marker = IdentityV5Marker {
        version: 5,
        artifacts_table: migrate::IDENTITY_V5_ARTIFACTS_TABLE.to_string(),
        relations_table: migrate::IDENTITY_V5_RELATIONS_TABLE.to_string(),
        evidence_table: migrate::IDENTITY_V5_EVIDENCE_TABLE.to_string(),
        source_digest: source.source_digest.clone(),
        document_count: source.documents.len(),
        completed: true,
        embeddings: marker.and_then(|old| old.embeddings.clone()),
        semantic,
    };
    proof.revalidate(proof.workspace())?;
    let after = migrate::read_identity_v5_source_at(proof.workspace(), operation_time).await?;
    if after.source_digest != source.source_digest {
        bail!("identity source changed during row preparation");
    }
    Ok(PreparedRepairRows {
        rows: [artifacts, relations, evidence],
        marker,
        had_legacy_vectors: has_old_vector,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(path: &str, body: &str) -> IdentityV5Document {
        migrate::parse_identity_v5_document(path, &format!(
            "---\nid: PRD-001\nkind: prd\ntitle: Example\nstatus: draft\ncreated_at: 2024-01-01\nupdated_at: 2024-01-01\n---\n{body}\n"
        ), "2025-01-01T00:00:00Z").unwrap()
    }

    fn pair(path: &str, body: &str) -> DocumentPair {
        let before = doc(path, body);
        DocumentPair {
            current: before.clone(),
            before,
        }
    }

    #[test]
    fn parent_forward_reference_is_preserved_but_invalid_is_rejected() {
        let pair = pair("prds/a.md", "alpha");
        let candidates = migrate::identity_v5_candidates(&IdentityV5Source {
            documents: vec![pair.before.clone()],
            source_digest: String::new(),
        });
        let mut unresolved = pair;
        unresolved.before.parent_epic = Some("EPIC-A01".into());
        assert!(
            indexed_parent_matches(
                &unresolved,
                Some("EPIC-A01"),
                &candidates,
                &candidates,
                &BTreeMap::new(),
                false
            )
            .unwrap()
        );
        unresolved.before.parent_epic = Some("---".into());
        assert!(
            indexed_parent_matches(
                &unresolved,
                Some("---"),
                &candidates,
                &candidates,
                &BTreeMap::new(),
                false
            )
            .is_err()
        );
        unresolved.current.parent_epic = Some("EPIC-A01".into());
        let row = new_document_row(&unresolved.current, &candidates, "2025-01-01").unwrap();
        assert_eq!(
            store::record_string(&row, "parent_epic", 0).as_deref(),
            Some("EPIC-A01")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reconcile_preserves_reviewed_parent_for_indexed_v4_child() {
        use crate::artifact::identity_repair::{self, ReferenceDecision};
        for (selected, journaled, wrong_indexed_parent) in [
            ("b", false, false),
            ("c", false, false),
            ("c", true, false),
            ("c", false, true),
        ] {
            let root = tempfile::tempdir().unwrap();
            let ws = migrate::populated_identity_v4_workspace_for_test(root.path())
                .await
                .unwrap()
                .canonicalize()
                .unwrap();
            std::fs::write(ws.join("config.yaml"), "{}\n").unwrap();
            std::fs::create_dir_all(ws.join("epics")).unwrap();
            for (name, key) in [
                ("a", "epic-shared"),
                ("b", "epic-shared"),
                ("c", "epic-stable"),
            ] {
                let links = if name == "c" && journaled {
                    "links:\n  - target: epic-shared\n    relation: informs\n"
                } else {
                    ""
                };
                std::fs::write(ws.join(format!("epics/{name}.md")), format!(
                    "---\nid: EPIC-001\nkind: epic\nstatus: draft\ntitle: Epic {name}\nslug: {key}\n{links}---\n\nEpic {name}.\n"
                )).unwrap();
            }
            let child_path = "prds/PRD-001-auth.md";
            let child_file = ws.join(child_path);
            let content = std::fs::read_to_string(&child_file).unwrap().replace(
                "slug: prd-auth-system\n",
                "slug: prd-auth-system\nparent_epic: EPIC-001\n",
            );
            std::fs::write(&child_file, content).unwrap();
            let db = lancedb::connect(ws.join("lance").to_str().unwrap())
                .execute()
                .await
                .unwrap();
            let table = db.open_table("artifacts").execute().await.unwrap();
            let mut rows = migrate::table_rows(&db, "artifacts").await.unwrap();
            let child = rows
                .iter()
                .position(|row| migrate::required_string(row, "id").unwrap() == "PRD-001")
                .unwrap();
            rows[child] = migrate::replace_string(
                &rows[child],
                "parent_epic",
                if wrong_indexed_parent {
                    "EPIC-999"
                } else {
                    "EPIC-001"
                },
            )
            .unwrap();
            table.delete("true").await.unwrap();
            table.add(rows).execute().await.unwrap();
            drop(table);
            drop(db);

            let census = identity_repair::census_workspace(&ws).unwrap();
            let source = census
                .documents
                .iter()
                .find(|document| document.relative_path == child_path)
                .unwrap();
            let mut decisions = vec![ReferenceDecision {
                source_path: child_path.into(),
                field: "parent_epic".into(),
                occurrence: Some(0),
                source_sha256: Some(source.hashes.file_sha256.clone()),
                from: "EPIC-001".into(),
                target_path: format!("epics/{selected}.md"),
            }];
            if journaled {
                let c = census
                    .documents
                    .iter()
                    .find(|document| document.relative_path == "epics/c.md")
                    .unwrap();
                decisions.push(ReferenceDecision {
                    source_path: "epics/c.md".into(),
                    field: "links".into(),
                    occurrence: Some(0),
                    source_sha256: Some(c.hashes.file_sha256.clone()),
                    from: "epic-shared".into(),
                    target_path: "epics/b.md".into(),
                });
            }
            let plan = identity_repair::build_repair_plan(&ws, &census, &decisions).unwrap();
            let expected_key = if selected == "c" {
                "epic-stable".to_string()
            } else {
                plan.sources
                    .iter()
                    .find(|source| source.relative_path == "epics/b.md")
                    .unwrap()
                    .new_key
                    .clone()
            };
            let outcome = identity_repair::apply_repair_plan(&ws, &plan).unwrap();
            let saved = std::fs::read(&child_file).unwrap();
            let result =
                store::LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await;
            if wrong_indexed_parent {
                assert!(result.is_err());
                assert_eq!(
                    identity_repair::inspect_repair_journal(&ws, &outcome.transaction_id)
                        .unwrap()
                        .status,
                    JournalStatus::CanonicalCommittedIndexPending
                );
                assert!(
                    migrate::read_identity_v5_marker(&ws.join("lance"))
                        .unwrap()
                        .is_none()
                );
            } else {
                result.unwrap();
                let db = lancedb::connect(ws.join("lance").to_str().unwrap())
                    .execute()
                    .await
                    .unwrap();
                let rows = migrate::table_rows(&db, migrate::IDENTITY_V5_ARTIFACTS_TABLE)
                    .await
                    .unwrap();
                let child = rows
                    .iter()
                    .find(|row| migrate::required_string(row, "id").unwrap() == "PRD-001")
                    .unwrap();
                assert_eq!(
                    store::record_string(child, "parent_epic", 0).as_deref(),
                    Some(expected_key.as_str())
                );
                assert_eq!(
                    identity_repair::inspect_repair_journal(&ws, &outcome.transaction_id)
                        .unwrap()
                        .status,
                    JournalStatus::CanonicalCommitted
                );
            }
            assert_eq!(std::fs::read(child_file).unwrap(), saved);
        }
    }

    #[test]
    fn colliding_display_id_cannot_attribute_evidence_by_id_alone() {
        let pairs = vec![pair("prds/a.md", "alpha"), pair("prds/b.md", "beta")];
        assert!(unique_old_key(&pairs, "PRD-001", false).is_err());
        assert!(unique_old_key(&pairs, "PRD-999", false).is_err());
        assert_eq!(
            unique_old_key(&pairs[..1], "PRD-001", false)
                .unwrap()
                .before
                .relative_path,
            "prds/a.md"
        );
    }

    #[test]
    fn artifact_payload_can_distinguish_preimages_but_not_identical_documents() {
        let first = pair("prds/a.md", "alpha");
        let second = pair("prds/b.md", "beta");
        let duplicate = pair("prds/c.md", "alpha");
        let source = IdentityV5Source {
            documents: vec![first.before.clone()],
            source_digest: String::new(),
        };
        let row = new_document_row(
            &first.before,
            &migrate::identity_v5_candidates(&source),
            "2025-01-01",
        )
        .unwrap();
        assert!(old_document_matches(&row, &first.before, false).unwrap());
        assert!(!old_document_matches(&row, &second.before, false).unwrap());
        assert!(old_document_matches(&row, &duplicate.before, false).unwrap());
    }

    #[test]
    fn mismatching_metadata_does_not_become_ownership_evidence() {
        let first = pair("prds/a.md", "alpha");
        let source = IdentityV5Source {
            documents: vec![first.before.clone()],
            source_digest: String::new(),
        };
        let row = new_document_row(
            &first.before,
            &migrate::identity_v5_candidates(&source),
            "2025-01-01",
        )
        .unwrap();
        let row = migrate::replace_string(&row, "status", "active").unwrap();
        assert!(!old_document_matches(&row, &first.before, false).unwrap());
    }

    fn marker() -> IdentityV5Marker {
        IdentityV5Marker {
            version: 5,
            artifacts_table: migrate::IDENTITY_V5_ARTIFACTS_TABLE.to_string(),
            relations_table: migrate::IDENTITY_V5_RELATIONS_TABLE.to_string(),
            evidence_table: migrate::IDENTITY_V5_EVIDENCE_TABLE.to_string(),
            source_digest: "0".repeat(64),
            document_count: 1,
            completed: true,
            embeddings: None,
            semantic: None,
        }
    }

    fn mapping(before: &IdentityV5Document, after: &IdentityV5Document) -> LegacyIdentityMapping {
        LegacyIdentityMapping {
            path: before.relative_path.clone(),
            display_id: before.display_id.clone(),
            old_key: before.canonical_key.clone(),
            new_key: after.canonical_key.clone(),
            preimage: before.full_text.as_bytes().to_vec(),
            postimage: after.full_text.as_bytes().to_vec(),
        }
    }

    #[test]
    fn rows_select_journal_image_without_matching_historical_marker_digest() {
        let before = doc("prds/a.md", "before");
        let after = doc("prds/a.md", "after");
        let source = IdentityV5Source {
            documents: vec![after.clone()],
            source_digest: String::new(),
        };
        let mapping = mapping(&before, &after);
        let mappings = BTreeMap::from([(mapping.path.clone(), mapping)]);
        let row = new_document_row(
            &before,
            &migrate::identity_v5_candidates(&IdentityV5Source {
                documents: vec![before.clone()],
                source_digest: String::new(),
            }),
            "2025-01-01T00:00:00Z",
        )
        .unwrap();

        let (pairs, postimage) = attributed_pairs(
            &[row],
            &source,
            &mappings,
            "2025-01-01T00:00:00Z",
            true,
            Some(&marker()),
            None,
        )
        .unwrap();

        assert!(!postimage);
        assert_eq!(pairs[0].before.full_text, before.full_text);
        assert_eq!(pairs[0].current.full_text, after.full_text);
        let post_row = new_document_row(
            &after,
            &migrate::identity_v5_candidates(&source),
            "2025-01-01T00:00:00Z",
        )
        .unwrap();
        let (post_pairs, postimage) = attributed_pairs(
            &[post_row],
            &source,
            &mappings,
            "2025-01-01T00:00:00Z",
            true,
            Some(&marker()),
            None,
        )
        .unwrap();
        assert!(postimage);
        assert_eq!(post_pairs[0].before.full_text, after.full_text);
    }

    #[test]
    fn linked_predecessor_postimage_rows_are_attributed_but_metadata_mismatches_are_refused() {
        let legacy = migrate::parse_identity_v5_document(
            "prds/a.md",
            "---\nid: PRD-001\nkind: prd\ntitle: Example\nstatus: draft\nauthor: Eli Rum\nslug: prd-legacy-key\n---\nbody\n",
            "2025-01-01T00:00:00Z",
        )
        .unwrap();
        let predecessor = migrate::parse_identity_v5_document(
            "prds/a.md",
            "---\nid: PRD-001\nkind: prd\ntitle: Example\nstatus: draft\nauthor: Eli Rum\nslug: prd-predecessor-key\n---\nbody\n",
            "2025-01-01T00:00:00Z",
        )
        .unwrap();
        let successor = migrate::parse_identity_v5_document(
            "prds/a.md",
            "---\nid: PRD-001\nkind: prd\ntitle: Example\nstatus: draft\nauthor: Eli Rum\nslug: prd-successor-key\n---\nbody\n",
            "2025-01-01T00:00:00Z",
        )
        .unwrap();
        let source = IdentityV5Source {
            documents: vec![successor.clone()],
            source_digest: String::new(),
        };
        let successor_mapping = mapping(&legacy, &successor);
        let successor_mappings =
            BTreeMap::from([(successor_mapping.path.clone(), successor_mapping)]);
        let predecessor_mappings = [mapping(&legacy, &predecessor)];
        let predecessor_source = IdentityV5Source {
            documents: vec![predecessor.clone()],
            source_digest: String::new(),
        };
        let row = new_document_row(
            &predecessor,
            &migrate::identity_v5_candidates(&predecessor_source),
            "2025-01-01T00:00:00Z",
        )
        .unwrap();

        let (pairs, postimage) = attributed_pairs(
            std::slice::from_ref(&row),
            &source,
            &successor_mappings,
            "2025-01-01T00:00:00Z",
            true,
            Some(&marker()),
            Some(&predecessor_mappings),
        )
        .unwrap();
        assert!(postimage);
        assert_eq!(pairs[0].before.canonical_key, "prd-predecessor-key");
        assert_eq!(pairs[0].current.canonical_key, "prd-successor-key");
        assert_eq!(pairs[0].before.full_text, predecessor.full_text);
        assert_eq!(pairs[0].before.author.as_deref(), Some("Eli Rum"));
        let unrelated =
            migrate::replace_string(&row, "canonical_key", "prd-unrelated-key").unwrap();
        assert!(
            attributed_pairs(
                &[unrelated],
                &source,
                &successor_mappings,
                "2025-01-01T00:00:00Z",
                true,
                Some(&marker()),
                Some(&predecessor_mappings),
            )
            .is_err()
        );
        assert!(
            attributed_pairs(
                &[],
                &source,
                &successor_mappings,
                "2025-01-01T00:00:00Z",
                true,
                Some(&marker()),
                Some(&predecessor_mappings),
            )
            .is_err()
        );

        let corrupted = migrate::replace_string(&row, "status", "active").unwrap();
        assert!(
            attributed_pairs(
                &[corrupted],
                &source,
                &successor_mappings,
                "2025-01-01T00:00:00Z",
                true,
                Some(&marker()),
                Some(&predecessor_mappings),
            )
            .is_err()
        );
    }

    #[test]
    fn rows_that_do_not_distinguish_journal_images_are_rejected() {
        let before = migrate::parse_identity_v5_document(
            "prds/a.md",
            "---\nid: PRD-001\nkind: prd\ntitle: Example\nstatus: draft\nparent_epic: EPIC-001\n---\nsame\n",
            "2025-01-01T00:00:00Z",
        ).unwrap();
        let after = migrate::parse_identity_v5_document(
            "prds/a.md",
            "---\nid: PRD-001\nkind: prd\ntitle: Example\nstatus: draft\nparent_epic: EPIC-002\n---\nsame\n",
            "2025-01-01T00:00:00Z",
        ).unwrap();
        let source = IdentityV5Source {
            documents: vec![after.clone()],
            source_digest: String::new(),
        };
        let mapping = mapping(&before, &after);
        let mappings = BTreeMap::from([(mapping.path.clone(), mapping)]);
        let candidates = migrate::identity_v5_candidates(&IdentityV5Source {
            documents: vec![before.clone()],
            source_digest: String::new(),
        });
        let row = new_document_row(&before, &candidates, "2025-01-01T00:00:00Z").unwrap();
        let row = set_body(&row, "same").unwrap();
        assert!(old_document_matches(&row, &before, true).unwrap());
        assert!(old_document_matches(&row, &after, true).unwrap());

        assert!(
            attributed_pairs(
                &[row],
                &source,
                &mappings,
                "2025-01-01T00:00:00Z",
                true,
                Some(&marker()),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn source_only_timestamps_use_durable_operation_time() {
        let text = "---\nid: PRD-001\nkind: prd\ntitle: Example\n---\nbody\n";
        let first =
            migrate::parse_identity_v5_document("prds/a.md", text, "2025-01-01T00:00:00Z").unwrap();
        let second =
            migrate::parse_identity_v5_document("prds/a.md", text, "2025-01-01T00:00:00Z").unwrap();
        let a = stable_times(&first, "2025-01-01T00:00:00Z").unwrap();
        let b = stable_times(&second, "2025-01-01T00:00:00Z").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.0, "2025-01-01T00:00:00Z");
        assert_eq!(a.1, "2025-01-01T00:00:00Z");
    }
}
