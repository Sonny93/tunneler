import { Button } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { buildImportDraft } from '#/externals/build_import_draft';
import { SPOKEN_SEPARATOR } from '#/tunnels/tunnel_endpoint_label';
import type { ExternalForwardDto, ExternalProcessDto } from '#/daemon/protocol';
import {
	getExternalForwardAccessibleLabel,
	getExternalForwardLabel,
	getExternalForwardRoute,
} from '#/externals/external_forward_label';

type ExternalForwardItemProps = {
	process: ExternalProcessDto;
	forward: ExternalForwardDto;
};

export function ExternalForwardItem({
	process,
	forward,
}: Readonly<ExternalForwardItemProps>) {
	const openTunnelImport = useUiStore((state) => state.openTunnelImport);
	const isImportable = forward.kind === 'local';
	const route = getExternalForwardRoute(forward, SPOKEN_SEPARATOR);

	const handleImport = () =>
		openTunnelImport(buildImportDraft(process, forward));

	return (
		<li className="flex flex-wrap items-center gap-2">
			<span className="font-mono text-sm text-tn-muted">
				<span aria-hidden="true">{getExternalForwardLabel(forward)}</span>
				<span className="sr-only">
					{getExternalForwardAccessibleLabel(forward)}
				</span>
			</span>
			{isImportable && (
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					aria-label={`Importer comme tunnel : ${route}`}
					onClick={handleImport}
				>
					Importer comme tunnel
				</Button>
			)}
		</li>
	);
}
