import * as jestDomMatchers from '@testing-library/jest-dom/matchers';
import { cleanup } from '@testing-library/react';
import { afterEach, beforeEach, expect, vi } from 'vitest';
import * as axeMatchers from 'vitest-axe/matchers';

import { activateLocale } from '#/i18n/activate_locale';
import {
	INITIAL_ANNOUNCER_STATE,
	useAnnouncerStore,
} from '#/shared/announcer/announcer_store';
import { useToastStore } from '#/shared/toasts/toast_store';
import { INITIAL_DAEMON_STATE } from '#/state/daemon_state';
import { useDaemonStore } from '#/state/daemon_store';
import { INITIAL_UI_STATE, useUiStore } from '#/state/ui_store';
import { INITIAL_UPDATE_STATE, useUpdateStore } from '#/updates/update_store';

expect.extend(jestDomMatchers);
expect.extend(axeMatchers);

vi.stubGlobal(
	'matchMedia',
	(query: string): MediaQueryList =>
		({
			matches: false,
			media: query,
			addEventListener: vi.fn(),
			removeEventListener: vi.fn(),
		}) as unknown as MediaQueryList
);

beforeEach(async () => {
	await activateLocale('en');
});

afterEach(() => {
	cleanup();
	localStorage.clear();
	useDaemonStore.setState(INITIAL_DAEMON_STATE);
	useUiStore.setState(INITIAL_UI_STATE);
	useUpdateStore.setState(INITIAL_UPDATE_STATE);
	useAnnouncerStore.setState(INITIAL_ANNOUNCER_STATE);
	useToastStore.setState({ toasts: [] });
});
