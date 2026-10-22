import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import {
	describeTagDeletion,
	describeTagDeletionWarning,
} from '#/tag_delete/describe_tag_deletion';

describe('describeTagDeletionWarning', () => {
	it('should pluralize the tunnels that lose the tag', () => {
		expect(describeTagDeletionWarning(3, false, i18n)).toBe(
			'The tag will be removed from 3 tunnels. The tunnels are kept.'
		);
	});

	it('should keep the singular for one tunnel', () => {
		expect(describeTagDeletionWarning(1, false, i18n)).toBe(
			'The tag will be removed from 1 tunnel. The tunnels are kept.'
		);
	});

	it('should mention the launch tags when the tag is one of them', () => {
		expect(describeTagDeletionWarning(2, true, i18n)).toBe(
			'The tag will be removed from 2 tunnels. The tunnels are kept. It will also be removed from the "Tags to start on launch".'
		);
	});
});

describe('describeTagDeletion', () => {
	it('should pluralize the tunnels that lost the tag', () => {
		expect(describeTagDeletion('dev', 3, i18n)).toBe(
			'Tag dev removed from 3 tunnels'
		);
	});

	it('should keep the singular for one tunnel', () => {
		expect(describeTagDeletion('dev', 1, i18n)).toBe(
			'Tag dev removed from 1 tunnel'
		);
	});
});
