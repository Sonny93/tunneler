import { ToastItem } from '#/shared/toasts/toast_item';
import { useToastStore } from '#/shared/toasts/toast_store';

export function ToastRegion() {
	const toasts = useToastStore((state) => state.toasts);
	const dismissToast = useToastStore((state) => state.dismissToast);

	return (
		<div className="fixed bottom-4 right-4 z-50 flex w-96 max-w-[calc(100vw-2rem)] flex-col gap-2 max-[480px]:left-4 max-[480px]:w-auto">
			{toasts.map((toast) => (
				<ToastItem key={toast.id} toast={toast} onDismiss={dismissToast} />
			))}
		</div>
	);
}
