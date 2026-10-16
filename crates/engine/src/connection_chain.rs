use crate::error::JumpChainError;

/// OpenSSH has no fixed limit on nested `ProxyJump`; this one only stops
/// runaway configurations.
pub(crate) const MAXIMUM_JUMP_DEPTH: usize = 8;

/// The aliases being connected, outermost first, used to refuse cycles and
/// runaway nesting of jump hosts that have their own `ProxyJump`.
#[derive(Debug, Clone, Default)]
pub(crate) struct ConnectionChain {
    aliases: Vec<String>,
}

impl ConnectionChain {
    pub(crate) fn enter(&self, alias: &str) -> Result<Self, JumpChainError> {
        let mut aliases = self.aliases.clone();
        aliases.push(alias.to_owned());
        if self.aliases.iter().any(|existing| existing == alias) {
            return Err(JumpChainError::Cycle { chain: aliases });
        }
        if aliases.len() > MAXIMUM_JUMP_DEPTH {
            return Err(JumpChainError::TooDeep {
                chain: aliases,
                maximum: MAXIMUM_JUMP_DEPTH,
            });
        }
        Ok(Self { aliases })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain_of(aliases: &[&str]) -> Result<ConnectionChain, JumpChainError> {
        aliases
            .iter()
            .try_fold(ConnectionChain::default(), |chain, alias| {
                chain.enter(alias)
            })
    }

    #[test]
    fn should_accept_distinct_aliases_when_entering_a_chain() {
        assert!(chain_of(&["target", "bastion"]).is_ok());
    }

    #[test]
    fn should_report_the_chain_when_an_alias_repeats() {
        assert_eq!(
            chain_of(&["a", "b", "a"]).unwrap_err(),
            JumpChainError::Cycle {
                chain: vec!["a".to_owned(), "b".to_owned(), "a".to_owned()]
            }
        );
    }

    #[test]
    fn should_accept_the_maximum_depth_when_aliases_are_distinct() {
        let aliases: Vec<String> = (0..MAXIMUM_JUMP_DEPTH).map(|n| format!("h{n}")).collect();
        let borrowed: Vec<&str> = aliases.iter().map(String::as_str).collect();
        assert!(chain_of(&borrowed).is_ok());
    }

    #[test]
    fn should_refuse_beyond_the_maximum_depth_when_aliases_are_distinct() {
        let aliases: Vec<String> = (0..=MAXIMUM_JUMP_DEPTH).map(|n| format!("h{n}")).collect();
        let borrowed: Vec<&str> = aliases.iter().map(String::as_str).collect();
        assert!(matches!(
            chain_of(&borrowed),
            Err(JumpChainError::TooDeep { maximum, .. }) if maximum == MAXIMUM_JUMP_DEPTH
        ));
    }
}
