import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { AvailableOpenerDto, DefaultOpenerDto } from '#/daemon/protocol';

export const BROWSER_OPENER_ID = 'browser';

/** One way to open a tunnel: the browser or an application. */
export type OpenChoice = { id: string; name: string };

export function getBrowserChoice(i18n: I18n): OpenChoice {
	return { id: BROWSER_OPENER_ID, name: i18n._(msg`Browser`) };
}

/** The default action of a tunnel as a choice; the browser name is worded here, not by the daemon. */
export function toDefaultChoice(
	defaultOpener: Readonly<DefaultOpenerDto>,
	i18n: I18n
): OpenChoice {
	return defaultOpener.id === BROWSER_OPENER_ID
		? getBrowserChoice(i18n)
		: { id: defaultOpener.id, name: defaultOpener.name };
}

/** The browser first, then every available application. */
export function listOpenChoices(
	openers: readonly Readonly<AvailableOpenerDto>[],
	i18n: I18n
): OpenChoice[] {
	return [
		getBrowserChoice(i18n),
		...openers.map(({ id, name }) => ({ id, name })),
	];
}

/** Every choice except the default one: what "Open with…" offers. */
export function listAlternativeChoices(
	defaultOpener: Readonly<DefaultOpenerDto>,
	openers: readonly Readonly<AvailableOpenerDto>[],
	i18n: I18n
): OpenChoice[] {
	return listOpenChoices(openers, i18n).filter(
		({ id }) => id !== defaultOpener.id
	);
}

/** The label of the primary open action: it says what is going to happen. */
export function describeOpenAction(
	choice: Readonly<OpenChoice>,
	i18n: I18n
): string {
	if (choice.id === BROWSER_OPENER_ID) return i18n._(msg`Open in the browser`);
	const { name } = choice;
	return i18n._(msg`Open with ${name}`);
}

/** The palette label of an open action on a tunnel. */
export function describeTunnelOpenCommand(
	endpointLabel: string,
	choice: Readonly<OpenChoice>,
	i18n: I18n
): string {
	if (choice.id === BROWSER_OPENER_ID) {
		return i18n._(msg`Open ${endpointLabel} in the browser`);
	}
	const { name } = choice;
	return i18n._(msg`Open ${endpointLabel} with ${name}`);
}
