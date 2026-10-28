//! Owner choices bind a frozen candidate/manifest and cannot rewrite observations.

use super::*;

fn references(values: &[ScenarioRefDto]) -> AppResult<std::collections::BTreeSet<(&str, u32)>> {
    let mut set = std::collections::BTreeSet::new();
    for reference in values {
        validation::uuid(&reference.scenario_id, "acknowledged scenario id")?;
        if reference.revision == 0
            || !set.insert((reference.scenario_id.as_str(), reference.revision))
        {
            return Err(validation::invalid(
                "acknowledged scenarios",
                "must name each frozen revision once",
            ));
        }
    }
    Ok(set)
}

impl OwnerStore {
    pub fn record_owner_decision(
        &self,
        input: RecordOwnerEvidenceDecisionInput,
    ) -> AppResult<OwnerEvidenceDecisionDto> {
        validate_run_ids(&input.app_id, &input.run_id)?;
        validation::uuid(&input.id, "decision id")?;
        validation::uuid(&input.change_id, "change id")?;
        validation::required(&input.reason, "owner decision reason")?;
        if input.decision == DecisionKind::Pending {
            return Err(validation::invalid(
                "owner decision",
                "choose accept, accept with exceptions, reject or defer",
            ));
        }
        if input.acknowledged_unresolved.len() > MAX_SCENARIOS_PER_APP {
            return Err(validation::invalid(
                "acknowledged scenarios",
                "at most 100 scenario references are supported",
            ));
        }
        references(&input.acknowledged_unresolved)?;
        self.mutate(input.operation_id.clone(),"record_owner_evidence_decision",input,|tx,clean| {
            let input=clean.get();validation::required(&input.reason,"owner decision reason")?;require_app(tx,&input.app_id)?;
            let review=read::review(tx,&input.app_id,&input.run_id)?;
            if review.run.change_id!=input.change_id || review.run.version!=input.expected_run_version {return Err(validation::invalid("run version","conflict or run belongs to another change"));}
            let change:OwnerChangeDraftDto=row(tx,"SELECT payload FROM owner_change_drafts WHERE app_id=?1 AND id=?2",params![input.app_id,input.change_id])?
                .ok_or_else(||validation::invalid("change","does not belong to this app"))?;
            if change.version!=input.expected_change_version {return Err(validation::invalid("expected_change_version","conflict: reload the saved change"));}
            if input.decision==DecisionKind::Accepted && !review.can_accept {return Err(validation::invalid("owner acceptance","manual or unknown-build evidence cannot establish current acceptance; review explicit exceptions"));}
            if input.decision==DecisionKind::AcceptedWithExceptions {
                if !review.can_accept_with_exceptions {return Err(validation::invalid("owner acceptance","the run must be completed and its frozen inputs must not be stale"));}
                if review.freshness==EvidenceFreshness::Unknown && !input.acknowledge_unknown_build {return Err(validation::invalid("unknown build acknowledgment","explicitly acknowledge that manual observations do not establish build currency"));}
                if references(&input.acknowledged_unresolved)?!=references(&review.unresolved_scenarios)? {return Err(validation::invalid("unresolved scenario acknowledgment","acknowledge every failed, blocked or not-run frozen scenario exactly once"));}
            }
            if row::<OwnerEvidenceDecisionDto>(tx,"SELECT payload FROM owner_decisions WHERE id=?1",[&input.id])?.is_some(){return Err(validation::invalid("decision id","already exists; decisions cannot be overwritten"));}
            let count:i64=tx.query_row("SELECT count(*) FROM owner_decisions WHERE app_id=?1 AND json_extract(payload,'$.run_id')=?2",params![input.app_id,input.run_id],|r|r.get(0)).during("bound per-run owner decisions")?;
            if count>=100 {return Err(validation::invalid("owner decisions","at most 100 immutable decisions per run are supported"));}
            capacity(tx,"SELECT count(*) FROM owner_decisions WHERE app_id=?1",&input.app_id,1000,"owner decisions")?;
            let result=clean_record(OwnerEvidenceDecisionDto {id:input.id.clone(),app_id:input.app_id.clone(),change_id:input.change_id.clone(),change_version:change.version,
                run_id:review.run.id.clone(),run_version:review.run.version,candidate_id:review.run.manifest.candidate.id.clone(),manifest_digest:review.run.manifest_digest.clone(),decision:input.decision,
                reason:input.reason.clone(),acknowledged_unresolved:input.acknowledged_unresolved.clone(),acknowledged_unknown_build:input.acknowledge_unknown_build,
                evidence_freshness:review.freshness,current_freshness:review.freshness,recorded_at:crate::domain::now_ms()})?;
            tx.execute("INSERT INTO owner_decisions(id,app_id,change_id,candidate_id,payload) VALUES(?1,?2,?3,?4,?5)",params![result.id,result.app_id,result.change_id,result.candidate_id,to_json(&result)?]).during("record immutable owner evidence decision")?;
            Ok(result)
        })
    }

    pub fn list_owner_decisions(
        &self,
        app_id: &str,
        change_id: &str,
        page: OwnerPageInput,
    ) -> AppResult<OwnerPageDto<OwnerEvidenceDecisionDto>> {
        validation::uuid(app_id, "app id")?;
        validation::uuid(change_id, "change id")?;
        validation::page(&page)?;
        self.read(|conn| {
            let tx=conn.unchecked_transaction().during("begin owner decision history snapshot")?;
            let mut statement=tx.prepare("SELECT id,payload FROM owner_decisions WHERE app_id=?1 AND change_id=?2 AND id>?3 ORDER BY id LIMIT ?4").during("list owner evidence decisions")?;
            let records=statement.query_map(params![app_id,change_id,page.after.unwrap_or_default(),page.limit+1],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).during("list owner evidence decisions")?.collect::<rusqlite::Result<Vec<_>>>().during("decode owner decision page")?;
            let more=records.len()>page.limit as usize;let mut items=Vec::new();let mut last=None;
            for (id,text) in records.into_iter().take(page.limit as usize) {
                let mut decision:OwnerEvidenceDecisionDto=crate::storage::from_json("owner evidence decision",&text)?;
                let run=get_run(&tx,app_id,&decision.run_id)?;
                decision.current_freshness=read::freshness(&tx,&run)?.0;
                items.push(decision);last=Some(id);
            }
            Ok(OwnerPageDto{items,next_cursor:if more{last}else{None}})
        })
    }
}
