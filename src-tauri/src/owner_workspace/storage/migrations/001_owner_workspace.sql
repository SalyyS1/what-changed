CREATE TABLE owner_schema_migrations (
 version INTEGER PRIMARY KEY CHECK(version > 0), checksum TEXT NOT NULL
);
CREATE TABLE owner_meta (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1),
 epoch INTEGER NOT NULL CHECK(epoch>=0), admission_open INTEGER NOT NULL CHECK(admission_open IN (0,1))
);
INSERT INTO owner_meta VALUES(1,0,1);
CREATE TABLE owner_apps (
 id TEXT PRIMARY KEY, version INTEGER NOT NULL CHECK(version>0), archived INTEGER NOT NULL CHECK(archived IN(0,1)),
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576)
);
CREATE TABLE owner_capabilities (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, version INTEGER NOT NULL CHECK(version>0), archived INTEGER NOT NULL CHECK(archived IN(0,1)),
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576), UNIQUE(app_id,id),
 FOREIGN KEY(app_id) REFERENCES owner_apps(id)
);
CREATE TABLE owner_rule_revisions (
 rule_id TEXT NOT NULL, app_id TEXT NOT NULL, capability_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576),
 PRIMARY KEY(rule_id,revision), UNIQUE(app_id,rule_id,revision),
 FOREIGN KEY(app_id,capability_id) REFERENCES owner_capabilities(app_id,id)
);
CREATE TABLE owner_environment_revisions (
 id TEXT NOT NULL, app_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), revoked INTEGER NOT NULL CHECK(revoked IN(0,1)),
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576),
 PRIMARY KEY(id,revision), UNIQUE(app_id,id,revision), FOREIGN KEY(app_id) REFERENCES owner_apps(id)
);
CREATE TABLE owner_candidates (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, environment_id TEXT NOT NULL, environment_revision INTEGER NOT NULL,
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576), UNIQUE(app_id,id),
 FOREIGN KEY(app_id,environment_id,environment_revision) REFERENCES owner_environment_revisions(app_id,id,revision)
);
CREATE TABLE owner_change_drafts (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, version INTEGER NOT NULL CHECK(version>0), candidate_id TEXT,
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576), UNIQUE(app_id,id),
 FOREIGN KEY(app_id) REFERENCES owner_apps(id), FOREIGN KEY(app_id,candidate_id) REFERENCES owner_candidates(app_id,id)
);
CREATE TABLE owner_change_rules (
 change_id TEXT NOT NULL, app_id TEXT NOT NULL, rule_id TEXT NOT NULL, revision INTEGER NOT NULL,
 PRIMARY KEY(change_id,rule_id,revision),
 FOREIGN KEY(app_id,change_id) REFERENCES owner_change_drafts(app_id,id),
 FOREIGN KEY(app_id,rule_id,revision) REFERENCES owner_rule_revisions(app_id,rule_id,revision)
);
CREATE TABLE owner_scenario_revisions (
 id TEXT NOT NULL, app_id TEXT NOT NULL, capability_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576),
 PRIMARY KEY(id,revision), UNIQUE(app_id,id,revision),
 FOREIGN KEY(app_id,capability_id) REFERENCES owner_capabilities(app_id,id)
);
CREATE TABLE owner_scenario_rules (
 scenario_id TEXT NOT NULL, scenario_revision INTEGER NOT NULL, app_id TEXT NOT NULL, rule_id TEXT NOT NULL, rule_revision INTEGER NOT NULL,
 PRIMARY KEY(scenario_id,scenario_revision,rule_id,rule_revision),
 FOREIGN KEY(app_id,scenario_id,scenario_revision) REFERENCES owner_scenario_revisions(app_id,id,revision),
 FOREIGN KEY(app_id,rule_id,rule_revision) REFERENCES owner_rule_revisions(app_id,rule_id,revision)
);
CREATE TABLE owner_runs (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, candidate_id TEXT NOT NULL, environment_id TEXT NOT NULL, environment_revision INTEGER NOT NULL,
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576), UNIQUE(app_id,id),
 FOREIGN KEY(app_id,candidate_id) REFERENCES owner_candidates(app_id,id),
 FOREIGN KEY(app_id,environment_id,environment_revision) REFERENCES owner_environment_revisions(app_id,id,revision)
);
CREATE TABLE owner_results (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, run_id TEXT NOT NULL, scenario_id TEXT NOT NULL, scenario_revision INTEGER NOT NULL,
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576), UNIQUE(app_id,id), UNIQUE(run_id,scenario_id,scenario_revision),
 FOREIGN KEY(app_id,run_id) REFERENCES owner_runs(app_id,id),
 FOREIGN KEY(app_id,scenario_id,scenario_revision) REFERENCES owner_scenario_revisions(app_id,id,revision)
);
CREATE TABLE owner_decisions (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, change_id TEXT NOT NULL, candidate_id TEXT NOT NULL,
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576),
 FOREIGN KEY(app_id,change_id) REFERENCES owner_change_drafts(app_id,id),
 FOREIGN KEY(app_id,candidate_id) REFERENCES owner_candidates(app_id,id)
);
CREATE TABLE owner_operations (
 operation_id TEXT PRIMARY KEY, kind TEXT NOT NULL, request_digest TEXT NOT NULL,
 response TEXT NOT NULL CHECK(length(CAST(response AS BLOB)) <= 1048576)
);
CREATE INDEX owner_capabilities_app ON owner_capabilities(app_id,id);
CREATE INDEX owner_rules_app ON owner_rule_revisions(app_id,rule_id,revision);
CREATE INDEX owner_changes_app ON owner_change_drafts(app_id,id);
CREATE INDEX owner_environments_app ON owner_environment_revisions(app_id,id,revision);
CREATE INDEX owner_candidates_app ON owner_candidates(app_id,id);
CREATE INDEX owner_scenarios_app ON owner_scenario_revisions(app_id,id,revision);
CREATE INDEX owner_runs_app ON owner_runs(app_id,candidate_id,id);
CREATE INDEX owner_results_app ON owner_results(app_id,scenario_id,scenario_revision,id);
CREATE INDEX owner_decisions_app ON owner_decisions(app_id,id);
CREATE TRIGGER immutable_owner_rules BEFORE UPDATE ON owner_rule_revisions BEGIN SELECT RAISE(ABORT,'immutable rule revision'); END;
CREATE TRIGGER immutable_owner_environments BEFORE UPDATE ON owner_environment_revisions BEGIN SELECT RAISE(ABORT,'immutable environment revision'); END;
CREATE TRIGGER immutable_owner_candidates BEFORE UPDATE ON owner_candidates BEGIN SELECT RAISE(ABORT,'immutable candidate revision'); END;
CREATE TRIGGER immutable_owner_scenarios BEFORE UPDATE ON owner_scenario_revisions BEGIN SELECT RAISE(ABORT,'immutable scenario revision'); END;
CREATE TRIGGER immutable_owner_results BEFORE UPDATE ON owner_results BEGIN SELECT RAISE(ABORT,'immutable result'); END;
CREATE TRIGGER immutable_owner_decisions BEFORE UPDATE ON owner_decisions BEGIN SELECT RAISE(ABORT,'immutable owner decision'); END;
CREATE TRIGGER immutable_owner_operations BEFORE UPDATE ON owner_operations BEGIN SELECT RAISE(ABORT,'immutable operation'); END;
