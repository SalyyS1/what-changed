//! Closed developer-tool allowlist and version extraction rules.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use regex::Regex;

/// One executable the developer-environment collector may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub display_name: &'static str,
    version_pattern: &'static str,
}

impl ToolDefinition {
    pub fn extract_version(self, output: &str) -> Option<String> {
        let regexes = VERSION_REGEXES.get_or_init(|| {
            TOOL_ALLOWLIST
                .iter()
                .map(|tool| Regex::new(tool.version_pattern).ok())
                .collect()
        });
        let index = TOOL_ALLOWLIST
            .iter()
            .position(|candidate| candidate.name == self.name)?;
        regexes
            .get(index)?
            .as_ref()?
            .captures(output.trim())?
            .get(1)
            .map(|capture| capture.as_str().to_owned())
    }
}

static VERSION_REGEXES: OnceLock<Vec<Option<Regex>>> = OnceLock::new();

/// Fixed names only. No IPC or user setting can extend this list.
pub const TOOL_ALLOWLIST: &[ToolDefinition] = &[
    tool("node", "Node.js", r"(?i)^v?(\d+(?:\.\d+){1,3})"),
    tool("bun", "Bun", r"(?i)^(\d+(?:\.\d+){1,3})"),
    tool("npm", "npm", r"(?i)^(\d+(?:\.\d+){1,3})"),
    tool("pnpm", "pnpm", r"(?i)^(\d+(?:\.\d+){1,3})"),
    tool("yarn", "Yarn", r"(?i)^(\d+(?:\.\d+){1,3})"),
    tool("python", "Python", r"(?i)^python\s+(\d+(?:\.\d+){1,3})"),
    tool("pip", "pip", r"(?i)^pip\s+(\d+(?:\.\d+){1,3})"),
    tool(
        "java",
        "Java",
        r#"(?i)(?:version\s+\")?([0-9]+(?:\.[0-9]+){1,3})"#,
    ),
    tool("dotnet", ".NET", r"(?i)^(\d+(?:\.\d+){1,3})"),
    tool("go", "Go", r"(?i)^go version go(\d+(?:\.\d+){1,3})"),
    tool("rustc", "Rust", r"(?i)^rustc\s+(\d+(?:\.\d+){1,3})"),
    tool("cargo", "Cargo", r"(?i)^cargo\s+(\d+(?:\.\d+){1,3})"),
    tool("git", "Git", r"(?i)^git version\s+(\d+(?:\.\d+){1,3})"),
    tool("docker", "Docker", r"(?i).*?version\s+(\d+(?:\.\d+){1,3})"),
    tool("kubectl", "kubectl", r"(?i).*?v(\d+(?:\.\d+){1,3})"),
];

const fn tool(
    name: &'static str,
    display_name: &'static str,
    version_pattern: &'static str,
) -> ToolDefinition {
    ToolDefinition {
        name,
        display_name,
        version_pattern,
    }
}

pub fn definition(name: &str) -> Option<&'static ToolDefinition> {
    TOOL_ALLOWLIST.iter().find(|tool| tool.name == name)
}

/// Split Windows PATH syntax without treating quoted semicolons as separators.
pub fn path_entries(value: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in value.chars() {
        match character {
            '"' => quoted = !quoted,
            ';' if !quoted => push_path_entry(&mut entries, &mut seen, &mut current),
            other => current.push(other),
        }
    }
    push_path_entry(&mut entries, &mut seen, &mut current);
    entries
}

fn push_path_entry(entries: &mut Vec<String>, seen: &mut BTreeSet<String>, current: &mut String) {
    let entry = current.trim().trim_matches('"');
    if !entry.is_empty() && seen.insert(entry.to_ascii_lowercase()) {
        entries.push(entry.to_owned());
    }
    current.clear();
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn extracts_representative_versions() {
        let cases = [
            ("node", "v22.6.0", "22.6.0"),
            ("python", "Python 3.12.4", "3.12.4"),
            ("go", "go version go1.23.1 windows/amd64", "1.23.1"),
            ("git", "git version 2.46.0.windows.1", "2.46.0"),
        ];
        for (name, output, expected) in cases {
            assert_eq!(
                definition(name).unwrap().extract_version(output).as_deref(),
                Some(expected)
            );
        }
    }
}
