import { useLingui } from '@lingui/react/macro';
import { useCallback } from 'react';

import type { DocumentationPageDto } from '#/daemon/protocol';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { SOURCE_LOCALE, isSupportedLocale } from '#/i18n/supported_locales';

type OpenDocumentation = (page: DocumentationPageDto) => Promise<boolean>;

/** Opens a page of the documentation in the active language; an error toast reports a failure. */
export function useOpenDocumentation(): OpenDocumentation {
	const { t, i18n } = useLingui();
	const runDaemonAction = useDaemonAction();
	const locale = isSupportedLocale(i18n.locale) ? i18n.locale : SOURCE_LOCALE;

	return useCallback(
		(page) =>
			runDaemonAction(
				(gateway) => gateway.openDocumentation(page, locale),
				t`Couldn't open the documentation`
			),
		[runDaemonAction, locale, t]
	);
}
