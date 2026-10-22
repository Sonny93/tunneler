import { Checkbox } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

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
	const { t } = useLingui();
	const isEverythingSelected = selectedCount === visibleCount;
	const isPartial = selectedCount > 0 && !isEverythingSelected;

	const handleChange = () => {
		if (selectedCount > 0 && isEverythingSelected) {
			onClear();
			return;
		}
		onSelectAll();
	};

	return (
		<Checkbox
			label={t`Select all`}
			checked={selectedCount > 0 && isEverythingSelected}
			indeterminate={isPartial}
			onChange={handleChange}
		/>
	);
}
