import { useMemo, useSyncExternalStore } from 'react';

const LANGUAGE_CHANGE_EVENT = 'languagechange';
const LANGUAGES_SEPARATOR = ',';

function subscribe(onChange: () => void): () => void {
	window.addEventListener(LANGUAGE_CHANGE_EVENT, onChange);
	return () => window.removeEventListener(LANGUAGE_CHANGE_EVENT, onChange);
}

function readLanguages(): string {
	return navigator.languages.join(LANGUAGES_SEPARATOR);
}

/** `navigator.languages`, re-read when the OS language list changes. */
export function usePreferredLanguages(): readonly string[] {
	const languages = useSyncExternalStore(subscribe, readLanguages);
	return useMemo(() => languages.split(LANGUAGES_SEPARATOR), [languages]);
}
