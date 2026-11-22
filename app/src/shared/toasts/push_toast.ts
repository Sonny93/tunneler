import { toast } from '@minimalstuff/ui';

export type ToastTone = 'info' | 'success' | 'warning' | 'error';

export type ToastDraft = { tone: ToastTone; message: string };

export function pushToast(draft: Readonly<ToastDraft>): void {
	toast[draft.tone](draft.message);
}
