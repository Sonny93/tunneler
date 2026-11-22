import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type {
	ConfigImportEntryDto,
	ConfigImportEntryWarningDto,
	ConfigImportRiskDto,
	ConfigImportStatusDto,
	TunnelDto,
	TunnelKindDto,
} from '#/daemon/protocol';
import { describeTunnelHolder } from '#/import/describe_import_conflict';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

const PARTS_SEPARATOR = ' · ';

/** A sentence around a raw English text from the engine (`raw`), which is marked `lang="en"` apart. */
export type ConfigImportText = { before: string; raw: string; after: string };

const plainText = (before: string): ConfigImportText => ({
	before,
	raw: '',
	after: '',
});
const LIST_SEPARATOR = ', ';

function describeAlreadyConfigured(
	ignoredTags: readonly string[],
	i18n: I18n
): string {
	if (ignoredTags.length === 0) return i18n._(msg`Already configured`);
	const tagList = ignoredTags.join(LIST_SEPARATOR);
	return i18n._(msg`Already configured (tags not applied: ${tagList})`);
}

export type ConfigImportContext = {
	tunnels: readonly Readonly<TunnelDto>[];
	/** The entries of the file, to name the one a duplicate points to. */
	entries: readonly ConfigImportEntryDto[];
};

function describeImportable(isSelectedByDefault: boolean, i18n: I18n): string {
	return isSelectedByDefault
		? i18n._(msg`Will be added`)
		: i18n._(msg`Can be added, not selected by default`);
}

function describeDuplicate(
	earlierIndex: number,
	entries: readonly ConfigImportEntryDto[],
	i18n: I18n
): string {
	const position = earlierIndex + 1;
	const earlier = entries.find((entry) => entry.index === earlierIndex);
	if (earlier === undefined) {
		return i18n._(msg`Same route as tunnel ${position} of the file`);
	}
	const route = getTunnelAccessibleEndpointLabel(earlier.tunnel, i18n);
	return i18n._(msg`Same route as tunnel ${position} of the file (${route})`);
}

/** Why the entry will, or will not, be added, as one phrase. */
export function describeConfigImportStatus(
	status: Readonly<ConfigImportStatusDto>,
	{ tunnels, entries }: Readonly<ConfigImportContext>,
	i18n: I18n,
	isSelectedByDefault = true
): ConfigImportText {
	switch (status.kind) {
		case 'importable':
			return plainText(describeImportable(isSelectedByDefault, i18n));
		case 'alreadyConfigured':
			return plainText(describeAlreadyConfigured(status.ignoredTags, i18n));
		case 'listenConflict': {
			const holder = describeTunnelHolder(status.holderId, tunnels, i18n);
			return plainText(i18n._(msg`Port already used by ${holder}`));
		}
		case 'duplicateInFile':
			return plainText(describeDuplicate(status.earlierIndex, entries, i18n));
		case 'invalid':
			return {
				before: i18n._(msg`Invalid: `),
				raw: status.messages.join(LIST_SEPARATOR),
				after: '',
			};
	}
}

function describeNonLoopbackListen(
	address: string,
	kind: TunnelKindDto,
	i18n: I18n
): string {
	if (kind === 'remote') {
		return i18n._(
			msg`Risk: listens on ${address} on the server, reachable from the server's network`
		);
	}
	return i18n._(
		msg`Risk: listens on ${address}, reachable from other machines`
	);
}

export function describeConfigImportRisk(
	risk: Readonly<ConfigImportRiskDto>,
	tunnelKind: TunnelKindDto,
	i18n: I18n
): string {
	switch (risk.kind) {
		case 'nonLoopbackListen':
			return describeNonLoopbackListen(risk.address, tunnelKind, i18n);
		case 'remoteNonLoopbackTarget': {
			const { targetHost } = risk;
			return i18n._(
				msg`Risk: forwards remote connections to ${targetHost}, which is not this machine`
			);
		}
		case 'startsOnLaunch': {
			const tagList = risk.tags.join(LIST_SEPARATOR);
			return i18n._(msg`Risk: its tags start on launch (${tagList})`);
		}
	}
}

export function describeConfigImportWarning(
	warning: Readonly<ConfigImportEntryWarningDto>,
	i18n: I18n
): string {
	switch (warning.kind) {
		case 'unknownHost': {
			const { host } = warning;
			return i18n._(msg`No ssh_config Host entry names ${host} exactly`);
		}
		case 'openerDropped': {
			const { opener } = warning;
			return i18n._(
				msg`Opener ${opener} is not available here and was removed`
			);
		}
	}
}

/** The route of a skipped entry, as the start of its reason: "3333 to nova:3333 skipped: ". */
export function describeSkippedLead(
	entry: Readonly<ConfigImportEntryDto>,
	i18n: I18n
): string {
	const route = getTunnelAccessibleEndpointLabel(entry.tunnel, i18n);
	return i18n._(msg`${route} skipped: `);
}

/** The tags, the status, the risks and the warnings of an entry, as one line of text. */
export function describeConfigImportEntry(
	entry: Readonly<ConfigImportEntryDto>,
	context: Readonly<ConfigImportContext>,
	i18n: I18n
): ConfigImportText {
	const tagList = entry.tunnel.tags.join(LIST_SEPARATOR);
	const tagText = tagList === '' ? null : i18n._(msg`tags: ${tagList}`);
	const status = describeConfigImportStatus(
		entry.status,
		context,
		i18n,
		entry.selectedByDefault
	);
	const followers = [
		...entry.risks.map((risk) =>
			describeConfigImportRisk(risk, entry.tunnel.kind, i18n)
		),
		...entry.warnings.map((warning) =>
			describeConfigImportWarning(warning, i18n)
		),
	];
	const lead = tagText === null ? '' : `${tagText}${PARTS_SEPARATOR}`;
	const trail = followers.map((text) => `${PARTS_SEPARATOR}${text}`).join('');
	return {
		before: `${lead}${status.before}`,
		raw: status.raw,
		after: `${status.after}${trail}`,
	};
}
