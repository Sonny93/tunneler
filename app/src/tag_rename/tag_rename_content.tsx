import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { Button, Input, ModalFooter } from '@minimalstuff/ui';
import {
	useId,
	useRef,
	useState,
	type ChangeEvent,
	type FormEvent,
} from 'react';

import {
	toDaemonCommandError,
	type DaemonCommandError,
} from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { getSubmissionErrorMessages } from '#/shared/submission_error_messages';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { describeTagRename } from '#/tag_rename/describe_tag_rename';
import { validateTagName } from '#/tag_rename/validate_tag_name';

const VALIDATION_FAILED_CODE = 'validation_failed';

type TagRenameContentProps = {
	tag: string;
	/** Whether the sidebar row of the new tag must take the focus once renamed. */
	isFocusingRow: boolean;
};

function describeMerge(name: string, i18n: I18n): string {
	return t(i18n)`The tag ${name} already exists: the two will be merged.`;
}

/** Content of the rename dialog: one text field prefilled with the current name. */
export function TagRenameContent({
	tag,
	isFocusingRow,
}: Readonly<TagRenameContentProps>) {
	const { i18n } = useLingui();
	const formId = useId();
	const inputRef = useRef<HTMLInputElement>(null);
	const gateway = useDaemonGateway();
	const tunnels = useDaemonStore((state) => state.tunnels);
	const summary = useFormErrorSummary();
	const [name, setName] = useState(tag);
	const [error, setError] = useState<string | undefined>();
	const [isSaving, setIsSaving] = useState(false);
	const trimmedName = name.trim();
	const isMerge =
		trimmedName !== tag &&
		tunnels.some(({ tags }) => tags.includes(trimmedName));

	const rejectName = (messages: readonly string[]) => {
		setError(messages.join(' '));
		summary.reportMessages(messages);
		inputRef.current?.focus();
	};

	const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
		setName(event.target.value);
		setError(undefined);
		summary.clearFailure();
	};

	const reportSuccess = (renamedCount: number) => {
		const ui = useUiStore.getState();
		useToastStore.getState().pushToast({
			tone: 'success',
			message: describeTagRename(tag, trimmedName, renamedCount, i18n),
		});
		if (ui.filter.kind === 'tag' && ui.filter.tag === tag) {
			ui.setFilter({ kind: 'tag', tag: trimmedName });
		}
		if (isFocusingRow) ui.focusTagRow(trimmedName);
		ui.closeTagRename();
	};

	const reportFailure = (failure: Readonly<DaemonCommandError>) => {
		if (failure.code === VALIDATION_FAILED_CODE) {
			rejectName(getSubmissionErrorMessages(failure, i18n));
			return;
		}
		const failureMessage = t(i18n)`Couldn't rename the tag`;
		const errorMessage = describeDaemonError(failure, i18n);
		useToastStore.getState().pushToast({
			tone: 'error',
			message: t(i18n)`${failureMessage}: ${errorMessage}`,
		});
	};

	const rename = async () => {
		setIsSaving(true);
		try {
			reportSuccess(await gateway.renameTag(tag, trimmedName));
		} catch (rejection) {
			reportFailure(toDaemonCommandError(rejection));
		} finally {
			setIsSaving(false);
		}
	};

	const handleCancel = () => useUiStore.getState().closeTagRename();

	const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (isSaving) return;
		const validationError = validateTagName(name, tag);
		if (validationError !== null) {
			rejectName([i18n._(validationError)]);
			return;
		}
		void rename();
	};

	return (
		<>
			<form id={formId} noValidate onSubmit={handleSubmit}>
				<FormErrorSummary
					messages={summary.messages}
					attach={summary.attachSummary}
				/>
				<Input
					ref={inputRef}
					label={t(i18n)`New tag name`}
					value={name}
					onChange={handleChange}
					error={error}
					description={isMerge ? describeMerge(trimmedName, i18n) : undefined}
					autoComplete="off"
					required
				/>
			</form>
			<ModalFooter>
				<Button variant="outline" color="neutral" onClick={handleCancel}>
					{t(i18n)`Cancel`}
				</Button>
				<Button type="submit" form={formId} loading={isSaving}>
					{t(i18n)`Rename`}
				</Button>
			</ModalFooter>
		</>
	);
}
