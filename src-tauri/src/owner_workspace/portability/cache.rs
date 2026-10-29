//! One expiring review in memory. Purge clears it before reopening admission.
use super::PortableWorkspace;
use crate::dto::owner_portability_dto::*;
use crate::error::AppResult;
use crate::owner_workspace::validation;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const REVIEW_TTL: Duration = Duration::from_secs(600);
pub enum ReviewData {
    Export {
        bytes: Vec<u8>,
        preview: OwnerBundlePreviewDto,
        selection: OwnerExportSelectionInput,
    },
    Import {
        data: Box<PortableWorkspace>,
        preview: OwnerImportPreviewDto,
    },
}
struct Review {
    epoch: u64,
    created: Instant,
    data: ReviewData,
}
#[derive(Default)]
pub struct PortabilityReviewCache {
    review: Mutex<Option<Review>>,
}

impl PortabilityReviewCache {
    pub fn clear(&self) {
        *self.review.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
    pub fn put(&self, epoch: u64, data: ReviewData) {
        *self.review.lock().unwrap_or_else(|p| p.into_inner()) = Some(Review {
            epoch,
            created: Instant::now(),
            data,
        });
    }
    fn reviewed_export(
        &self,
        epoch: u64,
        app_id: &str,
        digest: &str,
    ) -> AppResult<(Vec<u8>, OwnerExportSelectionInput)> {
        let mut guard = self.review.lock().unwrap_or_else(|p| p.into_inner());
        if guard
            .as_ref()
            .is_some_and(|r| r.epoch != epoch || r.created.elapsed() > REVIEW_TTL)
        {
            *guard = None;
        }
        if let Some(Review {
            data:
                ReviewData::Export {
                    bytes,
                    preview,
                    selection,
                },
            ..
        }) = guard.as_ref()
        {
            if preview.app_id == app_id && preview.digest == digest {
                return Ok((bytes.clone(), selection.clone()));
            }
        }
        Err(validation::invalid(
            "export review",
            "expired or changed; preview this bundle again",
        ))
    }
    /// A dialog may stay open while rules, targets or evidence change. Compare
    /// the current sanitized snapshot using the exact reviewed section choices.
    pub fn export(
        &self,
        epoch: u64,
        app_id: &str,
        digest: &str,
        current: PortableWorkspace,
    ) -> AppResult<Vec<u8>> {
        let (bytes, selection) = self.reviewed_export(epoch, app_id, digest)?;
        let (_, latest) = super::prepare_bundle(current, &selection)?;
        if latest.digest != digest {
            return Err(validation::invalid(
                "export review",
                "workspace or evidence changed; prepare and review a new bundle",
            ));
        }
        Ok(bytes)
    }
    pub fn import(
        &self,
        epoch: u64,
        review_id: &str,
        digest: &str,
    ) -> AppResult<(PortableWorkspace, String)> {
        let mut guard = self.review.lock().unwrap_or_else(|p| p.into_inner());
        if guard
            .as_ref()
            .is_some_and(|r| r.epoch != epoch || r.created.elapsed() > REVIEW_TTL)
        {
            *guard = None;
        }
        if let Some(Review {
            data: ReviewData::Import { data, preview },
            ..
        }) = guard.as_ref()
        {
            if preview.review_id == review_id && preview.digest == digest {
                return Ok(((**data).clone(), preview.proposed_app_id.clone()));
            }
        }
        Err(validation::invalid(
            "import review",
            "expired or changed; select and review the bundle again",
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn purge_epoch_and_clear_invalidate_preview_bytes() {
        let cache = PortabilityReviewCache::default();
        cache.put(
            4,
            ReviewData::Export {
                bytes: vec![1],
                selection: OwnerExportSelectionInput {
                    include_target_details: false,
                },
                preview: OwnerBundlePreviewDto {
                    app_id: "app".into(),
                    digest: "digest".into(),
                    bytes: 1,
                    markdown: String::new(),
                    rules: vec![],
                    omissions: vec![],
                    limitations: vec![],
                },
            },
        );
        assert_eq!(
            cache
                .reviewed_export(4, "app", "digest")
                .expect("review exists")
                .0,
            vec![1]
        );
        assert!(cache.reviewed_export(5, "app", "digest").is_err());
        assert!(cache.reviewed_export(4, "app", "digest").is_err());
        cache.clear();
    }
}
