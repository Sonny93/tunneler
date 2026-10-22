import { msg } from '@lingui/core/macro';
import type { MessageDescriptor } from '@lingui/core';

import { findReservedTagProblem } from '#/shared/reserved_tag_problem';

const TAG_SEPARATOR = ',';
const WHITESPACE_PATTERN = /\s/u;

export const EMPTY_TAG_MESSAGE = msg`Enter a tag name.`;
export const SEPARATOR_TAG_MESSAGE = msg`Only one tag, without a comma.`;
export const WHITESPACE_TAG_MESSAGE = msg`The tag cannot contain spaces.`;
export const UNCHANGED_TAG_MESSAGE = msg`Choose a different name.`;

/** The error of a new tag name, or `null` when it can be sent to the daemon. */
export function validateTagName(
	text: string,
	currentTag: string
): MessageDescriptor | null {
	const name = text.trim();
	if (name === '') return EMPTY_TAG_MESSAGE;
	if (name.includes(TAG_SEPARATOR)) return SEPARATOR_TAG_MESSAGE;
	if (WHITESPACE_PATTERN.test(name)) return WHITESPACE_TAG_MESSAGE;
	const reservedProblem = findReservedTagProblem(name);
	if (reservedProblem !== null) return reservedProblem;
	if (name === currentTag) return UNCHANGED_TAG_MESSAGE;
	return null;
}
