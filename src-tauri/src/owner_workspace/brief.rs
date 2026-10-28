//! Deterministic, review-safe Markdown handoff for an existing coding AI.
//!
//! Rendering is a pure local read. It does not read the clipboard, open a URL,
//! invoke a runner, or write an export file. The caller decides whether to copy
//! the returned preview after the owner reviews it.

use crate::error::AppResult;
use crate::owner_workspace::storage::brief_snapshot;
use crate::owner_workspace::{
    validation, OwnerBriefPreviewDto, OwnerBriefSelectionInput, OwnerChangeDraftDto,
    OwnerEnvironmentRevisionDto, OwnerRuleRevisionDto, OwnerStore, RecipeActionDto,
    ScenarioRevisionDto,
};
use crate::privacy::Sanitizer;

const MAX_BRIEF_BYTES: usize = 32 * 1024;

#[derive(Default)]
struct RenderPrivacy {
    redacted: bool,
    omitted_private: bool,
}

fn escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '`' => output.push_str("&#96;"),
            '\\' | '*' | '_' | '[' | ']' | '|' | '#' | '!' | '(' | ')' => {
                output.push('\\');
                output.push(character);
            }
            '\r' => {}
            '\n' => output.push_str("<br>"),
            character if character.is_control() || matches!(character, '\u{2028}' | '\u{2029}') => {
                output.push(' ')
            }
            character => output.push(character),
        }
    }
    output
}

fn scrub(value: &str, privacy: &mut RenderPrivacy) -> String {
    let (sanitized, report) = Sanitizer::new().sanitize(value.to_owned());
    privacy.redacted |= !report.is_empty();
    static PRIVATE_TEXT: std::sync::OnceLock<Option<regex::Regex>> = std::sync::OnceLock::new();
    // Quoted paths are matched before unquoted forms so spaces inside a quoted
    // username or directory cannot leak a path suffix. URLs never become links.
    let filter = PRIVATE_TEXT.get_or_init(|| regex::Regex::new(
        r#"(?i)(?P<url>(?:[a-z][a-z0-9+.-]*://|www\.)[^\s<>"'`\[\](){}]+)|(?P<path>"(?:[a-z]:[\\/]|\\|/(?:home|users|mnt|tmp|var|private|volumes|media)/)[^"\r\n]+"|'(?:[a-z]:[\\/]|\\|/(?:home|users|mnt|tmp|var|private|volumes|media)/)[^'\r\n]+'|(?:[a-z]:[\\/]|\\|/(?:home|users|mnt|tmp|var|private|volumes|media)/)[^\s<>"'`\[\](){}]+)"#,
    ).ok());
    let Some(filter) = filter else {
        privacy.omitted_private = true;
        return "[text omitted: privacy filter unavailable]".into();
    };
    let filtered = filter.replace_all(sanitized.get(), |captures: &regex::Captures<'_>| {
        privacy.omitted_private = true;
        if captures.name("url").is_some() {
            "[URL omitted]"
        } else {
            "[local path omitted]"
        }
    });
    escape(&filtered)
}

fn selected_origin(value: &str, privacy: &mut RenderPrivacy) -> String {
    let (sanitized, report) = Sanitizer::new().sanitize(value.to_owned());
    privacy.redacted |= !report.is_empty();
    escape(sanitized.get())
}

fn optional_text(value: Option<&str>, redactions: &mut RenderPrivacy) -> String {
    value
        .map(|value| scrub(value, redactions))
        .unwrap_or_else(|| "(none)".into())
}

fn action_summary(action: &RecipeActionDto, redactions: &mut RenderPrivacy) -> String {
    match action {
        RecipeActionDto::Navigate { path } => format!("navigate to `{}`", scrub(path, redactions)),
        RecipeActionDto::Fill {
            locator,
            fixture_field,
        } => {
            format!(
                "fill {} from fixture field `{}`",
                locator_summary(locator, redactions),
                scrub(fixture_field, redactions)
            )
        }
        RecipeActionDto::Click { locator } => {
            format!("click {}", locator_summary(locator, redactions))
        }
        RecipeActionDto::ReadBack { adapter_id } => format!(
            "read back using adapter `{}`",
            scrub(adapter_id, redactions)
        ),
        RecipeActionDto::DenyOtherRole { adapter_id } => format!(
            "check denial using adapter `{}`",
            scrub(adapter_id, redactions)
        ),
        RecipeActionDto::Cleanup { locator } => {
            format!("clean up with {}", locator_summary(locator, redactions))
        }
    }
}

fn locator_summary(
    locator: &crate::owner_workspace::LocatorDto,
    redactions: &mut RenderPrivacy,
) -> String {
    match locator {
        crate::owner_workspace::LocatorDto::Role { role, name } => {
            format!(
                "role `{}` named `{}`",
                scrub(role, redactions),
                scrub(name, redactions)
            )
        }
        crate::owner_workspace::LocatorDto::Label { text } => {
            format!("label `{}`", scrub(text, redactions))
        }
        crate::owner_workspace::LocatorDto::TestId { value } => {
            format!("test id `{}`", scrub(value, redactions))
        }
    }
}

fn render_rule(rule: &OwnerRuleRevisionDto, redactions: &mut RenderPrivacy) -> String {
    format!(
        "- `{}` revision {}: {}\n",
        scrub(&rule.rule_id, redactions),
        rule.revision,
        scrub(&rule.expected_behavior, redactions)
    )
}

fn render_scenario(scenario: &ScenarioRevisionDto, redactions: &mut RenderPrivacy) -> String {
    let mut output = format!(
        "### {} revision {}\n\n",
        scrub(&scenario.title, redactions),
        scenario.revision
    );
    output.push_str(&format!(
        "Scenario ID: `{}`; method: `{:?}`; expected behavior: {}\n\n",
        scrub(&scenario.id, redactions),
        scenario.method,
        scrub(&scenario.expected_behavior, redactions)
    ));
    for (index, step) in scenario.recipe.steps.iter().enumerate() {
        output.push_str(&format!(
            "{}. {}\n",
            index + 1,
            action_summary(&step.action, redactions)
        ));
        for assertion in &step.assertions {
            output.push_str(&format!(
                "   - Assert `{}`: {}. Criterion: {}\n",
                scrub(&assertion.id, redactions),
                scrub(&assertion.expected_behavior, redactions),
                assertion_summary(&assertion.assertion, redactions)
            ));
        }
    }
    for adapter in &scenario.recipe.adapters {
        output.push_str(&format!("- Adapter `{}` ({:?}): GET relative path `{}`; role `{}`; allowed fields: {}. Origin omitted; requires separately reviewed environment configuration.\n",
            scrub(&adapter.id, redactions), adapter.effect, scrub(&adapter.path_template, redactions), scrub(&adapter.role_ref, redactions),
            adapter.allowed_fields.iter().map(|field| scrub(field, redactions)).collect::<Vec<_>>().join(", ")));
    }
    output.push('\n');
    output
}

fn assertion_summary(
    assertion: &crate::owner_workspace::RecipeAssertionDto,
    privacy: &mut RenderPrivacy,
) -> String {
    use crate::owner_workspace::RecipeAssertionDto;
    match assertion {
        RecipeAssertionDto::Visible { locator } => {
            format!("{} is visible", locator_summary(locator, privacy))
        }
        RecipeAssertionDto::TextEquals { locator, expected } => format!(
            "{} text equals `{}`",
            locator_summary(locator, privacy),
            scrub(expected, privacy)
        ),
        RecipeAssertionDto::FieldEquals {
            field,
            fixture_field,
        } => format!(
            "response field `{}` equals fixture field `{}`",
            scrub(field, privacy),
            scrub(fixture_field, privacy)
        ),
        RecipeAssertionDto::Denied {
            allowed_statuses,
            absent_fields,
        } => format!(
            "status is one of {}; response excludes fields {}",
            allowed_statuses
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            absent_fields
                .iter()
                .map(|field| scrub(field, privacy))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn render_environment(
    environment: &OwnerEnvironmentRevisionDto,
    redactions: &mut RenderPrivacy,
) -> String {
    let mut output = format!(
        "- Environment `{}` revision {} ({:?})\n",
        scrub(&environment.label, redactions),
        environment.revision,
        environment.kind
    );
    for origin in &environment.origins {
        output.push_str(&format!(
            "  - Origin `{}`\n",
            selected_origin(
                &format!("{}://{}:{}", origin.scheme, origin.host, origin.port),
                redactions
            )
        ));
    }
    output
}

fn render_core(
    app: &crate::owner_workspace::OwnerAppDto,
    change: &OwnerChangeDraftDto,
    rules: &[OwnerRuleRevisionDto],
    candidate: Option<&crate::owner_workspace::OwnerCandidateRevisionDto>,
    environment: Option<&OwnerEnvironmentRevisionDto>,
    scenarios: &[ScenarioRevisionDto],
    selection: &OwnerBriefSelectionInput,
) -> OwnerBriefPreviewDto {
    let mut redactions = RenderPrivacy::default();
    let mut markdown = String::new();
    let mut omitted = Vec::new();
    let mut limitations: Vec<String> = Vec::new();
    markdown.push_str(&format!(
        "# Owner brief: {}\n\n",
        scrub(&app.name, &mut redactions)
    ));
    markdown.push_str("## Request\n\n");
    markdown.push_str(&format!("- App ID: `{}`\n- Change ID: `{}`\n- Change version: {}\n- State: `{:?}`\n- Desired outcome: {}\n- Example: {}\n- Scope: {}\n\n",
        scrub(&app.id, &mut redactions), scrub(&change.id, &mut redactions), change.version, change.state,
        scrub(&change.intent, &mut redactions), optional_text(Some(&change.example), &mut redactions), optional_text(Some(&change.scope), &mut redactions)));
    markdown.push_str("## Preserved confirmed rules\n\n");
    if rules.is_empty() {
        markdown.push_str(
            "- None confirmed. The owner must review expected behavior before implementation.\n\n",
        );
        limitations.push("No confirmed rules are attached to this change.".into());
    } else {
        for rule in rules {
            markdown.push_str(&render_rule(rule, &mut redactions));
        }
        markdown.push('\n');
    }
    markdown.push_str("## Candidate and evidence boundary\n\n");
    if let Some(candidate) = candidate {
        markdown.push_str(&format!(
            "- Candidate ID: `{}`\n- Identity trust: `{:?}`\n- Identity source: `{:?}`\n",
            scrub(&candidate.id, &mut redactions),
            candidate.trust,
            candidate.source
        ));
        if selection.include_candidate_identity {
            markdown.push_str(&format!(
                "- Candidate identity (owner selected): `{}`\n",
                scrub(&candidate.identity, &mut redactions)
            ));
        } else {
            omitted.push("candidate_identity (select explicitly to include)".into());
        }
        markdown.push_str("- No verification run is implied by this brief; a brief is a request and review handoff.\n\n");
    } else {
        markdown.push_str(
            "- No candidate selected. Verification cannot be current-build evidence.\n\n",
        );
        limitations
            .push("No candidate is selected; this brief cannot establish build identity.".into());
    }
    if selection.include_environment_details {
        if let Some(environment) = environment {
            markdown.push_str("### Selected environment (owner included)\n\n");
            markdown.push_str(&render_environment(environment, &mut redactions));
            markdown.push('\n');
        } else {
            limitations.push("The selected candidate has no matching environment revision.".into());
        }
    } else {
        omitted.push("environment_details (local URL and paths omitted by default)".into());
    }
    if selection.include_scenarios {
        markdown.push_str("## Acceptance scenarios\n\n");
        if scenarios.is_empty() {
            markdown.push_str(
                "- No confirmed scenarios are selected. This remains an explicit no-run state.\n\n",
            );
            limitations.push("No confirmed acceptance scenarios are selected.".into());
        } else {
            for scenario in scenarios {
                markdown.push_str(&render_scenario(scenario, &mut redactions));
            }
        }
    } else {
        omitted.push("acceptance_scenarios (selection disabled)".into());
        limitations.push("Acceptance scenarios were excluded by owner selection.".into());
    }
    if selection.include_limitations {
        markdown.push_str("## Unresolved tests and limitations\n\n");
        markdown.push_str("- No runner was started and no automated pass is claimed.\n");
        markdown.push_str("- The coding AI must preserve every confirmed rule and report blocked or missing evidence.\n");
        for limitation in &limitations {
            markdown.push_str(&format!("- {}\n", scrub(limitation, &mut redactions)));
        }
        if redactions.redacted {
            markdown.push_str("- One or more text values were redacted before rendering.\n");
        }
        markdown.push('\n');
    } else {
        omitted.push("limitations (selection disabled)".into());
    }
    if redactions.omitted_private {
        omitted.push(
            "free_text_urls_and_local_paths (fragments substituted; surrounding rules retained)"
                .into(),
        );
        markdown.push_str("Privacy: local paths and URLs inside free text were substituted with visible placeholders.\n\n");
    }
    let bytes = markdown.len() as u64;
    OwnerBriefPreviewDto {
        app_id: app.id.clone(),
        change_id: change.id.clone(),
        change_version: change.version,
        estimated_tokens: bytes.div_ceil(4),
        bytes,
        markdown,
        omitted_sections: omitted,
        limitations,
    }
}

impl OwnerStore {
    /// Render one deterministic Markdown preview from one consistent owner read snapshot.
    pub fn render_brief(
        &self,
        app_id: &str,
        change_id: &str,
        selection: OwnerBriefSelectionInput,
    ) -> AppResult<OwnerBriefPreviewDto> {
        validation::uuid(app_id, "app id")?;
        validation::uuid(change_id, "change id")?;
        if selection.scenario_ids.len() > crate::owner_workspace::MAX_SCENARIOS_PER_APP {
            return Err(validation::invalid(
                "scenario selection",
                "at most 100 scenarios can be selected; narrow the preview",
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for selected in &selection.scenario_ids {
            validation::uuid(&selected.scenario_id, "scenario id")?;
            if selected.revision == 0 {
                return Err(validation::invalid(
                    "scenario revision",
                    "must be greater than zero",
                ));
            }
            if !seen.insert((&selected.scenario_id, selected.revision)) {
                return Err(validation::invalid(
                    "scenario selection",
                    "duplicate revisions are not allowed",
                ));
            }
        }
        let preview = self.read(|conn| {
            let snapshot = brief_snapshot(conn, app_id, change_id, &selection.scenario_ids)?;
            if !selection.scenario_ids.is_empty() {
                let selected_count = selection
                    .scenario_ids
                    .iter()
                    .filter(|selected| {
                        snapshot.scenarios.iter().any(|scenario| {
                            scenario.id == selected.scenario_id && scenario.revision == selected.revision
                        })
                    })
                    .count();
                if selected_count != selection.scenario_ids.len() {
                    return Err(validation::invalid(
                        "scenario selection",
                        "one or more selected revisions are unavailable; choose confirmed revisions",
                    ));
                }
            }
            Ok(render_core(&snapshot.app, &snapshot.change, &snapshot.rules, snapshot.candidate.as_ref(), snapshot.environment.as_ref(), &snapshot.scenarios, &selection))
        })?;
        if preview.markdown.len() > MAX_BRIEF_BYTES {
            return Err(validation::invalid("brief size", "preview exceeds 32 KiB; select fewer scenarios or omit candidate/environment details"));
        }
        Ok(preview)
    }
}
