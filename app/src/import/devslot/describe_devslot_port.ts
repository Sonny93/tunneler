import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { DevslotPortDto } from '#/daemon/protocol';
import { isWebPort } from '#/import/devslot/devslot_ports';

const SENTENCE_SEPARATOR = '. ';

function describeKind(port: Readonly<DevslotPortDto>, i18n: I18n): string {
	if (isWebPort(port)) {
		const url = port.url ?? '';
		return url === '' ? i18n._(msg`Web`) : i18n._(msg`Web: ${url}`);
	}
	const service = port.service ?? '';
	const target = port.target;
	if (service !== '' && target !== undefined) {
		return i18n._(msg`Service ${service}, target ${target}`);
	}
	return service === ''
		? i18n._(msg`Service`)
		: i18n._(msg`Service ${service}`);
}

/** Plain text for the checkbox of a port: its kind and address, then what makes it special. */
export function describeDevslotPort(
	port: Readonly<DevslotPortDto>,
	i18n: I18n
): string {
	const sentences = [
		describeKind(port, i18n),
		port.forced
			? i18n._(msg`Forced port, may be shared between instances`)
			: '',
		port.status === 'ambiguous' ? i18n._(msg`Listed by several instances`) : '',
		port.status === 'alreadyTunneled'
			? i18n._(msg`Not selectable: a tunnel already forwards to this port`)
			: '',
	];
	return sentences
		.filter((sentence) => sentence !== '')
		.join(SENTENCE_SEPARATOR);
}
