use serde::{Deserialize, Serialize};
use thiserror::Error;
use unicode_normalization_alignments::UnicodeNormalization;

pub const PORTABLE_WORKSPACE_PATH_POLICY_ID: &str = "portable-v1";
pub const PORTABLE_WORKSPACE_MAX_COMPONENT_BYTES: usize = 120;
pub const PORTABLE_WORKSPACE_MAX_PATH_BYTES: usize = 240;
pub const PORTABLE_WORKSPACE_MAX_DEPTH: usize = 32;

const FORBIDDEN_PORTABLE_CHARACTERS: [char; 8] = ['<', '>', ':', '"', '\\', '|', '?', '*'];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspacePathSafety {
    Safe,
    Unsafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspacePathPortability {
    Portable,
    NonPortable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum WorkspacePathIssue {
    AbsolutePath,
    EmptyPath,
    EmptyComponent,
    CurrentDirectoryComponent,
    ParentDirectoryComponent,
    NullByte,
    ControlCharacter,
    ForbiddenCharacter { character: char },
    TrailingSpaceOrPeriod,
    ReservedDeviceName,
    ReservedInternalName,
    ComponentTooLong { bytes: usize, maximum: usize },
    PathTooLong { bytes: usize, maximum: usize },
    TooDeep { depth: usize, maximum: usize },
    NotUnicodeNfc,
}

impl WorkspacePathIssue {
    pub fn makes_path_unsafe(&self) -> bool {
        matches!(
            self,
            Self::AbsolutePath
                | Self::EmptyPath
                | Self::EmptyComponent
                | Self::CurrentDirectoryComponent
                | Self::ParentDirectoryComponent
                | Self::NullByte
                | Self::ReservedInternalName
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePathCollisionKeys {
    /// Canonically normalized, case-preserving key.
    pub normalized: String,
    /// Canonically normalized, Unicode-lowercased key for case-insensitive hosts.
    pub case_folded: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePathAssessment {
    pub policy_id: String,
    pub normalized_relative_path: String,
    pub safety: WorkspacePathSafety,
    pub portability: WorkspacePathPortability,
    pub issues: Vec<WorkspacePathIssue>,
    pub collision_keys: WorkspacePathCollisionKeys,
}

impl WorkspacePathAssessment {
    pub fn is_safe_to_read(&self) -> bool {
        self.safety == WorkspacePathSafety::Safe
    }

    pub fn is_portable(&self) -> bool {
        self.portability == WorkspacePathPortability::Portable
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("workspace path is not valid for creation")]
pub struct WorkspacePathValidationError {
    /// Boxed so the error variant stays small. `WorkspacePathAssessment` carries
    /// two `String`s, a `Vec`, and the collision keys; inline, every `Result`
    /// from this module would be sized for the failure case rather than the
    /// success case it almost always returns.
    pub assessment: Box<WorkspacePathAssessment>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PortableWorkspacePathPolicy;

impl PortableWorkspacePathPolicy {
    pub const fn new() -> Self {
        Self
    }

    /// Assess a root-relative path already obtained from a trusted workspace
    /// root. Nonportable host-valid names remain readable, while structural
    /// escapes and MindVault-reserved names are unsafe.
    pub fn assess_existing(&self, relative_path: &str) -> WorkspacePathAssessment {
        let normalized_relative_path = relative_path.nfc().map(|item| item.0).collect::<String>();
        let mut issues = Vec::new();

        if relative_path.is_empty() {
            issues.push(WorkspacePathIssue::EmptyPath);
        }
        if relative_path.starts_with('/') {
            issues.push(WorkspacePathIssue::AbsolutePath);
        }
        if relative_path.as_bytes().contains(&0) {
            issues.push(WorkspacePathIssue::NullByte);
        }
        if normalized_relative_path != relative_path {
            issues.push(WorkspacePathIssue::NotUnicodeNfc);
        }

        let components = relative_path.split('/').collect::<Vec<_>>();
        if components.len() > PORTABLE_WORKSPACE_MAX_DEPTH {
            issues.push(WorkspacePathIssue::TooDeep {
                depth: components.len(),
                maximum: PORTABLE_WORKSPACE_MAX_DEPTH,
            });
        }
        if normalized_relative_path.len() > PORTABLE_WORKSPACE_MAX_PATH_BYTES {
            issues.push(WorkspacePathIssue::PathTooLong {
                bytes: normalized_relative_path.len(),
                maximum: PORTABLE_WORKSPACE_MAX_PATH_BYTES,
            });
        }

        for component in components {
            assess_component(component, &mut issues);
        }

        let safety = if issues.iter().any(WorkspacePathIssue::makes_path_unsafe) {
            WorkspacePathSafety::Unsafe
        } else {
            WorkspacePathSafety::Safe
        };
        let portability = if issues.is_empty() {
            WorkspacePathPortability::Portable
        } else {
            WorkspacePathPortability::NonPortable
        };
        let case_folded = normalized_relative_path
            .chars()
            .flat_map(char::to_lowercase)
            .nfc()
            .map(|item| item.0)
            .collect();

        WorkspacePathAssessment {
            policy_id: PORTABLE_WORKSPACE_PATH_POLICY_ID.to_string(),
            collision_keys: WorkspacePathCollisionKeys {
                normalized: normalized_relative_path.clone(),
                case_folded,
            },
            normalized_relative_path,
            safety,
            portability,
            issues,
        }
    }

    /// Validate a new folder or file path against the full portable contract.
    pub fn validate_for_creation(
        &self,
        relative_path: &str,
    ) -> Result<WorkspacePathAssessment, WorkspacePathValidationError> {
        let assessment = self.assess_existing(relative_path);
        if assessment.is_safe_to_read() && assessment.is_portable() {
            Ok(assessment)
        } else {
            Err(WorkspacePathValidationError {
                assessment: Box::new(assessment),
            })
        }
    }

    pub fn has_supported_markdown_extension(&self, relative_path: &str) -> bool {
        relative_path
            .rsplit_once('.')
            .map(|(_, extension)| {
                extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
            })
            .unwrap_or(false)
    }
}

fn assess_component(component: &str, issues: &mut Vec<WorkspacePathIssue>) {
    match component {
        "" => issues.push(WorkspacePathIssue::EmptyComponent),
        "." => issues.push(WorkspacePathIssue::CurrentDirectoryComponent),
        ".." => issues.push(WorkspacePathIssue::ParentDirectoryComponent),
        _ => {}
    }

    let normalized = component.nfc().map(|item| item.0).collect::<String>();
    if normalized.len() > PORTABLE_WORKSPACE_MAX_COMPONENT_BYTES {
        issues.push(WorkspacePathIssue::ComponentTooLong {
            bytes: normalized.len(),
            maximum: PORTABLE_WORKSPACE_MAX_COMPONENT_BYTES,
        });
    }
    if component.ends_with([' ', '.']) {
        issues.push(WorkspacePathIssue::TrailingSpaceOrPeriod);
    }

    let lowercase = component.to_ascii_lowercase();
    if lowercase == ".mindvault" || lowercase.starts_with(".mindvault-") {
        issues.push(WorkspacePathIssue::ReservedInternalName);
    }

    let device_stem = component
        .split_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(component)
        .to_ascii_uppercase();
    if is_reserved_windows_device_name(&device_stem) {
        issues.push(WorkspacePathIssue::ReservedDeviceName);
    }

    for character in component.chars() {
        if character == '\0' {
            continue;
        }
        if character.is_ascii_control() {
            issues.push(WorkspacePathIssue::ControlCharacter);
        } else if FORBIDDEN_PORTABLE_CHARACTERS.contains(&character) {
            issues.push(WorkspacePathIssue::ForbiddenCharacter { character });
        }
    }
}

fn is_reserved_windows_device_name(value: &str) -> bool {
    matches!(value, "CON" | "PRN" | "AUX" | "NUL")
        || matches!(
            value.strip_prefix("COM"),
            Some("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        )
        || matches!(
            value.strip_prefix("LPT"),
            Some("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct PortablePathFixture {
        path: String,
        safe_to_read: bool,
        portable: bool,
        normalized_path: String,
    }

    #[test]
    fn portable_path_passes_creation_validation() {
        let assessment = PortableWorkspacePathPolicy::new()
            .validate_for_creation("Projects/MindVault/Architecture.md")
            .unwrap();

        assert!(assessment.is_safe_to_read());
        assert!(assessment.is_portable());
    }

    #[test]
    fn existing_nonportable_path_is_safe_but_warned() {
        let assessment =
            PortableWorkspacePathPolicy::new().assess_existing("Research/CON.notes.md");

        assert!(assessment.is_safe_to_read());
        assert!(!assessment.is_portable());
        assert!(assessment
            .issues
            .contains(&WorkspacePathIssue::ReservedDeviceName));
    }

    #[test]
    fn traversal_and_internal_paths_are_unsafe() {
        let policy = PortableWorkspacePathPolicy::new();

        assert!(!policy.assess_existing("../escape.md").is_safe_to_read());
        assert!(!policy
            .assess_existing(".mindvault/index.md")
            .is_safe_to_read());
    }

    #[test]
    fn normalization_and_case_keys_detect_portable_collisions() {
        let policy = PortableWorkspacePathPolicy::new();
        let decomposed = policy.assess_existing("Cafe\u{301}.md");
        let composed = policy.assess_existing("Caf\u{e9}.md");
        let uppercase = policy.assess_existing("CAFÉ.md");

        assert_eq!(
            decomposed.collision_keys.normalized,
            composed.collision_keys.normalized
        );
        assert_eq!(
            composed.collision_keys.case_folded,
            uppercase.collision_keys.case_folded
        );
        assert!(decomposed
            .issues
            .contains(&WorkspacePathIssue::NotUnicodeNfc));
    }

    #[test]
    fn markdown_extensions_support_existing_markdown_files() {
        let policy = PortableWorkspacePathPolicy::new();

        assert!(policy.has_supported_markdown_extension("note.md"));
        assert!(policy.has_supported_markdown_extension("note.MARKDOWN"));
        assert!(!policy.has_supported_markdown_extension("note.txt"));
    }

    #[test]
    fn portable_path_contract_fixture_matches_the_runtime_policy() {
        let fixtures: Vec<PortablePathFixture> = serde_json::from_str(include_str!(
            "../../../docs/architecture/fixtures/knowledge-workspace/portable-path-cases.json"
        ))
        .unwrap();
        let policy = PortableWorkspacePathPolicy::new();

        for fixture in fixtures {
            let assessment = policy.assess_existing(&fixture.path);
            assert_eq!(
                assessment.is_safe_to_read(),
                fixture.safe_to_read,
                "{} safety",
                fixture.path
            );
            assert_eq!(
                assessment.is_portable(),
                fixture.portable,
                "{} portability",
                fixture.path
            );
            assert_eq!(
                assessment.normalized_relative_path, fixture.normalized_path,
                "{} normalization",
                fixture.path
            );
        }
    }
}
