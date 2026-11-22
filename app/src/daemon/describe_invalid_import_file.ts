import type { I18n, MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { InvalidImportReasonDto } from '#/daemon/protocol';

const BYTES_PER_KIBIBYTE = 1024;

const REASONS: ReadonlySet<string> = new Set<InvalidImportReasonDto>([
	'syntax',
	'notTunnelerFile',
	'newerVersion',
	'unsafeCharacters',
	'tooLarge',
	'tooManyEntries',
]);

/** The fields of a tunnel that hold text, as the engine names them, in words. */
const FIELD_LABELS: Readonly<Record<string, MessageDescriptor>> = {
	host: msg`host`,
	listen_address: msg`listen address`,
	target_host: msg`target host`,
	tags: msg`tags`,
	'open.path': msg`open path`,
	'open.with': msg`opener`,
	'open.host_name': msg`open host name`,
};

export function isKnownImportField(field: string): boolean {
	return Object.hasOwn(FIELD_LABELS, field);
}

function describeField(field: string, i18n: I18n): string {
	const label = FIELD_LABELS[field];
	return label === undefined ? field : i18n._(label);
}

function readNumber(data: object, key: string): number | null {
	const value: unknown = Reflect.get(data, key);
	return typeof value === 'number' ? value : null;
}

function readText(data: object, key: string): string | null {
	const value: unknown = Reflect.get(data, key);
	return typeof value === 'string' ? value : null;
}

function describeSyntax(data: object, i18n: I18n): string {
	const line = readNumber(data, 'line');
	const detail = readText(data, 'message');
	if (line !== null && detail !== null) {
		return i18n._(msg`The file cannot be read (line ${line}): ${detail}`);
	}
	if (detail !== null) return i18n._(msg`The file cannot be read: ${detail}`);
	return i18n._(msg`The file cannot be read.`);
}

function describeNewerVersion(data: object, i18n: I18n): string {
	const found = readNumber(data, 'found');
	const supported = readNumber(data, 'supported');
	if (found === null || supported === null) {
		return i18n._(msg`This file was written by a newer version of Tunneler.`);
	}
	return i18n._(
		msg`This file was written by a newer version of Tunneler (format ${found}, this version reads up to ${supported}).`
	);
}

function describeUnsafeCharacters(data: object, i18n: I18n): string {
	const entry = readNumber(data, 'entry');
	const field = readText(data, 'field');
	if (entry === null || field === null) {
		return i18n._(
			msg`The file contains control or hidden characters, so it was refused.`
		);
	}
	const position = entry + 1;
	const fieldName = describeField(field, i18n);
	return i18n._(
		msg`Tunnel ${position} of the file has control or hidden characters in the field ${fieldName}, so the whole file was refused.`
	);
}

function describeTooLarge(data: object, i18n: I18n): string {
	const limit = readNumber(data, 'limit');
	if (limit === null) {
		return i18n._(
			msg`This file's plan is too large to show: import a file with fewer tunnels`
		);
	}
	const kibibytes = Math.floor(limit / BYTES_PER_KIBIBYTE);
	return i18n._(msg`The file is larger than ${kibibytes} KiB.`);
}

function describeTooManyEntries(data: object, i18n: I18n): string {
	const limit = readNumber(data, 'limit');
	if (limit === null) return i18n._(msg`The file has too many tunnels.`);
	return i18n._(msg`The file has more than ${limit} tunnels.`);
}

/** The text for an `invalid_import_file` error, by its `reason`; the raw message when the data is unusable. */
export function describeInvalidImportFile(
	error: Readonly<DaemonCommandError>,
	i18n: I18n
): string {
	const { data } = error;
	if (typeof data !== 'object' || data === null) return error.message;
	const reason = readText(data, 'reason');
	if (reason === null || !REASONS.has(reason)) return error.message;
	switch (reason) {
		case 'syntax':
			return describeSyntax(data, i18n);
		case 'notTunnelerFile':
			return i18n._(msg`This is not a Tunneler export or tunnels file.`);
		case 'newerVersion':
			return describeNewerVersion(data, i18n);
		case 'unsafeCharacters':
			return describeUnsafeCharacters(data, i18n);
		case 'tooLarge':
			return describeTooLarge(data, i18n);
		default:
			return describeTooManyEntries(data, i18n);
	}
}
