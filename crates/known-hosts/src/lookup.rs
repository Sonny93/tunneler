use crate::key::HostKey;

pub(crate) const DEFAULT_SSH_PORT: u16 = 22;

/// The host being connected to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostLookup {
    /// The host name as typed or resolved from `HostName`.
    pub hostname: String,
    /// The TCP port.
    pub port: u16,
    /// The `HostKeyAlias`, which replaces the host name and the port.
    pub host_key_alias: Option<String>,
}

impl HostLookup {
    /// A lookup without alias.
    pub fn new(hostname: impl Into<String>, port: u16) -> Self {
        Self {
            hostname: hostname.into(),
            port,
            host_key_alias: None,
        }
    }

    /// The name under which keys are looked up and recorded, lowercased.
    ///
    /// It is the alias verbatim when one is set (OpenSSH does not add the port
    /// to an alias), otherwise the host name, as `[host]:port` when the port is
    /// not 22.
    pub fn lookup_name(&self) -> String {
        let name = match &self.host_key_alias {
            Some(alias) => alias.clone(),
            None => self.name_with_port(),
        };
        name.to_ascii_lowercase()
    }

    pub(crate) fn has_port_fallback(&self) -> bool {
        self.host_key_alias.is_none() && self.port != DEFAULT_SSH_PORT
    }

    pub(crate) fn plain_name(&self) -> String {
        self.hostname.to_ascii_lowercase()
    }

    fn name_with_port(&self) -> String {
        if self.port == DEFAULT_SSH_PORT {
            return self.hostname.clone();
        }
        format!("[{}]:{}", self.hostname, self.port)
    }
}

/// The key a server presented, once the engine has validated what it can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentedKey {
    /// A plain host key.
    Plain(HostKey),
    /// A host certificate whose signature, principals and validity the caller
    /// already verified. Only its signing authority is checked here.
    Certificate {
        /// The key of the certificate authority that signed the certificate.
        signing_authority: HostKey,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn should_use_plain_hostname_when_port_is_default() {
        assert_eq!(
            HostLookup::new("example.com", 22).lookup_name(),
            "example.com"
        );
    }

    #[test]
    fn should_bracket_hostname_when_port_is_not_default() {
        assert_eq!(
            HostLookup::new("example.com", 2222).lookup_name(),
            "[example.com]:2222"
        );
    }

    #[test]
    fn should_lowercase_name_when_hostname_has_uppercase_letters() {
        assert_eq!(
            HostLookup::new("Example.COM", 22).lookup_name(),
            "example.com"
        );
    }

    #[test]
    fn should_use_alias_without_port_when_alias_is_set() {
        let lookup = HostLookup {
            host_key_alias: Some("Jump".to_owned()),
            ..HostLookup::new("example.com", 2222)
        };

        assert_eq!(lookup.lookup_name(), "jump");
    }

    #[test]
    fn should_not_fall_back_to_plain_name_when_alias_is_set() {
        let lookup = HostLookup {
            host_key_alias: Some("jump".to_owned()),
            ..HostLookup::new("example.com", 2222)
        };

        assert!(!lookup.has_port_fallback());
    }

    #[test]
    fn should_fall_back_to_plain_name_when_port_is_not_default() {
        assert!(HostLookup::new("example.com", 2222).has_port_fallback());
    }

    #[test]
    fn should_not_fall_back_when_port_is_default() {
        assert!(!HostLookup::new("example.com", 22).has_port_fallback());
    }
}
