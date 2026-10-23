import { useLingui } from '@lingui/react/macro';
import { Button, Checkbox } from '@minimalstuff/ui';

import type { ImportCandidateDto } from '#/daemon/protocol';
import {
	describeImportCandidateDetails,
	describeImportCandidateLabel,
} from '#/import/describe_import_candidate';
import { describeImportConflict } from '#/import/describe_import_conflict';
import type { ImportConflict } from '#/import/find_import_conflicts';

type ImportCandidateListProps = {
	candidates: readonly ImportCandidateDto[];
	conflicts: readonly (ImportConflict | null)[];
	selectedIndexes: ReadonlySet<number>;
	fallbackHost: string;
	onToggle: (index: number) => void;
	onSelectAll: () => void;
	onSelectNone: () => void;
};

export function ImportCandidateList({
	candidates,
	conflicts,
	selectedIndexes,
	fallbackHost,
	onToggle,
	onSelectAll,
	onSelectNone,
}: Readonly<ImportCandidateListProps>) {
	const { t, i18n } = useLingui();
	const candidateCount = candidates.length;

	return (
		<fieldset className="m-0 flex flex-col gap-3 border-0 p-0">
			<legend className="mb-2 text-sm font-semibold">
				{t`Detected tunnels (${candidateCount})`}
			</legend>
			<div className="flex flex-wrap gap-2">
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					onClick={onSelectAll}
				>
					{t`Select all`}
				</Button>
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					onClick={onSelectNone}
				>
					{t`Deselect all`}
				</Button>
			</div>
			{candidates.map((candidate, index) => {
				const conflict = conflicts[index] ?? null;
				const conflictNote =
					conflict === null ? null : describeImportConflict(conflict, i18n);
				return (
					<Checkbox
						// oxlint-disable-next-line react/no-array-index-key -- candidates have no identity beyond their position in the preview
						key={index}
						label={describeImportCandidateLabel(candidate, fallbackHost, i18n)}
						description={describeImportCandidateDetails(
							candidate,
							conflictNote,
							i18n
						)}
						checked={selectedIndexes.has(index)}
						onChange={() => onToggle(index)}
					/>
				);
			})}
		</fieldset>
	);
}
