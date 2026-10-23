import type { MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';

const PORT_SELECTOR_SEPARATOR = ':';
const ONLY_DIGITS_PATTERN = /^\d+$/u;

export const COLON_TAG_MESSAGE = msg`The tag cannot contain ":" (reserved for the host:port form of the command line).`;
export const DIGITS_TAG_MESSAGE = msg`The tag cannot be made only of digits (the command line would read it as a port).`;

/** The error of a tag the CLI would read as a port selector, or `null`. */
export function findReservedTagProblem(tag: string): MessageDescriptor | null {
	if (tag.includes(PORT_SELECTOR_SEPARATOR)) return COLON_TAG_MESSAGE;
	if (ONLY_DIGITS_PATTERN.test(tag)) return DIGITS_TAG_MESSAGE;
	return null;
}
