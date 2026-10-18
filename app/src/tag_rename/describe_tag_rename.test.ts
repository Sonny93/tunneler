import { describe, expect, it } from 'vitest';

import { describeTagRename } from '#/tag_rename/describe_tag_rename';

describe('describeTagRename', () => {
	it('should pluralize the renamed tunnels', () => {
		expect(describeTagRename('dev', 'staging', 3)).toBe(
			'Tag dev renommé en staging (3 tunnels)'
		);
	});

	it('should keep the singular for one tunnel', () => {
		expect(describeTagRename('dev', 'staging', 1)).toBe(
			'Tag dev renommé en staging (1 tunnel)'
		);
	});
});
