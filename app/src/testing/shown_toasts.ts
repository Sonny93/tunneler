import { toast } from '@minimalstuff/ui';
import { vi, type MockInstance } from 'vitest';

import type { ToastDraft, ToastTone } from '#/shared/toasts/push_toast';

const TONES: readonly ToastTone[] = ['info', 'success', 'warning', 'error'];

type ToastCall = { order: number; draft: ToastDraft };

export function spyOnToasts(): void {
	for (const tone of TONES) vi.spyOn(toast, tone);
}

function getCalls(tone: ToastTone): readonly ToastCall[] {
	// oxlint-disable-next-line typescript/unbound-method -- only the mock metadata is read, the method is never called
	const spy: MockInstance = vi.mocked(toast[tone]);
	return spy.mock.calls.map(([message], index) => ({
		order: spy.mock.invocationCallOrder[index] ?? 0,
		draft: { tone, message: String(message) },
	}));
}

/** The toasts shown through `pushToast` so far in the current test, in call order. */
export function getShownToasts(): ToastDraft[] {
	// oxlint-disable-next-line typescript/unbound-method -- only checks whether the method is a mock
	if (!vi.isMockFunction(toast.info)) {
		throw new Error(
			'The toast spies are gone (vi.restoreAllMocks?): call spyOnToasts() again before reading the shown toasts.'
		);
	}
	return TONES.flatMap(getCalls)
		.sort((first, second) => first.order - second.order)
		.map(({ draft }) => draft);
}
