CREATE TABLE owner_project_decisions (
 id TEXT NOT NULL, app_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 capability_id TEXT, change_id TEXT,
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 1048576),
 PRIMARY KEY(id,revision), UNIQUE(app_id,id,revision),
 FOREIGN KEY(app_id) REFERENCES owner_apps(id),
 FOREIGN KEY(app_id,capability_id) REFERENCES owner_capabilities(app_id,id),
 FOREIGN KEY(app_id,change_id) REFERENCES owner_change_drafts(app_id,id)
);
CREATE INDEX owner_project_decisions_app ON owner_project_decisions(app_id,id,revision);
CREATE TRIGGER immutable_project_decisions BEFORE UPDATE ON owner_project_decisions BEGIN SELECT RAISE(ABORT,'immutable project decision'); END;
CREATE TABLE owner_imported_history (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, source_digest TEXT NOT NULL,
 payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 16777216),
 UNIQUE(app_id,source_digest), FOREIGN KEY(app_id) REFERENCES owner_apps(id)
);
CREATE TRIGGER immutable_imported_history BEFORE UPDATE ON owner_imported_history BEGIN SELECT RAISE(ABORT,'immutable imported history'); END;
