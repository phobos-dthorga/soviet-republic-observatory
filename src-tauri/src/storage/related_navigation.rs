use std::collections::HashMap;

use rusqlite::Connection;

use crate::error::ObservatoryError;
use crate::model::ExactObservationReference;

pub(crate) fn receiver_exact_observations(
    connection: &Connection,
    interpretation_id: &str,
) -> Result<HashMap<u32, ExactObservationReference>, ObservatoryError> {
    let mut statement = connection.prepare(
        r#"WITH RECURSIVE selected_history(
               node_id, parent_node_id, record_id, year, day
           ) AS (
               SELECT node.node_id, node.parent_node_id, node.record_id,
                      node.year, node.day
               FROM observation_history_tips tip
               JOIN observation_sources selected
                 ON selected.payload_hash = tip.payload_hash
               JOIN receiver_history_nodes node ON node.node_id = tip.tip_node_id
               WHERE selected.interpretation_id = ?1
               UNION ALL
               SELECT parent.node_id, parent.parent_node_id, parent.record_id,
                      parent.year, parent.day
               FROM receiver_history_nodes parent
               JOIN selected_history child ON parent.node_id = child.parent_node_id
           )
           SELECT history.record_id, candidate.interpretation_id,
                  membership.branch_id, history.year, history.day
           FROM selected_history history
           JOIN observation_history_tips candidate_tip
             ON candidate_tip.tip_node_id = history.node_id
           JOIN observation_sources candidate
             ON candidate.payload_hash = candidate_tip.payload_hash
           JOIN timeline_branch_memberships membership
             ON membership.interpretation_id = candidate.interpretation_id
           JOIN observation_sources selected ON selected.interpretation_id = ?1
           JOIN analysis_context_state context ON context.singleton_id = 1
           WHERE membership.branch_id = context.selected_branch_id
             AND candidate.resolved_profile_hash = selected.resolved_profile_hash
           ORDER BY history.record_id, candidate.interpretation_id"#,
    )?;
    let rows = statement.query_map([interpretation_id], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            ExactObservationReference {
                interpretation_id: row.get(1)?,
                branch_id: row.get(2)?,
                year: row.get(3)?,
                day: row.get(4)?,
            },
        ))
    })?;
    let mut candidates = HashMap::<u32, Vec<ExactObservationReference>>::new();
    for row in rows {
        let (record_id, reference) = row?;
        candidates.entry(record_id).or_default().push(reference);
    }
    Ok(unique_references(candidates))
}

pub(crate) fn market_exact_observations(
    connection: &Connection,
    interpretation_id: &str,
) -> Result<HashMap<String, ExactObservationReference>, ObservatoryError> {
    let mut statement = connection.prepare(
        r#"SELECT record.record_hash, candidate.interpretation_id,
                  membership.branch_id, record.year, record.day
           FROM timeline_branch_memberships membership
           JOIN observation_sources candidate
             ON candidate.interpretation_id = membership.interpretation_id
           JOIN market_observation_coverage coverage
             ON coverage.payload_hash = candidate.payload_hash
           JOIN market_observation_records membership_record
             ON membership_record.payload_hash = candidate.payload_hash
            AND membership_record.ordinal = coverage.history_records - 1
           JOIN market_records record USING(record_hash)
           JOIN observation_sources selected ON selected.interpretation_id = ?1
           JOIN analysis_context_state context ON context.singleton_id = 1
           WHERE membership.branch_id = context.selected_branch_id
             AND candidate.resolved_profile_hash = selected.resolved_profile_hash
           ORDER BY record.record_hash, candidate.interpretation_id"#,
    )?;
    let rows = statement.query_map([interpretation_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            ExactObservationReference {
                interpretation_id: row.get(1)?,
                branch_id: row.get(2)?,
                year: row.get(3)?,
                day: row.get(4)?,
            },
        ))
    })?;
    let mut candidates = HashMap::<String, Vec<ExactObservationReference>>::new();
    for row in rows {
        let (record_hash, reference) = row?;
        candidates.entry(record_hash).or_default().push(reference);
    }
    Ok(unique_references(candidates))
}

fn unique_references<K>(
    candidates: HashMap<K, Vec<ExactObservationReference>>,
) -> HashMap<K, ExactObservationReference>
where
    K: std::hash::Hash + Eq,
{
    candidates
        .into_iter()
        .filter_map(|(key, mut references)| {
            (references.len() == 1).then(|| (key, references.remove(0)))
        })
        .collect()
}

impl super::ObservatoryStorage {
    pub(crate) fn market_exact_observation_map(
        &self,
        interpretation_id: &str,
    ) -> Result<HashMap<String, ExactObservationReference>, ObservatoryError> {
        market_exact_observations(&self.connect()?, interpretation_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(interpretation_id: &str) -> ExactObservationReference {
        ExactObservationReference {
            interpretation_id: interpretation_id.to_owned(),
            branch_id: "main".to_owned(),
            year: 2018,
            day: 333,
        }
    }

    #[test]
    fn keeps_only_unambiguous_exact_save_references() {
        let resolved = unique_references(HashMap::from([
            (1_u32, vec![reference("exact")]),
            (2_u32, vec![reference("variant-a"), reference("variant-b")]),
        ]));

        assert_eq!(
            resolved.get(&1).map(|item| item.interpretation_id.as_str()),
            Some("exact")
        );
        assert!(!resolved.contains_key(&2));
    }
}
