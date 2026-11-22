import { useState } from 'react';

type UseImportSelectionReturn = {
	selectedIndexes: ReadonlySet<number>;
	toggle: (index: number) => void;
	selectAll: () => void;
	selectNone: () => void;
	deselect: (indexes: readonly number[]) => void;
	/** Checks exactly these candidates. */
	replace: (indexes: Iterable<number>) => void;
};

/** The checked candidates, by index. */
export function useImportSelection(
	initialIndexes: ReadonlySet<number>,
	candidateCount: number
): UseImportSelectionReturn {
	const [selectedIndexes, setSelectedIndexes] = useState(initialIndexes);

	const toggle = (index: number) =>
		setSelectedIndexes((current) => {
			const next = new Set(current);
			if (!next.delete(index)) next.add(index);
			return next;
		});
	const selectAll = () =>
		setSelectedIndexes(
			new Set(Array.from({ length: candidateCount }, (_, index) => index))
		);
	const selectNone = () => setSelectedIndexes(new Set());
	const deselect = (indexes: readonly number[]) =>
		setSelectedIndexes(
			(current) =>
				new Set([...current].filter((index) => !indexes.includes(index)))
		);

	const replace = (indexes: Iterable<number>) =>
		setSelectedIndexes(new Set(indexes));

	return { selectedIndexes, toggle, selectAll, selectNone, deselect, replace };
}
