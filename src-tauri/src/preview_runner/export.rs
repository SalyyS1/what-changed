//! A reviewed runnable artifact carries recipes, never browser auth or app consent.
use super::{
    digest, invalid, PreparePreviewRunInput, PreviewRecipeExportDto, PreviewRecipeFileDto,
    PreviewRunner,
};
use crate::error::AppResult;
use crate::owner_workspace::OwnerStore;

pub fn export_recipe(
    store: &OwnerStore,
    input: PreparePreviewRunInput,
) -> AppResult<PreviewRecipeExportDto> {
    if input.mode != "run" {
        return Err(invalid(
            "Export a run recipe rather than a locator-selection session",
        ));
    }
    // Validate through the same read-only scope builder without replacing active consent.
    let plan = PreviewRunner::default().prepare(store, input.clone())?;
    let manifest = serde_json::to_string_pretty(
        &serde_json::json!({"manifest":plan.manifest,"routes":input.routes}),
    )
    .map_err(|_| invalid("Recipe export encoding failed"))?;
    let fixture = serde_json::to_string_pretty(&input.fixture)
        .map_err(|_| invalid("Fixture export encoding failed"))?;
    let sources = [
        (
            "package.json",
            r#"{"name":"ownvero-reviewed-preview-recipe","private":true,"type":"module","engines":{"node":">=24.12.0 <25"},"dependencies":{"playwright-core":"1.62.1"}}"#,
        ),
        (
            "src/run.mjs",
            include_str!("../../../runner/src/export-harness.mjs"),
        ),
        (
            "src/execute-recipe.mjs",
            include_str!("../../../runner/src/execute-recipe.mjs"),
        ),
        (
            "src/browser-session.mjs",
            include_str!("../../../runner/src/browser-session.mjs"),
        ),
        (
            "src/egress-proxy.mjs",
            include_str!("../../../runner/src/egress-proxy.mjs"),
        ),
        (
            "src/policy.mjs",
            include_str!("../../../runner/src/policy.mjs"),
        ),
        (
            "src/recipe-validation.mjs",
            include_str!("../../../runner/src/recipe-validation.mjs"),
        ),
        (
            "src/revision-marker.mjs",
            include_str!("../../../runner/src/revision-marker.mjs"),
        ),
        (
            "fixtures/booking-app/server.mjs",
            include_str!("../../../runner/fixtures/booking-app/server.mjs"),
        ),
        (
            "README.md",
            include_str!("../../../runner/EXPORT-README.md"),
        ),
        ("manifest.json", manifest.as_str()),
        ("fixture.json", fixture.as_str()),
    ];
    let files = sources
        .into_iter()
        .map(|(path, content)| PreviewRecipeFileDto {
            path: path.into(),
            content: content.into(),
            media_type: if path.ends_with(".json") {
                "application/json"
            } else if path.ends_with(".md") {
                "text/markdown"
            } else {
                "text/javascript"
            }
            .into(),
        })
        .collect::<Vec<_>>();
    Ok(PreviewRecipeExportDto{app_id:input.run.app_id,digest:digest(&files)?,files,limitations:vec![
        "This separate artifact requires Node 24.12.x and installation of pinned Playwright/browser resources; setup may use the network.".into(),
        "It contains synthetic fixture values and exact test routes. Review before sharing. Cookies, passwords and one-run consent are excluded.".into(),
        "Results printed by this independent test are not imported as authoritative Ownvero evidence. Unknown revision identity exits unsuccessfully.".into(),
        "Remote previews remain blocked; native confinement, clean Windows and independent owner acceptance remain unverified.".into(),
    ]})
}

pub fn recipe_archive(preview: &PreviewRecipeExportDto) -> AppResult<Vec<u8>> {
    let files = preview
        .files
        .iter()
        .map(|file| (file.path.clone(), file.content.as_bytes().to_vec()))
        .collect();
    crate::owner_workspace::archive::encode_entries(&files)
}
