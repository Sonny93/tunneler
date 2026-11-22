import { useLingui } from '@lingui/react/macro';
import { Button, Checkbox } from '@minimalstuff/ui';

import type { ImportCandidateDto, ImportConflictDto } from '#/daemon/protocol';
import {
	describeImportCandidateDetails,
	describeImportCandidateLabel,
} from '#/import/describe_import_candidate';
import { describeImportConflict } from '#/import/describe_import_conflict';
import { describeSelectionCount } from '#/import/import_texts';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useDaemonStore } from '#/state/daemon_store';

type ImportCandidateListProps = {
	candidates: readonly ImportCandidateDto[];
	conflicts: readonly (ImportConflictDto | null)[];
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
	const tunnels = useDaemonStore((state) => state.tunnels);
	const candidateCount = candidates.length;

	const announceSelection = (count: number) =>
		useAnnouncerStore
			.getState()
			.announce(describeSelectionCount(count, 0, i18n), 'polite');
	const handleSelectAll = () => {
		onSelectAll();
		announceSelection(candidateCount);
	};
	const handleSelectNone = () => {
		onSelectNone();
		announceSelection(0);
	};

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
					onClick={handleSelectAll}
				>
					{t`Select all`}
				</Button>
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					onClick={handleSelectNone}
				>
					{t`Deselect all`}
				</Button>
			</div>
			{candidates.map((candidate, index) => {
				const conflict = conflicts[index] ?? null;
				const conflictNote =
					conflict === null
						? null
						: describeImportConflict(conflict, tunnels, i18n);
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
