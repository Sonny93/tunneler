import { useLingui } from '@lingui/react/macro';

import { CONFLICT_CHECK_FAILED_MESSAGE } from '#/import/use_conflict_announcements';

type ConflictCheckFailureProps = {
	hasFailed: boolean;
};

/** Visible text while the last check for conflicts failed; the previous answer stays in use. */
export function ConflictCheckFailure({
	hasFailed,
}: Readonly<ConflictCheckFailureProps>) {
	const { i18n } = useLingui();
	if (!hasFailed) return null;

	return (
		<p className="m-0 text-sm text-tn-danger">
			{i18n._(CONFLICT_CHECK_FAILED_MESSAGE)}
		</p>
	);
}
