const BARE_TOML_KEY_PATTERN = /^[A-Za-z0-9_-]+$/;
const DEVSLOT_PATH_PLACEHOLDER = '/path/to/ds';

function quoteTomlKey(key: string): string {
	if (BARE_TOML_KEY_PATTERN.test(key)) return key;
	const escaped = key.replaceAll('\\', '\\\\').replaceAll('"', '\\"');
	return `"${escaped}"`;
}

/** The TOML to add to the configuration file so that the host knows where `ds` is. */
export function buildDevslotConfigSnippet(host: string): string {
	return `[hosts.${quoteTomlKey(host)}]\ndevslot_path = "${DEVSLOT_PATH_PLACEHOLDER}"`;
}
