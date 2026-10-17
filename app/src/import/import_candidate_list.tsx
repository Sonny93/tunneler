import { Button, Checkbox } from '@minimalstuff/ui';

import type { ImportCandidateDto } from '#/daemon/protocol';
import type { ImportConflict } from '#/import/find_import_conflicts';
import { describeImportConflict } from '#/import/describe_import_conflict';
import { describeImportCandidate } from '#/import/describe_import_candidate';

type ImportCandidateListProps = {
	candidates: readonly ImportCandidateDto[];
	names: readonly string[];
	conflicts: readonly (ImportConflict | null)[];
	selectedIndexes: ReadonlySet<number>;
	fallbackHost: string;
	onToggle: (index: number) => void;
	onSelectAll: () => void;
	onSelectNone: () => void;
};

export function ImportCandidateList({
	candidates,
	names,
	conflicts,
	selectedIndexes,
	fallbackHost,
	onToggle,
	onSelectAll,
	onSelectNone,
}: Readonly<ImportCandidateListProps>) {
	return (
		<fieldset className="m-0 flex flex-col gap-3 border-0 p-0">
			<legend className="mb-2 text-sm font-semibold">
				Tunnels détectés ({candidates.length})
			</legend>
			<div className="flex flex-wrap gap-2">
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					onClick={onSelectAll}
				>
					Tout sélectionner
				</Button>
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					onClick={onSelectNone}
				>
					Tout désélectionner
				</Button>
			</div>
			{candidates.map((candidate, index) => {
				const conflict = conflicts[index] ?? null;
				const conflictNote =
					conflict === null ? null : describeImportConflict(conflict);
				return (
					<Checkbox
						// oxlint-disable-next-line react/no-array-index-key -- candidates have no identity beyond their position in the preview
						key={index}
						label={names[index]}
						description={describeImportCandidate(
							candidate,
							fallbackHost,
							conflictNote
						)}
						checked={selectedIndexes.has(index)}
						onChange={() => onToggle(index)}
					/>
				);
			})}
		</fieldset>
	);
}
