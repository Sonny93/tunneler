import { Button } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { useUiStore } from '#/state/ui_store';
import { buildImportDraft } from '#/externals/build_import_draft';
import type { ExternalForwardDto, ExternalProcessDto } from '#/daemon/protocol';
import {
	getExternalForwardAccessibleLabel,
	getExternalForwardLabel,
	getExternalForwardSpokenRoute,
} from '#/externals/external_forward_label';

type ExternalForwardItemProps = {
	process: ExternalProcessDto;
	forward: ExternalForwardDto;
};

export function ExternalForwardItem({
	process,
	forward,
}: Readonly<ExternalForwardItemProps>) {
	const { t, i18n } = useLingui();
	const openTunnelImport = useUiStore((state) => state.openTunnelImport);
	const route = getExternalForwardSpokenRoute(forward, i18n);

	const handleImport = () =>
		openTunnelImport(buildImportDraft(process, forward));

	return (
		<li className="flex flex-wrap items-center gap-2">
			<span className="font-mono text-sm text-tn-muted">
				<span aria-hidden="true">{getExternalForwardLabel(forward)}</span>
				<span className="sr-only">
					{getExternalForwardAccessibleLabel(forward, i18n)}
				</span>
			</span>
			<Button
				size="sm"
				variant="outline"
				color="neutral"
				aria-label={t`Import as a tunnel: ${route}`}
				onClick={handleImport}
			>
				{t`Import as a tunnel`}
			</Button>
		</li>
	);
}
