use ipc::protocol::{TunnelDefinitionDto, TunnelKindDto};

use super::super::args::OpenArguments;
use super::super::session::{ConnectionMode, connect};
use super::super::target::find_tunnel_by_name;
use super::super::{CliError, Context};
use crate::logging;

/// The address a local tunnel answers on.
pub fn local_url(listen_port: u16) -> String {
    format!("http://localhost:{listen_port}")
}

fn browsable_url(definition: &TunnelDefinitionDto) -> Result<String, CliError> {
    let spec = &definition.spec;
    match spec.kind {
        TunnelKindDto::Local => Ok(local_url(spec.listen_port)),
        TunnelKindDto::Remote => Err(CliError::NotOpenable {
            name: spec.name.clone(),
            kind: "remote",
        }),
        TunnelKindDto::Dynamic => Err(CliError::NotOpenable {
            name: spec.name.clone(),
            kind: "dynamic",
        }),
    }
}

/// `tunneler open`: opens a local tunnel in the browser.
pub async fn open(context: &Context, arguments: &OpenArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    let snapshot = client.snapshot().await?;
    let definitions: Vec<TunnelDefinitionDto> = snapshot
        .tunnels
        .into_iter()
        .map(|tunnel| tunnel.definition)
        .collect();
    let definition = find_tunnel_by_name(&arguments.name, &definitions)?;
    let url = browsable_url(definition)?;
    println!("Opening {url}");
    let target = url.clone();
    tokio::task::spawn_blocking(move || open::that_detached(target))
        .await
        .map_err(|error| CliError::OpenFailed {
            url: url.clone(),
            source: std::io::Error::other(error),
        })?
        .map_err(|source| CliError::OpenFailed { url, source })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_point_at_localhost_on_the_listen_port() {
        assert_eq!(local_url(3333), "http://localhost:3333");
    }
}
