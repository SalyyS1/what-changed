use super::*;
use crate::dto::owner_portability_dto::*;
use crate::privacy::{Redaction, Sanitizer, Scrub};
use crate::storage::to_json;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize)]
struct ImportWrite {
    input: ApplyOwnerImportInput,
    data: PortableWorkspace,
}
impl Scrub for ImportWrite {
    fn scrub_in_place(&mut self, s: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        self.input.name.scrub_in_place(s, field, report);
        self.input.confirmed_by.scrub_in_place(s, field, report);
        // Reuse the reviewed, sanitized bundle as one typed record graph. No
        // original unsanitized bytes enter persistence or the operation digest.
    }
}

impl OwnerStore {
    pub fn apply_portable_import(
        &self,
        data: PortableWorkspace,
        input: ApplyOwnerImportInput,
        new_app_id: String,
    ) -> AppResult<OwnerImportResultDto> {
        data.validate()?;
        validation::uuid(&new_app_id, "new app id")?;
        if data.history.len() >= 20 {
            return Err(validation::invalid(
                "import history",
                "at most 20 generations can be retained",
            ));
        }
        // Public Rust callers receive the same default privacy boundary as IPC.
        // Imported executable/target details stay archival even if selected
        // when the source bundle was exported.
        let (sanitized_bytes, _) = prepare_bundle(
            data,
            &OwnerExportSelectionInput {
                include_target_details: false,
            },
        )?;
        let data = validate_bundle(&sanitized_bytes)?;
        validation::required(&input.name, "imported app name")?;
        validation::required(&input.confirmed_by, "import reviewer")?;
        let expected: BTreeSet<_> = data
            .records
            .rules
            .iter()
            .map(|r| (&r.rule_id, r.revision))
            .collect();
        let confirmed: BTreeSet<_> = input
            .confirmed_rules
            .iter()
            .map(|r| (&r.rule_id, r.revision))
            .collect();
        if confirmed.len() != input.confirmed_rules.len() || confirmed != expected {
            return Err(validation::invalid("imported rules","review and explicitly confirm every imported rule revision before activating this memory"));
        }
        if data.records.capabilities.len() > MAX_CAPABILITIES_PER_APP
            || data
                .records
                .rules
                .iter()
                .map(|r| &r.rule_id)
                .collect::<BTreeSet<_>>()
                .len()
                > MAX_RULES_PER_APP
        {
            return Err(validation::invalid(
                "import",
                "workspace exceeds supported capacity",
            ));
        }
        // Large archival snapshots use their own 16 MiB bound, not the normal
        // 1 MiB per-operation response bound. The write primitive still checks
        // the durable admission epoch before and after this single transaction.
        let epoch = self.capture_epoch()?;
        validation::uuid(&input.operation_id, "operation id")?;
        let (clean, _) = Sanitizer::new().sanitize(ImportWrite { input, data });
        let request = to_json(clean.get())?;
        if request.len() > crate::owner_workspace::archive::MAX_ARCHIVE_BYTES {
            return Err(validation::invalid("import", "exceeds 16 MiB"));
        }
        let request_digest = super::super::migrations::digest(request.as_bytes());
        self.write_at_epoch(epoch,move|tx| {
            let request=clean.get(); let input=&request.input; let data=&request.data;
            let prior:Option<(String,String)>=tx.query_row("SELECT request_digest,response FROM owner_operations WHERE operation_id=?1 AND kind='import_memory'",[&input.operation_id],|r|Ok((r.get(0)?,r.get(1)?))).optional().during("check import retry")?;
            if let Some((digest,response))=prior {if digest!=request_digest {return Err(validation::invalid("operation id","conflicting import retry"));} return from_json("import response",&response);}
            super::super::repository::capacity(tx,"SELECT count(*) FROM owner_apps WHERE ?1 IS NOT NULL",&new_app_id,MAX_APPS,"apps")?;
            let now=crate::domain::now_ms();
            let app=OwnerAppDto{id:new_app_id.clone(),name:input.name.clone(),description:data.records.app.description.clone(),legacy_project_id:None,current_candidate_id:None,version:1,archived_at:None,created_at:now,updated_at:now};
            tx.execute("INSERT INTO owner_apps(id,version,archived,payload) VALUES(?1,1,0,?2)",params![app.id,to_json(&app)?]).during("import app")?;
            let mut cap_ids=BTreeMap::new();
            for cap in &data.records.capabilities {
                let mut cap=cap.clone(); let id=uuid::Uuid::new_v4().to_string(); cap_ids.insert(cap.id.clone(),id.clone()); cap.id=id; cap.app_id=new_app_id.clone();
                tx.execute("INSERT INTO owner_capabilities(id,app_id,version,archived,payload) VALUES(?1,?2,?3,?4,?5)",params![cap.id,cap.app_id,cap.version,cap.archived_at.is_some(),to_json(&cap)?]).during("import capability")?;
            }
            let mut rule_ids=BTreeMap::new();
            for rule in &data.records.rules {
                let mut rule=rule.clone(); let mapped=rule_ids.entry(rule.rule_id.clone()).or_insert_with(||uuid::Uuid::new_v4().to_string()).clone(); rule.rule_id=mapped; rule.app_id=new_app_id.clone();
                rule.capability_id=cap_ids.get(&rule.capability_id).ok_or_else(||validation::invalid("import rule","missing capability"))?.clone(); rule.provenance=RuleProvenance::ImportedUntrusted; rule.confirmed_by=input.confirmed_by.clone(); rule.confirmed_at=now;
                tx.execute("INSERT INTO owner_rule_revisions(rule_id,app_id,capability_id,revision,payload) VALUES(?1,?2,?3,?4,?5)",params![rule.rule_id,rule.app_id,rule.capability_id,rule.revision,to_json(&rule)?]).during("import reviewed rule")?;
            }
            for change in &data.records.changes {
                let mut change=change.clone(); change.id=uuid::Uuid::new_v4().to_string();change.app_id=new_app_id.clone();change.candidate_id=None;change.state=ChangeState::Draft;
                for rule in &mut change.preserved_rules {rule.rule_id=rule_ids.get(&rule.rule_id).ok_or_else(||validation::invalid("import change","missing rule"))?.clone();}
                tx.execute("INSERT INTO owner_change_drafts(id,app_id,version,candidate_id,payload) VALUES(?1,?2,?3,NULL,?4)",params![change.id,change.app_id,change.version,to_json(&change)?]).during("import draft")?;
                for rule in &change.preserved_rules {tx.execute("INSERT INTO owner_change_rules(change_id,app_id,rule_id,revision) VALUES(?1,?2,?3,?4)",params![change.id,change.app_id,rule.rule_id,rule.revision]).during("import preserved rule")?;}
            }
            // Evidence, decisions, target details, scenario definitions and
            // original IDs remain readable history, never active verification.
            let mut histories=data.history.clone();histories.push(PortableHistory{source_digest:input.digest.clone(),records:data.records.clone()});
            let mut seen=BTreeSet::new();
            for history in histories {if seen.insert(history.source_digest.clone()) {tx.execute("INSERT INTO owner_imported_history(id,app_id,source_digest,payload) VALUES(?1,?2,?3,?4)",params![uuid::Uuid::new_v4().to_string(),app.id,history.source_digest,to_json(&history)?]).during("import historical memory")?;}}
            let result=OwnerImportResultDto{app,source_digest:input.digest.clone(),limitations:IMPORT_LIMITATIONS.iter().map(|s|s.to_string()).collect()};
            tx.execute("INSERT INTO owner_operations(operation_id,kind,request_digest,response) VALUES(?1,'import_memory',?2,?3)",params![input.operation_id,request_digest,to_json(&result)?]).during("record import retry")?;
            Ok(result)
        })
    }
}
