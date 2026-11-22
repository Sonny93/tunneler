import { useLingui } from '@lingui/react/macro';
import { Checkbox, Input } from '@minimalstuff/ui';
import type { ChangeEvent } from 'react';

import { describeDevslotPort } from '#/import/devslot/describe_devslot_port';
import { describeDevslotProblem } from '#/import/devslot/describe_devslot_problem';
import {
	isPortTunnelable,
	type DevslotPortEntry,
} from '#/import/devslot/devslot_ports';
import type { DevslotPortProblem } from '#/import/devslot/find_devslot_problems';

type DevslotPortRowProps = {
	entry: DevslotPortEntry;
	isSelected: boolean;
	problem: DevslotPortProblem | null;
	localPortText: string;
	isShowingErrors: boolean;
	onToggle: (key: string) => void;
	onLocalPortChange: (key: string, text: string) => void;
};

export function DevslotPortRow({
	entry,
	isSelected,
	problem,
	localPortText,
	isShowingErrors,
	onToggle,
	onLocalPortChange,
}: Readonly<DevslotPortRowProps>) {
	const { t, i18n } = useLingui();
	const { key, port, instance } = entry;
	const variable = port.var;
	const instanceName = instance.name;
	const remotePort = port.port;
	const isTunnelable = isPortTunnelable(entry);
	const isLocalPortChanged = localPortText.trim() !== String(remotePort);
	const isFieldShown = isTunnelable && (problem !== null || isLocalPortChanged);
	const problemText =
		problem === null ? undefined : describeDevslotProblem(problem, i18n);
	const hasError = isShowingErrors && isSelected && problemText !== undefined;
	const mismatchWarning = isLocalPortChanged
		? t`Warning: the app builds its links with port ${remotePort}, which will not match this local port.`
		: undefined;
	const fieldDescription = [hasError ? undefined : problemText, mismatchWarning]
		.filter((text) => text !== undefined)
		.join(' ');

	const portDescription = describeDevslotPort(port, i18n);
	const checkboxDescription =
		isSelected && problemText !== undefined
			? `${portDescription}. ${problemText}`
			: portDescription;

	const handleToggle = () => onToggle(key);
	const handleLocalPortChange = (event: ChangeEvent<HTMLInputElement>) =>
		onLocalPortChange(key, event.target.value);

	return (
		<div className="flex flex-col gap-2">
			<Checkbox
				label={t`${variable} (${remotePort})`}
				description={checkboxDescription}
				checked={isSelected}
				disabled={!isTunnelable}
				onChange={handleToggle}
			/>
			{isFieldShown && (
				<Input
					name={`localPort-${key}`}
					label={t`Local port for ${variable} of ${instanceName}`}
					description={fieldDescription === '' ? undefined : fieldDescription}
					error={hasError ? problemText : undefined}
					value={localPortText}
					onChange={handleLocalPortChange}
					inputMode="numeric"
					autoComplete="off"
					wrapperClassName="ml-7"
				/>
			)}
		</div>
	);
}
