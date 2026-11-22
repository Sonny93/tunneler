import { useLingui } from '@lingui/react/macro';
import { Button, Select } from '@minimalstuff/ui';
import { useRef, useState, type ChangeEvent } from 'react';

import { DevslotNotFoundHint } from '#/import/devslot/devslot_not_found_hint';
import { useDaemonStore } from '#/state/daemon_store';

type ImportDevslotTabProps = {
	isDiscovering: boolean;
	notFoundHost: string | null;
	statusMessage: string;
	onDiscover: (host: string) => void;
};

export function ImportDevslotTab({
	isDiscovering,
	notFoundHost,
	statusMessage,
	onDiscover,
}: Readonly<ImportDevslotTabProps>) {
	const { t } = useLingui();
	const hosts = useDaemonStore((state) => state.hosts);
	const selectRef = useRef<HTMLSelectElement>(null);
	const [host, setHost] = useState('');
	const [error, setError] = useState<string>();
	const hostOptions = hosts.map((alias) => ({ value: alias, label: alias }));

	const handleHostChange = (event: ChangeEvent<HTMLSelectElement>) => {
		setError(undefined);
		setHost(event.target.value);
	};
	const handleDiscover = () => {
		if (host === '') {
			setError(t`Choose a host.`);
			selectRef.current?.focus();
			return;
		}
		onDiscover(host);
	};

	return (
		<div className="flex flex-col gap-3 pt-3">
			<p className="m-0 text-sm text-tn-muted">
				{t`Lists the devslot instances of a host (ds ls --json) and offers to tunnel their ports.`}
			</p>
			<Select
				ref={selectRef}
				name="devslotHost"
				label={t`Host`}
				options={hostOptions}
				placeholder={t`Choose a host`}
				value={host}
				onChange={handleHostChange}
				error={error}
				required
			/>
			<div>
				<Button
					startIcon="i-mdi-magnify"
					loading={isDiscovering}
					onClick={handleDiscover}
				>
					{t`Discover`}
				</Button>
			</div>
			<output className="block text-sm">{statusMessage}</output>
			{notFoundHost !== null && <DevslotNotFoundHint host={notFoundHost} />}
		</div>
	);
}
