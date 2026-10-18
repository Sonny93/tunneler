import { useEffect, useRef } from 'react';
import { Checkbox } from '@minimalstuff/ui';

type SelectAllCheckboxProps = {
	visibleCount: number;
	selectedCount: number;
	onSelectAll: () => void;
	onClear: () => void;
};

/** Checks every visible tunnel, or clears the selection; indeterminate while only some are selected. */
export function SelectAllCheckbox({
	visibleCount,
	selectedCount,
	onSelectAll,
	onClear,
}: Readonly<SelectAllCheckboxProps>) {
	const inputRef = useRef<HTMLInputElement>(null);
	const isEverythingSelected = selectedCount === visibleCount;
	const isPartial = selectedCount > 0 && !isEverythingSelected;

	useEffect(() => {
		if (inputRef.current) inputRef.current.indeterminate = isPartial;
	}, [isPartial]);

	const handleChange = () => {
		if (selectedCount > 0 && isEverythingSelected) {
			onClear();
			return;
		}
		onSelectAll();
	};

	return (
		<Checkbox
			ref={inputRef}
			label="Tout sélectionner"
			checked={selectedCount > 0 && isEverythingSelected}
			onChange={handleChange}
		/>
	);
}
