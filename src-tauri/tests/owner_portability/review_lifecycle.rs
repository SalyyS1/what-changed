use super::*;
use what_changed_lib::owner_workspace::portability::cache::{PortabilityReviewCache, ReviewData};

#[test]
fn saving_requires_the_workspace_to_still_match_the_reviewed_bundle() {
    let (store, app, _, _) = fixture();
    let selection = OwnerExportSelectionInput {
        include_target_details: false,
    };
    let (bytes, preview) =
        prepare_bundle(store.portable_snapshot(&app).unwrap(), &selection).unwrap();
    let cache = PortabilityReviewCache::default();
    cache.put(
        0,
        ReviewData::Export {
            bytes: bytes.clone(),
            preview: preview.clone(),
            selection,
        },
    );
    assert_eq!(
        cache
            .export(
                0,
                &app,
                &preview.digest,
                store.portable_snapshot(&app).unwrap()
            )
            .unwrap(),
        bytes
    );
    let old = store.portable_snapshot(&app).unwrap().records.app;
    store
        .save_app(SaveOwnerAppInput {
            operation_id: id(),
            id: app.clone(),
            expected_version: old.version,
            name: old.name,
            description: "The owner changed the app after reviewing".into(),
            legacy_project_id: None,
            archived: false,
        })
        .unwrap();
    assert!(cache
        .export(
            0,
            &app,
            &preview.digest,
            store.portable_snapshot(&app).unwrap()
        )
        .is_err());
    assert!(cache
        .export(
            1,
            &app,
            &preview.digest,
            store.portable_snapshot(&app).unwrap()
        )
        .is_err());
}
