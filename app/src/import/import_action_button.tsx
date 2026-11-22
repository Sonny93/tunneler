import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';
import { useId, type MouseEvent, type ReactNode } from 'react';

type ImportActionButtonProps = {
	children: ReactNode;
	startIcon: string;
	/** The action is running: the button shows its spinner. */
	isRunning: boolean;
	/** The daemon has not answered yet: which routes conflict, or the new preview. */
	isChecking: boolean;
	/** Said while checking; "Checking for conflicts…" when not given. */
	checkingHint?: string;
	isSelectionEmpty: boolean;
	/** Said while nothing is selected, for example "Select at least one tunnel". */
	emptyHint: string;
};

/**
 * The submit button of an import form. While it cannot act it stays focusable
 * (`aria-disabled`, never `disabled`), does nothing when activated, and says
 * why in a visible hint the button is described by.
 */
export function ImportActionButton({
	children,
	startIcon,
	isRunning,
	isChecking,
	checkingHint,
	isSelectionEmpty,
	emptyHint,
}: Readonly<ImportActionButtonProps>) {
	const { t } = useLingui();
	const hintId = useId();
	const hint = (() => {
		if (isChecking) return checkingHint ?? t`Checking for conflicts…`;
		return isSelectionEmpty ? emptyHint : null;
	})();
	const isBlocked = hint !== null;

	const handleClick = (event: MouseEvent<HTMLButtonElement>) => {
		if (isBlocked) event.preventDefault();
	};

	return (
		<div className="flex flex-col gap-1">
			<div>
				<Button
					type="submit"
					startIcon={startIcon}
					loading={isRunning}
					aria-disabled={isBlocked || isRunning || undefined}
					aria-describedby={isBlocked ? hintId : undefined}
					onClick={handleClick}
				>
					{children}
				</Button>
			</div>
			{isBlocked && (
				<p id={hintId} className="m-0 text-sm text-tn-muted">
					{hint}
				</p>
			)}
		</div>
	);
}
