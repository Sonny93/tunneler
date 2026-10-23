import { plural } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';

type SettingsTabErrorBadgeProps = {
	count: number;
};

/** The error count as text, so it is part of the accessible name of its tab. */
export function SettingsTabErrorBadge({
	count,
}: Readonly<SettingsTabErrorBadgeProps>) {
	const { t } = useLingui();

	return (
		<span className="text-xs font-semibold text-tn-danger">
			{t`${plural(count, { one: '# error', other: '# errors' })}`}
		</span>
	);
}
