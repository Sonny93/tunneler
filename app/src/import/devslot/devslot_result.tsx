import { useLingui } from '@lingui/react/macro';
import { useEffect, useId, useRef } from 'react';

import type { DevslotDiscoveryDto, SettingsDto } from '#/daemon/protocol';
import { DevslotAskList } from '#/import/devslot/devslot_ask_list';
import { DevslotStartForm } from '#/import/devslot/devslot_start_form';
import {
	describeDevslotFound,
	describeDevslotWarning,
} from '#/import/devslot/devslot_texts';

type DevslotResultProps = {
	discovery: DevslotDiscoveryDto;
	settings: SettingsDto;
	removedIds: ReadonlySet<string>;
	onStatus: (message: string) => void;
	onRemoved: (ids: readonly string[]) => void;
	onFailure: (rejection: unknown, title: string) => void;
};

/** Mount it with a new `key` per discovery: it takes the focus when mounted. */
export function DevslotResult({
	discovery,
	settings,
	removedIds,
	onStatus,
	onRemoved,
	onFailure,
}: Readonly<DevslotResultProps>) {
	const { t, i18n } = useLingui();
	const headingId = useId();
	const countId = useId();
	const headingRef = useRef<HTMLHeadingElement>(null);
	const askEntries = discovery.stale.filter(({ action }) => action === 'ask');

	useEffect(() => {
		headingRef.current?.focus();
	}, []);

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-4">
			<h3
				id={headingId}
				ref={headingRef}
				tabIndex={-1}
				aria-describedby={countId}
				className="m-0 text-base font-semibold"
			>
				{t`Discovery result`}
			</h3>
			<p id={countId} className="m-0 text-sm">
				{describeDevslotFound(discovery.instances.length, i18n)}
			</p>
			{discovery.warnings.length > 0 && (
				<ul className="m-0 flex list-disc flex-col gap-1 pl-5 text-sm">
					{discovery.warnings.map((warning) => {
						const text = describeDevslotWarning(warning, i18n);
						return <li key={text}>{text}</li>;
					})}
				</ul>
			)}
			{askEntries.length > 0 && (
				<DevslotAskList
					host={discovery.host}
					entries={askEntries}
					onStatus={onStatus}
					onRemoved={onRemoved}
					onFailure={onFailure}
				/>
			)}
			{discovery.instances.length > 0 && (
				<DevslotStartForm
					discovery={discovery}
					settings={settings}
					removedIds={removedIds}
				/>
			)}
		</section>
	);
}
