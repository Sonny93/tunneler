import type { MessageDescriptor } from '@lingui/core';

import { parseTags } from '#/tunnel_form/parse_tags';
import { findReservedTagProblem } from '#/shared/reserved_tag_problem';
import { WHITESPACE_TAG_MESSAGE } from '#/tag_rename/validate_tag_name';

const WHITESPACE_PATTERN = /\s/u;

/** The error of the first invalid tag of a comma-separated text, or `null`. */
export function validateTags(text: string): MessageDescriptor | null {
	for (const tag of parseTags(text)) {
		if (WHITESPACE_PATTERN.test(tag)) return WHITESPACE_TAG_MESSAGE;
		const reservedProblem = findReservedTagProblem(tag);
		if (reservedProblem !== null) return reservedProblem;
	}
	return null;
}
