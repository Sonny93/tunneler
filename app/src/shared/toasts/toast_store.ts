import { create } from 'zustand';

export type ToastTone = 'info' | 'success' | 'error';

export type Toast = { id: number; tone: ToastTone; message: string };

type ToastDraft = Omit<Toast, 'id'>;

type ToastState = {
	toasts: readonly Toast[];
	pushToast: (draft: ToastDraft) => void;
	dismissToast: (id: number) => void;
};

let nextToastId = 1;

export const useToastStore = create<ToastState>()((set) => ({
	toasts: [],
	pushToast: (draft) => {
		const toast: Toast = { ...draft, id: nextToastId };
		nextToastId += 1;
		set((state) => ({ toasts: [...state.toasts, toast] }));
	},
	dismissToast: (id) =>
		set((state) => ({
			toasts: state.toasts.filter((toast) => toast.id !== id),
		})),
}));
