import { toast } from '@minimalstuff/ui';
import * as jestDomMatchers from '@testing-library/jest-dom/matchers';
import { cleanup } from '@testing-library/react';
import { afterEach, beforeEach, expect } from 'vitest';
import * as axeMatchers from 'vitest-axe/matchers';

import { activateLocale } from '#/i18n/activate_locale';
import {
	INITIAL_ANNOUNCER_STATE,
	useAnnouncerStore,
} from '#/shared/announcer/announcer_store';
import { INITIAL_DAEMON_STATE } from '#/state/daemon_state';
import { useDaemonStore } from '#/state/daemon_store';
import { INITIAL_UI_STATE, useUiStore } from '#/state/ui_store';
import { spyOnToasts } from '#/testing/shown_toasts';
import { INITIAL_UPDATE_STATE, useUpdateStore } from '#/updates/update_store';

expect.extend(jestDomMatchers);
expect.extend(axeMatchers);

beforeEach(async () => {
	await activateLocale('en');
	spyOnToasts();
});

afterEach(() => {
	cleanup();
	localStorage.clear();
	useDaemonStore.setState(INITIAL_DAEMON_STATE);
	useUiStore.setState(INITIAL_UI_STATE);
	useUpdateStore.setState(INITIAL_UPDATE_STATE);
	useAnnouncerStore.setState(INITIAL_ANNOUNCER_STATE);
	toast.dismiss();
});
