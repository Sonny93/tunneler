import { useLingui } from '@lingui/react/macro';
import { useDisableHotkeysWhileMounted, IconButton } from '@minimalstuff/ui';
import { useEffect, useId, useRef, type ReactNode } from 'react';
/* oxlint-disable jsx-a11y/no-noninteractive-element-interactions, jsx-a11y/prefer-tag-over-role -- modal dialog: the focus trap needs a keydown handler and a native <dialog> cannot be a non-native-modal aria-modal portal panel */
import { createPortal } from 'react-dom';

import { useDrawerFocus } from '#/shared/use_drawer_focus';

type DrawerPanelProps = {
	title: string;
	onClose: () => void;
	children: ReactNode;
	footer?: ReactNode;
};

/** The open state of a drawer: mounting it disables global hotkeys and takes the focus. */
export function DrawerPanel({
	title,
	onClose,
	children,
	footer,
}: Readonly<DrawerPanelProps>) {
	const { t } = useLingui();
	const titleId = useId();
	const panelRef = useRef<HTMLDivElement>(null);
	const { handleKeyDown } = useDrawerFocus(panelRef);
	useDisableHotkeysWhileMounted();

	useEffect(() => {
		const closeOnEscape = (event: KeyboardEvent) => {
			if (event.key === 'Escape' && !event.defaultPrevented) onClose();
		};
		document.addEventListener('keydown', closeOnEscape);
		return () => document.removeEventListener('keydown', closeOnEscape);
	}, [onClose]);

	return createPortal(
		<div className="fixed inset-0 z-40">
			<div
				data-testid="drawer-backdrop"
				aria-hidden="true"
				onClick={onClose}
				className="absolute inset-0 bg-black/50"
			/>
			<div
				ref={panelRef}
				role="dialog"
				aria-modal="true"
				aria-labelledby={titleId}
				tabIndex={-1}
				onKeyDown={handleKeyDown}
				className="absolute inset-y-0 right-0 flex w-full flex-col border-l border-tn-divider bg-tn-surface text-tn-text shadow-xl sm:w-[420px]"
			>
				<div className="flex items-center justify-between gap-2 border-b border-tn-divider p-4">
					<h2 id={titleId} className="text-lg font-semibold">
						{title}
					</h2>
					<IconButton
						icon="i-mdi-close"
						aria-label={t`Close`}
						tooltip
						variant="ghost"
						color="neutral"
						onClick={onClose}
					/>
				</div>
				<div data-drawer-body className="flex-1 overflow-y-auto p-4">
					{children}
				</div>
				{footer && (
					<div className="border-t border-tn-divider p-4">{footer}</div>
				)}
			</div>
		</div>,
		document.body
	);
}
