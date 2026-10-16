use std::fs;
use std::path::{Path, PathBuf};

use glob::MatchOptions;

use crate::environment::LocalEnvironment;
use crate::error::{Error, Origin};
use crate::tokens::expand_tilde;

/// Maximum nesting of `Include` (OpenSSH `READCONF_MAX_DEPTH`).
pub(crate) const INCLUDE_DEPTH_LIMIT: usize = 16;

const HOME_MARKER: char = '~';
const USER_CONFIG_DIRECTORY: &str = ".ssh";

const GLOB_OPTIONS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: true,
};

/// Which top-level configuration an `Include` was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigScope {
    User,
    System,
}

/// The files currently being read, outermost first, as canonical paths.
#[derive(Debug, Default)]
pub(crate) struct IncludeStack {
    chain: Vec<PathBuf>,
}

impl IncludeStack {
    pub(crate) fn push_root(&mut self, path: &Path) {
        self.chain.push(canonical_or_original(path));
    }

    /// Enters an included file, failing when it is too deep or already open.
    pub(crate) fn enter_include(&mut self, path: &Path, origin: &Origin) -> Result<(), Error> {
        let canonical = canonical_or_original(path);
        if self.chain.contains(&canonical) {
            return Err(Error::IncludeCycle {
                origin: origin.clone(),
                chain: self.chain_with(canonical),
            });
        }
        if self.chain.len() > INCLUDE_DEPTH_LIMIT {
            return Err(Error::IncludeDepthExceeded {
                origin: origin.clone(),
                limit: INCLUDE_DEPTH_LIMIT,
                chain: self.chain_with(canonical),
            });
        }
        self.chain.push(canonical);
        Ok(())
    }

    pub(crate) fn leave(&mut self) {
        self.chain.pop();
    }

    fn chain_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut chain = self.chain.clone();
        chain.push(path);
        chain
    }
}

fn canonical_or_original(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Finds the files an already token-expanded `Include` argument designates.
///
/// Relative paths live in `~/.ssh` for the user configuration and in the system
/// configuration directory for the system one. Results are sorted and
/// directories are skipped. A pattern matching nothing yields no files.
pub(crate) fn find_included_files(
    expanded_pattern: &str,
    scope: ConfigScope,
    environment: &dyn LocalEnvironment,
    origin: &Origin,
) -> Result<Vec<PathBuf>, Error> {
    if scope == ConfigScope::System && expanded_pattern.starts_with(HOME_MARKER) {
        return Err(Error::InvalidIncludePath {
            origin: origin.clone(),
            path: expanded_pattern.to_owned(),
        });
    }
    let anchored = anchor_pattern(expanded_pattern, scope, environment);
    let home_directory = environment.home_directory().to_string_lossy().into_owned();
    let glob_pattern = expand_tilde(&anchored, &home_directory);
    let paths = glob::glob_with(&glob_pattern, GLOB_OPTIONS).map_err(|source| {
        Error::InvalidIncludePattern {
            origin: origin.clone(),
            pattern: glob_pattern.clone(),
            source,
        }
    })?;
    let mut files = Vec::new();
    for entry in paths {
        let path = entry.map_err(|error| Error::Io {
            path: error.path().to_path_buf(),
            source: error.into(),
        })?;
        if !path.is_dir() {
            files.push(path);
        }
    }
    Ok(files)
}

fn anchor_pattern(pattern: &str, scope: ConfigScope, environment: &dyn LocalEnvironment) -> String {
    if Path::new(pattern).is_absolute() || pattern.starts_with(HOME_MARKER) {
        return pattern.to_owned();
    }
    let base_directory = match scope {
        ConfigScope::User => environment.home_directory().join(USER_CONFIG_DIRECTORY),
        ConfigScope::System => environment.system_config_directory(),
    };
    base_directory.join(pattern).to_string_lossy().into_owned()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::environment::fake::FakeEnvironment;

    fn origin() -> Origin {
        Origin::new(Path::new("main"), 1)
    }

    fn environment_in(directory: &Path) -> FakeEnvironment {
        FakeEnvironment {
            home_directory: directory.join("home"),
            system_directory: directory.join("etc"),
            ..FakeEnvironment::default()
        }
    }

    fn touch(path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, "")
    }

    #[test]
    fn should_resolve_relative_pattern_in_user_ssh_directory_when_scope_is_user() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("home/.ssh/extra.conf")).unwrap();
        let files = find_included_files(
            "extra.conf",
            ConfigScope::User,
            &environment_in(directory.path()),
            &origin(),
        )
        .unwrap();
        assert_eq!(files, vec![directory.path().join("home/.ssh/extra.conf")]);
    }

    #[test]
    fn should_resolve_relative_pattern_in_system_directory_when_scope_is_system() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("etc/extra.conf")).unwrap();
        let files = find_included_files(
            "extra.conf",
            ConfigScope::System,
            &environment_in(directory.path()),
            &origin(),
        )
        .unwrap();
        assert_eq!(files, vec![directory.path().join("etc/extra.conf")]);
    }

    #[test]
    fn should_keep_absolute_pattern_when_it_is_absolute() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("elsewhere/x.conf")).unwrap();
        let pattern = directory.path().join("elsewhere/x.conf");
        let files = find_included_files(
            pattern.to_str().unwrap(),
            ConfigScope::User,
            &environment_in(directory.path()),
            &origin(),
        )
        .unwrap();
        assert_eq!(files, vec![pattern]);
    }

    #[test]
    fn should_expand_tilde_when_pattern_starts_with_home_marker() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("home/conf.d/a.conf")).unwrap();
        let files = find_included_files(
            "~/conf.d/a.conf",
            ConfigScope::User,
            &environment_in(directory.path()),
            &origin(),
        )
        .unwrap();
        assert_eq!(files, vec![directory.path().join("home/conf.d/a.conf")]);
    }

    #[test]
    fn should_return_sorted_files_when_pattern_has_a_wildcard() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("home/.ssh/conf.d/b.conf")).unwrap();
        touch(&directory.path().join("home/.ssh/conf.d/a.conf")).unwrap();
        touch(&directory.path().join("home/.ssh/conf.d/c.txt")).unwrap();
        let files = find_included_files(
            "conf.d/*.conf",
            ConfigScope::User,
            &environment_in(directory.path()),
            &origin(),
        )
        .unwrap();
        assert_eq!(
            files,
            vec![
                directory.path().join("home/.ssh/conf.d/a.conf"),
                directory.path().join("home/.ssh/conf.d/b.conf")
            ]
        );
    }

    #[test]
    fn should_skip_directories_when_wildcard_matches_them() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("home/.ssh/conf.d/sub")).unwrap();
        let files = find_included_files(
            "conf.d/*",
            ConfigScope::User,
            &environment_in(directory.path()),
            &origin(),
        )
        .unwrap();
        assert_eq!(files, Vec::<PathBuf>::new());
    }

    #[test]
    fn should_return_nothing_when_pattern_matches_no_file() {
        let directory = tempfile::tempdir().unwrap();
        let files = find_included_files(
            "missing/*.conf",
            ConfigScope::User,
            &environment_in(directory.path()),
            &origin(),
        )
        .unwrap();
        assert_eq!(files, Vec::<PathBuf>::new());
    }

    #[test]
    fn should_fail_when_system_include_starts_with_tilde() {
        let directory = tempfile::tempdir().unwrap();
        let result = find_included_files(
            "~/x",
            ConfigScope::System,
            &environment_in(directory.path()),
            &origin(),
        );
        assert!(matches!(result, Err(Error::InvalidIncludePath { .. })));
    }

    #[test]
    fn should_fail_when_glob_pattern_is_malformed() {
        let directory = tempfile::tempdir().unwrap();
        let result = find_included_files(
            "[",
            ConfigScope::User,
            &environment_in(directory.path()),
            &origin(),
        );
        assert!(matches!(result, Err(Error::InvalidIncludePattern { .. })));
    }

    #[test]
    fn should_detect_cycle_when_file_is_entered_twice() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("a.conf");
        touch(&file).unwrap();
        let mut stack = IncludeStack::default();
        stack.push_root(&file);
        let result = stack.enter_include(&file, &origin());
        assert!(matches!(result, Err(Error::IncludeCycle { chain, .. }) if chain.len() == 2));
    }

    #[test]
    fn should_fail_when_include_depth_exceeds_the_limit() {
        let directory = tempfile::tempdir().unwrap();
        let mut stack = IncludeStack::default();
        stack.push_root(&directory.path().join("root"));
        for level in 0..INCLUDE_DEPTH_LIMIT {
            let file = directory.path().join(format!("level{level}"));
            stack.enter_include(&file, &origin()).unwrap();
        }
        let result = stack.enter_include(&directory.path().join("too_deep"), &origin());
        assert!(matches!(
            result,
            Err(Error::IncludeDepthExceeded { chain, .. }) if chain.len() == INCLUDE_DEPTH_LIMIT + 2
        ));
    }

    #[test]
    fn should_allow_reentering_file_when_it_was_left() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("a.conf");
        let mut stack = IncludeStack::default();
        stack.push_root(&directory.path().join("root"));
        stack.enter_include(&file, &origin()).unwrap();
        stack.leave();
        assert!(stack.enter_include(&file, &origin()).is_ok());
    }
}
