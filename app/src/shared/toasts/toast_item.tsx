import { useCallback, useState } from 'react';

import { cn } from '#/shared/cn';
import { TooltipIconButton } from '#/shared/tooltip_icon_button';
import { useAutoDismiss } from '#/shared/toasts/use_auto_dismiss';
import type { Toast, ToastTone } from '#/shared/toasts/toast_store';

type ToastAppearance = { icon: string; iconClassName: string };

const TOAST_APPEARANCES: Readonly<Record<ToastTone, ToastAppearance>> = {
	info: { icon: 'i-mdi-information-outline', iconClassName: 'text-tn-info' },
	success: {
		icon: 'i-mdi-check-circle-outline',
		iconClassName: 'text-tn-success',
	},
	error: {
		icon: 'i-mdi-alert-circle-outline',
		iconClassName: 'text-tn-danger',
	},
};

type ToastItemProps = {
	toast: Toast;
	onDismiss: (id: number) => void;
};

export function ToastItem({ toast, onDismiss }: Readonly<ToastItemProps>) {
	const [isPaused, setIsPaused] = useState(false);
	const { icon, iconClassName } = TOAST_APPEARANCES[toast.tone];
	const isError = toast.tone === 'error';
	const dismiss = useCallback(() => onDismiss(toast.id), [onDismiss, toast.id]);
	const pause = () => setIsPaused(true);
	const resume = () => setIsPaused(false);

	useAutoDismiss(!isError, isPaused, dismiss);

	return (
		<div
			role={isError ? 'alert' : 'status'}
			onMouseEnter={pause}
			onMouseLeave={resume}
			onFocus={pause}
			onBlur={resume}
			className="flex items-start gap-3 rounded-lg border border-tn-divider bg-tn-surface-raised p-3 text-tn-text shadow-lg"
		>
			<span
				aria-hidden="true"
				className={cn(icon, 'mt-0.5 size-5 shrink-0', iconClassName)}
			/>
			<p className="min-w-0 flex-1 break-words text-sm">{toast.message}</p>
			<TooltipIconButton
				icon="i-mdi-close"
				aria-label="Fermer la notification"
				size="sm"
				variant="ghost"
				color="neutral"
				onClick={dismiss}
			/>
		</div>
	);
}
