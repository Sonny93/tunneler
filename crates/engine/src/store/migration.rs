use toml_edit::DocumentMut;

use super::error::StoreError;

/// The file format version this build reads and writes.
pub(crate) const CURRENT_VERSION: u32 = 1;

type MigrationStep = fn(DocumentMut) -> DocumentMut;

/// `MIGRATION_STEPS[index]` upgrades a document from version `index + 1` to
/// version `index + 2`. Version 1 is the first format, so there is no step yet.
const MIGRATION_STEPS: &[MigrationStep] = &[];

/// Upgrades a document written as `from_version` to [`CURRENT_VERSION`].
///
/// A version newer than the current one is refused, never rewritten.
pub(crate) fn migrate(document: DocumentMut, from_version: u32) -> Result<DocumentMut, StoreError> {
    let first_step = steps_from(from_version)?;
    Ok(MIGRATION_STEPS[first_step..]
        .iter()
        .fold(document, |upgraded, step| step(upgraded)))
}

fn steps_from(from_version: u32) -> Result<usize, StoreError> {
    let is_known = (1..=CURRENT_VERSION).contains(&from_version);
    if !is_known {
        return Err(StoreError::UnsupportedVersion {
            found: from_version,
            supported: CURRENT_VERSION,
        });
    }
    usize::try_from(from_version - 1).map_err(|_| StoreError::UnsupportedVersion {
        found: from_version,
        supported: CURRENT_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn should_have_one_step_per_version_after_the_first() {
        assert_eq!(MIGRATION_STEPS.len(), (CURRENT_VERSION - 1) as usize);
    }

    #[test]
    fn should_keep_document_when_it_is_already_current() {
        let document: DocumentMut = "version = 1\n".parse().unwrap();

        let migrated = migrate(document, CURRENT_VERSION).unwrap();

        assert_eq!(migrated.to_string(), "version = 1\n");
    }

    #[test]
    fn should_refuse_version_when_it_is_newer_than_current() {
        let outcome = migrate(DocumentMut::new(), CURRENT_VERSION + 1);

        assert!(matches!(
            outcome,
            Err(StoreError::UnsupportedVersion {
                found: 2,
                supported: 1
            })
        ));
    }

    #[test]
    fn should_refuse_version_when_it_is_zero() {
        let outcome = migrate(DocumentMut::new(), 0);

        assert!(matches!(
            outcome,
            Err(StoreError::UnsupportedVersion { found: 0, .. })
        ));
    }
}
