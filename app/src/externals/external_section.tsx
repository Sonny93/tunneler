import { useLingui } from '@lingui/react/macro';
import { useId } from 'react';

import { ExternalProcessItem } from '#/externals/external_process_item';
import { useDaemonStore } from '#/state/daemon_store';

export function ExternalSection() {
	const { t } = useLingui();
	const externals = useDaemonStore((state) => state.externals);
	const externalCount = externals.length;
	const headingId = useId();

	if (externals.length === 0) return null;

	return (
		<section aria-labelledby={headingId} className="mt-6 flex flex-col gap-3">
			<h2 id={headingId} className="text-base font-semibold">
				{t`Unmanaged externals (${externalCount})`}
			</h2>
			<ul role="list" className="m-0 flex list-none flex-col gap-2 p-0">
				{externals.map((process) => (
					<ExternalProcessItem key={process.pid} process={process} />
				))}
			</ul>
		</section>
	);
}
