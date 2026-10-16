use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::ast::{ConfigFile, ConfigLine, DirectiveLine, LineContent, parse_source};
use crate::criteria::{Criterion, CriterionKind};
use crate::directive::{
    CanonicalizeHostname, Keyword, KeywordClass, ParsedDirective, classify_keyword, parse_directive,
};
use crate::environment::LocalEnvironment;
use crate::error::{Error, Origin, TokenError, Warning, WarningKind};
use crate::include::{ConfigScope, IncludeStack, find_included_files};
use crate::pattern::{host_patterns_apply, match_hostname_list, match_pattern_list};
use crate::sources::ConfigSources;
use crate::tokens::{
    FULL_TOKENS, HOSTNAME_TOKENS, TokenValues, expand_tokens, expand_tokens_and_environment,
};

use super::DEFAULT_SSH_PORT;
use super::options::Options;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pass {
    First,
    Final,
}

#[derive(Debug, Clone, Copy)]
struct FileContext {
    scope: ConfigScope,
    never_match: bool,
}

/// Evaluates configuration files in order against one alias, the way OpenSSH
/// `readconf.c` does, accumulating what the active directives set.
pub(crate) struct Walker<'environment> {
    environment: &'environment dyn LocalEnvironment,
    alias: String,
    host: String,
    pass: Pass,
    is_active: bool,
    wants_final_pass: bool,
    include_stack: IncludeStack,
    parsed_files: HashMap<PathBuf, Rc<ConfigFile>>,
    pub(crate) options: Options,
    pub(crate) warnings: Vec<Warning>,
}

impl<'environment> Walker<'environment> {
    pub(crate) fn new(environment: &'environment dyn LocalEnvironment, alias: &str) -> Self {
        Self {
            environment,
            alias: alias.to_owned(),
            host: alias.to_owned(),
            pass: Pass::First,
            is_active: true,
            wants_final_pass: false,
            include_stack: IncludeStack::default(),
            parsed_files: HashMap::new(),
            options: Options::default(),
            warnings: Vec::new(),
        }
    }

    pub(crate) fn wants_final_pass(&self) -> bool {
        self.wants_final_pass
    }

    /// Walks the user file then the system file.
    pub(crate) fn walk_sources(&mut self, sources: &ConfigSources) -> Result<(), Error> {
        if let Some(user_path) = &sources.user {
            self.walk_top_level(user_path, ConfigScope::User)?;
        }
        if let Some(system_path) = &sources.system {
            self.walk_top_level(system_path, ConfigScope::System)?;
        }
        Ok(())
    }

    /// The host name once the first pass is done: `Hostname` with `%h`
    /// expanded, or the alias, lowercased unless it is an address.
    pub(crate) fn settled_host(&self) -> Result<String, Error> {
        let host = match self.options.text(Keyword::Hostname) {
            Some(hostname) => self.expand_hostname(&hostname.value, &hostname.origin)?,
            None => self.alias.clone(),
        };
        Ok(lowercase_unless_address(host))
    }

    /// Prepares the second walk, where `Host` lines match the settled host.
    pub(crate) fn begin_final_pass(&mut self, settled_host: String) {
        self.pass = Pass::Final;
        self.host = settled_host;
    }

    fn expand_hostname(&self, template: &str, origin: &Origin) -> Result<String, Error> {
        let values = TokenValues {
            remote_host: self.host.clone(),
            ..TokenValues::default()
        };
        expand_tokens(template, HOSTNAME_TOKENS, &values).map_err(|source| Error::Token {
            origin: origin.clone(),
            source,
        })
    }

    fn walk_top_level(&mut self, path: &Path, scope: ConfigScope) -> Result<(), Error> {
        let Some(file) = self.load_file(path)? else {
            return Ok(());
        };
        self.include_stack.push_root(path);
        self.is_active = true;
        let context = FileContext {
            scope,
            never_match: false,
        };
        let outcome = self.walk_lines(&file, context);
        self.include_stack.leave();
        outcome
    }

    fn load_file(&mut self, path: &Path) -> Result<Option<Rc<ConfigFile>>, Error> {
        if let Some(cached) = self.parsed_files.get(path) {
            return Ok(Some(Rc::clone(cached)));
        }
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(Error::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        let file = Rc::new(parse_source(path, &text)?);
        self.parsed_files
            .insert(path.to_path_buf(), Rc::clone(&file));
        Ok(Some(file))
    }

    fn walk_lines(&mut self, file: &ConfigFile, context: FileContext) -> Result<(), Error> {
        for line in &file.lines {
            self.process_line(file, line, context)?;
        }
        Ok(())
    }

    fn process_line(
        &mut self,
        file: &ConfigFile,
        line: &ConfigLine,
        context: FileContext,
    ) -> Result<(), Error> {
        let origin = Origin::new(&file.path, line.number);
        match &line.content {
            LineContent::Host { patterns } => {
                self.is_active = !context.never_match && host_patterns_apply(patterns, &self.host);
                Ok(())
            }
            LineContent::Match { criteria } => self.process_match(criteria, &origin, context),
            LineContent::Directive(directive) => {
                self.process_directive(directive, &origin, context)
            }
        }
    }

    fn push_warning(&mut self, origin: &Origin, kind: WarningKind) {
        let warning = Warning {
            path: origin.path.to_path_buf(),
            line: origin.line,
            kind,
        };
        if !self.warnings.contains(&warning) {
            self.warnings.push(warning);
        }
    }

    fn process_match(
        &mut self,
        criteria: &[Criterion],
        origin: &Origin,
        context: FileContext,
    ) -> Result<(), Error> {
        self.warn_about_unsupported_criteria(criteria, origin);
        let asks_for_final_pass = criteria
            .iter()
            .any(|criterion| criterion.kind == CriterionKind::Final);
        if asks_for_final_pass && self.pass == Pass::First {
            self.wants_final_pass = true;
        }
        self.is_active = !context.never_match && self.criteria_apply(criteria, origin)?;
        Ok(())
    }

    fn warn_about_unsupported_criteria(&mut self, criteria: &[Criterion], origin: &Origin) {
        for criterion in criteria {
            if let CriterionKind::Unsupported(name) = &criterion.kind {
                let kind = WarningKind::UnsupportedCriterion {
                    criterion: name.clone(),
                };
                self.push_warning(origin, kind);
            }
        }
    }

    fn criteria_apply(&self, criteria: &[Criterion], origin: &Origin) -> Result<bool, Error> {
        let host = self.current_host()?;
        let mut applies = true;
        for criterion in criteria {
            if let CriterionKind::Unsupported(_) = criterion.kind {
                return Ok(false);
            }
            let is_matched = self.criterion_matches(&criterion.kind, &host, applies, origin)?;
            if is_matched == criterion.is_negated {
                applies = false;
            }
        }
        Ok(applies)
    }

    fn criterion_matches(
        &self,
        kind: &CriterionKind,
        host: &str,
        should_run_commands: bool,
        origin: &Origin,
    ) -> Result<bool, Error> {
        Ok(match kind {
            CriterionKind::All => true,
            CriterionKind::Canonical | CriterionKind::Final => self.pass == Pass::Final,
            CriterionKind::Host(list) => match_hostname_list(host, list).is_positive(),
            CriterionKind::OriginalHost(list) => {
                match_hostname_list(&self.alias, list).is_positive()
            }
            CriterionKind::User(list) => {
                match_pattern_list(&self.remote_user_so_far(), list).is_positive()
            }
            CriterionKind::LocalUser(list) => {
                match_pattern_list(&self.environment.user_name(), list).is_positive()
            }
            CriterionKind::Tagged(list) => self.tag_matches(list),
            CriterionKind::Exec(command) => {
                self.run_exec(command, host, should_run_commands, origin)?
            }
            CriterionKind::Unsupported(_) => false,
        })
    }

    fn tag_matches(&self, list: &str) -> bool {
        let tag = self
            .options
            .text(Keyword::Tag)
            .map_or("", |tag| tag.value.as_str());
        if tag.is_empty() {
            return list.is_empty();
        }
        match_pattern_list(tag, list).is_positive()
    }

    fn run_exec(
        &self,
        command: &str,
        host: &str,
        should_run: bool,
        origin: &Origin,
    ) -> Result<bool, Error> {
        let values = self.walk_token_values(host);
        let expanded =
            expand_tokens(command, FULL_TOKENS, &values).map_err(|source| Error::Token {
                origin: origin.clone(),
                source,
            })?;
        if !should_run {
            return Ok(false);
        }
        self.environment
            .command_succeeds(&expanded)
            .map_err(|source| Error::MatchExec {
                origin: origin.clone(),
                command: expanded,
                source,
            })
    }

    fn remote_user_so_far(&self) -> String {
        self.options
            .text(Keyword::User)
            .map_or_else(|| self.environment.user_name(), |user| user.value.clone())
    }

    /// The host as `Match host` and `Include` see it at this point.
    fn current_host(&self) -> Result<String, Error> {
        if self.pass == Pass::Final {
            return Ok(self.host.clone());
        }
        match self.options.text(Keyword::Hostname) {
            Some(hostname) => self.expand_hostname(&hostname.value, &hostname.origin),
            None => Ok(self.host.clone()),
        }
    }

    fn walk_token_values(&self, host: &str) -> TokenValues {
        let key_alias = self
            .options
            .text(Keyword::HostKeyAlias)
            .map_or_else(|| host.to_owned(), |alias| alias.value.clone());
        TokenValues {
            remote_host: host.to_owned(),
            port: self.options.port.unwrap_or(DEFAULT_SSH_PORT).to_string(),
            key_alias,
            jump_host: self.options.jump_host_name(),
            original_host: self.alias.clone(),
            ..TokenValues::from_environment(self.environment)
        }
        .with_remote_user(self.remote_user_so_far())
    }

    fn process_directive(
        &mut self,
        directive: &DirectiveLine,
        origin: &Origin,
        context: FileContext,
    ) -> Result<(), Error> {
        match classify_keyword(&directive.keyword) {
            KeywordClass::Unknown => {
                self.handle_unknown_keyword(directive, origin);
                Ok(())
            }
            KeywordClass::KnownUnmodeled => {
                self.keep_raw_directive(directive);
                Ok(())
            }
            KeywordClass::Modeled(keyword) => {
                self.process_modeled_directive(keyword, directive, origin, context)
            }
        }
    }

    fn process_modeled_directive(
        &mut self,
        keyword: Keyword,
        directive: &DirectiveLine,
        origin: &Origin,
        context: FileContext,
    ) -> Result<(), Error> {
        let parsed = parse_directive(
            keyword,
            &directive.arguments,
            &directive.raw_value,
            self.environment,
        )
        .map_err(|kind| Error::Value {
            origin: origin.clone(),
            keyword: directive.keyword.clone(),
            kind,
        })?;
        self.apply_parsed(parsed, origin, context)
    }

    fn handle_unknown_keyword(&mut self, directive: &DirectiveLine, origin: &Origin) {
        let is_ignored = self
            .options
            .text(Keyword::IgnoreUnknown)
            .is_some_and(|patterns| {
                match_hostname_list(&directive.keyword, &patterns.value).is_positive()
            });
        if is_ignored {
            return;
        }
        let kind = WarningKind::UnknownKeyword {
            keyword: directive.keyword.clone(),
        };
        self.push_warning(origin, kind);
    }

    fn keep_raw_directive(&mut self, directive: &DirectiveLine) {
        if !self.is_active {
            return;
        }
        self.options
            .other
            .entry(directive.keyword.clone())
            .or_insert_with(|| directive.arguments.clone());
    }

    fn apply_parsed(
        &mut self,
        parsed: ParsedDirective,
        origin: &Origin,
        context: FileContext,
    ) -> Result<(), Error> {
        match parsed {
            ParsedDirective::Include(patterns) => self.process_includes(&patterns, origin, context),
            ParsedDirective::CanonicalizeHostname(value) => {
                self.warn_about_canonicalization(value, origin);
                Ok(())
            }
            other => {
                if self.is_active {
                    self.options.apply(other, origin, context.scope);
                }
                Ok(())
            }
        }
    }

    fn warn_about_canonicalization(&mut self, value: CanonicalizeHostname, origin: &Origin) {
        if !self.is_active || value == CanonicalizeHostname::No {
            return;
        }
        let kind = WarningKind::UnsupportedCanonicalization {
            value: format!("{value:?}").to_ascii_lowercase(),
        };
        self.push_warning(origin, kind);
    }

    fn process_includes(
        &mut self,
        patterns: &[String],
        origin: &Origin,
        context: FileContext,
    ) -> Result<(), Error> {
        for pattern in patterns {
            let Some(expanded) = self.expand_include_pattern(pattern, origin)? else {
                continue;
            };
            let files = find_included_files(&expanded, context.scope, self.environment, origin)?;
            self.walk_included_files(&files, origin, context)?;
        }
        Ok(())
    }

    fn expand_include_pattern(
        &mut self,
        pattern: &str,
        origin: &Origin,
    ) -> Result<Option<String>, Error> {
        let host = self.current_host()?;
        let values = self.walk_token_values(&host);
        match expand_tokens_and_environment(pattern, FULL_TOKENS, &values, self.environment) {
            Ok(expanded) => Ok(Some(expanded)),
            Err(reason @ TokenError::UndefinedEnvironmentVariable { .. }) => {
                let kind = WarningKind::IncludeSkipped {
                    pattern: pattern.to_owned(),
                    reason,
                };
                self.push_warning(origin, kind);
                Ok(None)
            }
            Err(source) => Err(Error::Token {
                origin: origin.clone(),
                source,
            }),
        }
    }

    fn walk_included_files(
        &mut self,
        files: &[PathBuf],
        origin: &Origin,
        context: FileContext,
    ) -> Result<(), Error> {
        let was_active = self.is_active;
        let included_context = FileContext {
            never_match: context.never_match || !was_active,
            ..context
        };
        for path in files {
            self.include_stack.enter_include(path, origin)?;
            let outcome = self.walk_included_file(path, included_context);
            self.include_stack.leave();
            outcome?;
            self.is_active = was_active;
        }
        Ok(())
    }

    fn walk_included_file(&mut self, path: &Path, context: FileContext) -> Result<(), Error> {
        match self.load_file(path)? {
            Some(file) => self.walk_lines(&file, context),
            None => Ok(()),
        }
    }
}

fn lowercase_unless_address(host: String) -> String {
    if host.parse::<IpAddr>().is_ok() {
        return host;
    }
    host.to_ascii_lowercase()
}
