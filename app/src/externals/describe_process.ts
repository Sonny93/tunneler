import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import type { ExternalProcessDto, ProcessOwnerDto } from '#/daemon/protocol';

/** Example: `ssh (pid 4242)`. */
export function describeProcess({
	process,
	pid,
}: Readonly<ProcessOwnerDto>): string {
	return `${process} (pid ${pid})`;
}

/** Example: `ssh to nova (pid 4242)`, or `ssh (pid 4242)` without a known destination. */
export function describeProcessHeading(
	{ process, pid, destination }: Readonly<ExternalProcessDto>,
	i18n: I18n
): string {
	if (!destination) return t(i18n)`${process} (pid ${pid})`;
	return t(i18n)`${process} to ${destination} (pid ${pid})`;
}
