import type { MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';

const WHITESPACE_PATTERN = /\s/u;
const CONTROL_CHARACTER_PATTERN = /\p{Cc}/u;

const MISSING_SLASH_MESSAGE = msg`The path must start with "/".`;
const DOUBLE_SLASH_MESSAGE = msg`The path must not start with "//".`;
const FORBIDDEN_CHARACTER_MESSAGE = msg`The path must not contain spaces or control characters.`;

/** The error of an open path, or `null` when it is valid; blank means the default `/`. Same rules as the engine. */
export function validateOpenPath(text: string): MessageDescriptor | null {
	const path = text.trim();
	if (path === '') return null;
	if (!path.startsWith('/')) return MISSING_SLASH_MESSAGE;
	if (path.startsWith('//')) return DOUBLE_SLASH_MESSAGE;
	if (WHITESPACE_PATTERN.test(path) || CONTROL_CHARACTER_PATTERN.test(path)) {
		return FORBIDDEN_CHARACTER_MESSAGE;
	}
	return null;
}
