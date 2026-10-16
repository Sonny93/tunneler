use std::fs;
use std::io;
use std::path::Path;

use crate::ast::{ConfigFile, DirectiveLine, LineContent, parse_source};
use crate::directive::{Keyword, ParsedDirective, parse_directive};
use crate::environment::LocalEnvironment;
use crate::error::{Error, Origin, TokenError};
use crate::include::{ConfigScope, IncludeStack, find_included_files};
use crate::pattern::is_concrete_host_pattern;
use crate::sources::ConfigSources;
use crate::tokens::{FULL_TOKENS, TokenValues, expand_tokens_and_environment};

use super::DEFAULT_SSH_PORT;

const INCLUDE_KEYWORD: &str = "include";

/// Lists the concrete host aliases of `Host` lines: no wildcard, no negation.
///
/// Every `Include` is followed whether or not its block is active. Aliases are
/// deduplicated and returned in the order they are first seen, user file first.
/// Tokens in `Include` paths cannot refer to a host here and expand to empty
/// text.
pub fn list_hosts(
    sources: &ConfigSources,
    environment: &impl LocalEnvironment,
) -> Result<Vec<String>, Error> {
    let mut collector = HostCollector {
        environment,
        include_stack: IncludeStack::default(),
        aliases: Vec::new(),
    };
    if let Some(user_path) = &sources.user {
        collector.collect_top_level(user_path, ConfigScope::User)?;
    }
    if let Some(system_path) = &sources.system {
        collector.collect_top_level(system_path, ConfigScope::System)?;
    }
    Ok(collector.aliases)
}

struct HostCollector<'environment> {
    environment: &'environment dyn LocalEnvironment,
    include_stack: IncludeStack,
    aliases: Vec<String>,
}

impl HostCollector<'_> {
    fn collect_top_level(&mut self, path: &Path, scope: ConfigScope) -> Result<(), Error> {
        let Some(file) = read_config_file(path)? else {
            return Ok(());
        };
        self.include_stack.push_root(path);
        let outcome = self.collect_file(&file, scope);
        self.include_stack.leave();
        outcome
    }

    fn collect_file(&mut self, file: &ConfigFile, scope: ConfigScope) -> Result<(), Error> {
        for line in &file.lines {
            match &line.content {
                LineContent::Host { patterns } => self.add_aliases(patterns),
                LineContent::Directive(directive) if directive.keyword == INCLUDE_KEYWORD => {
                    let origin = Origin::new(&file.path, line.number);
                    self.collect_includes(directive, &origin, scope)?;
                }
                LineContent::Match { .. } | LineContent::Directive(_) => {}
            }
        }
        Ok(())
    }

    fn add_aliases(&mut self, patterns: &[String]) {
        for pattern in patterns
            .iter()
            .filter(|pattern| is_concrete_host_pattern(pattern))
        {
            if !self.aliases.contains(pattern) {
                self.aliases.push(pattern.clone());
            }
        }
    }

    fn collect_includes(
        &mut self,
        directive: &DirectiveLine,
        origin: &Origin,
        scope: ConfigScope,
    ) -> Result<(), Error> {
        let parsed = parse_directive(
            Keyword::Include,
            &directive.arguments,
            &directive.raw_value,
            self.environment,
        )
        .map_err(|kind| Error::Value {
            origin: origin.clone(),
            keyword: directive.keyword.clone(),
            kind,
        })?;
        let ParsedDirective::Include(patterns) = parsed else {
            return Ok(());
        };
        for pattern in &patterns {
            let Some(expanded) = self.expand_pattern(pattern, origin)? else {
                continue;
            };
            let files = find_included_files(&expanded, scope, self.environment, origin)?;
            self.collect_included_files(&files, origin, scope)?;
        }
        Ok(())
    }

    fn expand_pattern(&self, pattern: &str, origin: &Origin) -> Result<Option<String>, Error> {
        let values = TokenValues {
            port: DEFAULT_SSH_PORT.to_string(),
            ..TokenValues::from_environment(self.environment)
        }
        .with_remote_user(self.environment.user_name());
        match expand_tokens_and_environment(pattern, FULL_TOKENS, &values, self.environment) {
            Ok(expanded) => Ok(Some(expanded)),
            Err(TokenError::UndefinedEnvironmentVariable { .. }) => Ok(None),
            Err(source) => Err(Error::Token {
                origin: origin.clone(),
                source,
            }),
        }
    }

    fn collect_included_files(
        &mut self,
        files: &[std::path::PathBuf],
        origin: &Origin,
        scope: ConfigScope,
    ) -> Result<(), Error> {
        for path in files {
            self.include_stack.enter_include(path, origin)?;
            let outcome = match read_config_file(path)? {
                Some(file) => self.collect_file(&file, scope),
                None => Ok(()),
            };
            self.include_stack.leave();
            outcome?;
        }
        Ok(())
    }
}

fn read_config_file(path: &Path) -> Result<Option<ConfigFile>, Error> {
    match fs::read_to_string(path) {
        Ok(text) => parse_source(path, &text).map(Some),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}
