import { describe, expect, it } from 'vitest';

import {
	describeTagDeletion,
	describeTagDeletionWarning,
} from '#/tag_delete/describe_tag_deletion';

describe('describeTagDeletionWarning', () => {
	it('should pluralize the tunnels that lose the tag', () => {
		expect(describeTagDeletionWarning(3, false)).toBe(
			'Le tag sera retiré de 3 tunnels. Les tunnels sont conservés.'
		);
	});

	it('should keep the singular for one tunnel', () => {
		expect(describeTagDeletionWarning(1, false)).toBe(
			'Le tag sera retiré de 1 tunnel. Les tunnels sont conservés.'
		);
	});

	it('should mention the launch tags when the tag is one of them', () => {
		expect(describeTagDeletionWarning(2, true)).toBe(
			'Le tag sera retiré de 2 tunnels. Les tunnels sont conservés. Il sera aussi retiré des « Tags démarrés au lancement ».'
		);
	});
});

describe('describeTagDeletion', () => {
	it('should pluralize the tunnels that lost the tag', () => {
		expect(describeTagDeletion('dev', 3)).toBe('Tag dev retiré de 3 tunnels');
	});

	it('should keep the singular for one tunnel', () => {
		expect(describeTagDeletion('dev', 1)).toBe('Tag dev retiré de 1 tunnel');
	});
});
