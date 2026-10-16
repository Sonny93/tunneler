use super::model::{TunnelDefinition, TunnelId, TunnelKind};

pub(crate) fn tunnel(kind: TunnelKind) -> TunnelDefinition {
    TunnelDefinition {
        id: TunnelId::generate(),
        name: "Adonis".to_owned(),
        host: "nova".to_owned(),
        kind,
        listen_port: 3333,
        listen_address: None,
        target_host: None,
        target_port: None,
        tags: Vec::new(),
    }
}

pub(crate) fn named_tunnel(kind: TunnelKind, name: &str, listen_port: u16) -> TunnelDefinition {
    TunnelDefinition {
        name: name.to_owned(),
        listen_port,
        ..tunnel(kind)
    }
}
