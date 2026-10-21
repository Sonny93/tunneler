use super::model::{OpenOptions, TunnelDefinition, TunnelId, TunnelKind};

pub(crate) fn tunnel(kind: TunnelKind) -> TunnelDefinition {
    TunnelDefinition {
        id: TunnelId::generate(),
        host: "nova".to_owned(),
        kind,
        listen_port: 3333,
        listen_address: None,
        target_host: None,
        target_port: None,
        tags: Vec::new(),
        open: OpenOptions::default(),
        standby: None,
    }
}

pub(crate) fn tunnel_on(kind: TunnelKind, listen_port: u16) -> TunnelDefinition {
    TunnelDefinition {
        listen_port,
        ..tunnel(kind)
    }
}
