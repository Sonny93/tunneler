import type { AvailableOpenerDto, DefaultOpenerDto } from '#/daemon/protocol';

export const BROWSER_OPENER_ID = 'browser';
const BROWSER_NAME = 'Navigateur';

/** One way to open a tunnel: the browser or an application. */
export type OpenChoice = { id: string; name: string };

export const BROWSER_CHOICE: OpenChoice = {
	id: BROWSER_OPENER_ID,
	name: BROWSER_NAME,
};

/** The default action of a tunnel as a choice; the browser name is worded here, not by the daemon. */
export function toDefaultChoice(
	defaultOpener: Readonly<DefaultOpenerDto>
): OpenChoice {
	return defaultOpener.id === BROWSER_OPENER_ID
		? BROWSER_CHOICE
		: { id: defaultOpener.id, name: defaultOpener.name };
}

/** The browser first, then every available application. */
export function listOpenChoices(
	openers: readonly Readonly<AvailableOpenerDto>[]
): OpenChoice[] {
	return [BROWSER_CHOICE, ...openers.map(({ id, name }) => ({ id, name }))];
}

/** Every choice except the default one: what "Ouvrir avec…" offers. */
export function listAlternativeChoices(
	defaultOpener: Readonly<DefaultOpenerDto>,
	openers: readonly Readonly<AvailableOpenerDto>[]
): OpenChoice[] {
	return listOpenChoices(openers).filter(({ id }) => id !== defaultOpener.id);
}

/** The label of the primary open action: it says what is going to happen. */
export function describeOpenAction(choice: Readonly<OpenChoice>): string {
	return choice.id === BROWSER_OPENER_ID
		? 'Ouvrir dans le navigateur'
		: `Ouvrir avec ${choice.name}`;
}

/** The palette label of an open action on a tunnel. */
export function describeTunnelOpenCommand(
	endpointLabel: string,
	choice: Readonly<OpenChoice>
): string {
	return choice.id === BROWSER_OPENER_ID
		? `Ouvrir ${endpointLabel} dans le navigateur`
		: `Ouvrir ${endpointLabel} avec ${choice.name}`;
}
