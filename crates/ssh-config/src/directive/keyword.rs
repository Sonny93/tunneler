/// The directives this crate models with typed values.
#[allow(
    missing_docs,
    reason = "each variant is named after the ssh_config keyword it models"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    AddKeysToAgent,
    AddressFamily,
    BatchMode,
    CanonicalizeHostname,
    CertificateFile,
    ClearAllForwardings,
    Compression,
    ConnectTimeout,
    ConnectionAttempts,
    DynamicForward,
    ExitOnForwardFailure,
    ForwardAgent,
    ForwardX11,
    ForwardX11Trusted,
    GatewayPorts,
    GlobalKnownHostsFile,
    HashKnownHosts,
    HostKeyAlias,
    Hostname,
    IdentitiesOnly,
    IdentityAgent,
    IdentityFile,
    IgnoreUnknown,
    Include,
    KbdInteractiveAuthentication,
    LocalForward,
    NumberOfPasswordPrompts,
    PasswordAuthentication,
    Port,
    PreferredAuthentications,
    ProxyCommand,
    ProxyJump,
    PubkeyAuthentication,
    RemoteForward,
    SendEnv,
    ServerAliveCountMax,
    ServerAliveInterval,
    SetEnv,
    StrictHostKeyChecking,
    Tag,
    TcpKeepAlive,
    User,
    UserKnownHostsFile,
}

/// How OpenSSH classifies a keyword found in a configuration line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordClass {
    Modeled(Keyword),
    KnownUnmodeled,
    Unknown,
}

const MODELED_KEYWORDS: &[(&str, Keyword)] = &[
    ("addkeystoagent", Keyword::AddKeysToAgent),
    ("addressfamily", Keyword::AddressFamily),
    ("batchmode", Keyword::BatchMode),
    ("canonicalizehostname", Keyword::CanonicalizeHostname),
    ("certificatefile", Keyword::CertificateFile),
    (
        "challengeresponseauthentication",
        Keyword::KbdInteractiveAuthentication,
    ),
    ("clearallforwardings", Keyword::ClearAllForwardings),
    ("compression", Keyword::Compression),
    ("connectionattempts", Keyword::ConnectionAttempts),
    ("connecttimeout", Keyword::ConnectTimeout),
    ("dsaauthentication", Keyword::PubkeyAuthentication),
    ("dynamicforward", Keyword::DynamicForward),
    ("exitonforwardfailure", Keyword::ExitOnForwardFailure),
    ("forwardagent", Keyword::ForwardAgent),
    ("forwardx11", Keyword::ForwardX11),
    ("forwardx11trusted", Keyword::ForwardX11Trusted),
    ("gatewayports", Keyword::GatewayPorts),
    ("globalknownhostsfile", Keyword::GlobalKnownHostsFile),
    ("hashknownhosts", Keyword::HashKnownHosts),
    ("hostkeyalias", Keyword::HostKeyAlias),
    ("hostname", Keyword::Hostname),
    ("identitiesonly", Keyword::IdentitiesOnly),
    ("identityagent", Keyword::IdentityAgent),
    ("identityfile", Keyword::IdentityFile),
    ("identityfile2", Keyword::IdentityFile),
    ("ignoreunknown", Keyword::IgnoreUnknown),
    ("include", Keyword::Include),
    (
        "kbdinteractiveauthentication",
        Keyword::KbdInteractiveAuthentication,
    ),
    ("keepalive", Keyword::TcpKeepAlive),
    ("localforward", Keyword::LocalForward),
    ("numberofpasswordprompts", Keyword::NumberOfPasswordPrompts),
    ("passwordauthentication", Keyword::PasswordAuthentication),
    ("port", Keyword::Port),
    (
        "preferredauthentications",
        Keyword::PreferredAuthentications,
    ),
    ("proxycommand", Keyword::ProxyCommand),
    ("proxyjump", Keyword::ProxyJump),
    ("pubkeyauthentication", Keyword::PubkeyAuthentication),
    ("remoteforward", Keyword::RemoteForward),
    ("sendenv", Keyword::SendEnv),
    ("serveralivecountmax", Keyword::ServerAliveCountMax),
    ("serveraliveinterval", Keyword::ServerAliveInterval),
    ("setenv", Keyword::SetEnv),
    ("skeyauthentication", Keyword::KbdInteractiveAuthentication),
    ("stricthostkeychecking", Keyword::StrictHostKeyChecking),
    ("tag", Keyword::Tag),
    ("tcpkeepalive", Keyword::TcpKeepAlive),
    ("tisauthentication", Keyword::KbdInteractiveAuthentication),
    ("user", Keyword::User),
    ("userknownhostsfile", Keyword::UserKnownHostsFile),
];

/// Every other keyword of OpenSSH 10.0 `readconf.c`, including deprecated and
/// unsupported ones. They are not modeled and never produce a warning.
const KNOWN_UNMODELED_KEYWORDS: &[&str] = &[
    "afstokenpassing",
    "bindaddress",
    "bindinterface",
    "canonicaldomains",
    "canonicalizefallbacklocal",
    "canonicalizemaxdots",
    "canonicalizepermittedcnames",
    "casignaturealgorithms",
    "channeltimeout",
    "checkhostip",
    "cipher",
    "ciphers",
    "compressionlevel",
    "controlmaster",
    "controlpath",
    "controlpersist",
    "enableescapecommandline",
    "enablesshkeysign",
    "escapechar",
    "fallbacktorsh",
    "fingerprinthash",
    "forkafterauthentication",
    "forwardx11timeout",
    "globalknownhostsfile2",
    "gssapiauthentication",
    "gssapidelegatecredentials",
    "hostbasedacceptedalgorithms",
    "hostbasedauthentication",
    "hostbasedkeytypes",
    "hostkeyalgorithms",
    "ipqos",
    "kbdinteractivedevices",
    "kerberosauthentication",
    "kerberostgtpassing",
    "kexalgorithms",
    "knownhostscommand",
    "localcommand",
    "loglevel",
    "logverbose",
    "macs",
    "nohostauthenticationforlocalhost",
    "obscurekeystroketiming",
    "permitlocalcommand",
    "permitremoteopen",
    "pkcs11provider",
    "protocol",
    "proxyusefdpass",
    "pubkeyacceptedalgorithms",
    "pubkeyacceptedkeytypes",
    "rekeylimit",
    "remotecommand",
    "requestty",
    "requiredrsasize",
    "revokedhostkeys",
    "rhostsauthentication",
    "rhostsrsaauthentication",
    "rsaauthentication",
    "securitykeyprovider",
    "sessiontype",
    "smartcarddevice",
    "stdinnull",
    "streamlocalbindmask",
    "streamlocalbindunlink",
    "syslogfacility",
    "tunnel",
    "tunneldevice",
    "updatehostkeys",
    "useprivilegedport",
    "useroaming",
    "usersh",
    "verifyhostkeydns",
    "versionaddendum",
    "visualhostkey",
    "xauthlocation",
];

impl Keyword {
    /// Parses a keyword name, ignoring case and accepting OpenSSH's aliases.
    pub fn parse(name: &str) -> Option<Self> {
        let lowercase_name = name.to_ascii_lowercase();
        MODELED_KEYWORDS
            .iter()
            .find(|(candidate, _)| *candidate == lowercase_name)
            .map(|(_, keyword)| *keyword)
    }
}

pub(crate) fn classify_keyword(name: &str) -> KeywordClass {
    if let Some(keyword) = Keyword::parse(name) {
        return KeywordClass::Modeled(keyword);
    }
    if KNOWN_UNMODELED_KEYWORDS.contains(&name.to_ascii_lowercase().as_str()) {
        return KeywordClass::KnownUnmodeled;
    }
    KeywordClass::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_keyword_when_name_is_lowercase() {
        assert_eq!(Keyword::parse("hostname"), Some(Keyword::Hostname));
    }

    #[test]
    fn should_parse_keyword_when_name_is_mixed_case() {
        assert_eq!(Keyword::parse("ProxyJump"), Some(Keyword::ProxyJump));
    }

    #[test]
    fn should_parse_keyword_when_name_is_an_alias() {
        assert_eq!(
            Keyword::parse("ChallengeResponseAuthentication"),
            Some(Keyword::KbdInteractiveAuthentication)
        );
    }

    #[test]
    fn should_not_parse_keyword_when_it_is_not_modeled() {
        assert_eq!(Keyword::parse("loglevel"), None);
    }

    #[test]
    fn should_classify_known_keyword_as_silent_when_it_is_not_modeled() {
        assert_eq!(classify_keyword("loglevel"), KeywordClass::KnownUnmodeled);
    }

    #[test]
    fn should_classify_keyword_as_unknown_when_openssh_does_not_know_it() {
        assert_eq!(classify_keyword("frobnicate"), KeywordClass::Unknown);
    }

    #[test]
    fn should_classify_modeled_keyword_when_it_is_in_the_table() {
        assert_eq!(
            classify_keyword("port"),
            KeywordClass::Modeled(Keyword::Port)
        );
    }

    #[test]
    fn should_not_list_a_keyword_in_both_tables() {
        let overlap = KNOWN_UNMODELED_KEYWORDS
            .iter()
            .filter(|name| Keyword::parse(name).is_some())
            .count();
        assert_eq!(overlap, 0);
    }
}
