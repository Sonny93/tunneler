import { useLingui } from '@lingui/react/macro';
import { useId, useState, type ChangeEvent, type FormEvent } from 'react';
import { Button, Input, ModalFooter, RadioOptions } from '@minimalstuff/ui';

import { useDaemonStore } from '#/state/daemon_store';
import type { CredentialAnswer } from '#/daemon/daemon_gateway';
import type { CredentialRequestDto, RememberDto } from '#/daemon/protocol';
import {
	DEFAULT_REMEMBER_CHOICE,
	buildCredentialAnswer,
	canRemember,
	getCredentialDescription,
	getCredentialFields,
	getCredentialSubmitLabel,
	getRememberOptions,
	toRemember,
	type RememberChoice,
} from '#/prompts/credential_prompt_text';

type CredentialPromptContentProps = {
	request: CredentialRequestDto;
	onSubmit: (answer: CredentialAnswer, remember?: RememberDto) => void;
	onCancel: () => void;
};

/** Secrets live only in this component's state and vanish when the dialog closes. */
export function CredentialPromptContent({
	request,
	onSubmit,
	onCancel,
}: Readonly<CredentialPromptContentProps>) {
	const { t, i18n } = useLingui();
	const formId = useId();
	const submitLabel = getCredentialSubmitLabel(request, i18n);
	const fields = getCredentialFields(request, i18n);
	const description = getCredentialDescription(request, i18n);
	const isKeyringEnabled = useDaemonStore(
		(state) => state.settings?.useOsKeyring ?? false
	);
	const [rememberChoice, setRememberChoice] = useState<RememberChoice>(
		DEFAULT_REMEMBER_CHOICE
	);
	const [answers, setAnswers] = useState<readonly string[]>(() =>
		fields.map(() => '')
	);

	const changeAnswer =
		(position: number) => (event: ChangeEvent<HTMLInputElement>) =>
			setAnswers((current) =>
				current.map((answer, index) =>
					index === position ? event.target.value : answer
				)
			);
	const rememberOptions = getRememberOptions(isKeyringEnabled, i18n);
	const changeRememberChoice = (selected: string) => {
		const option = rememberOptions.find(({ value }) => value === selected);
		if (option) setRememberChoice(option.value);
	};
	const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		const answer = buildCredentialAnswer(request, answers);
		if (!canRemember(request)) {
			onSubmit(answer);
			return;
		}
		onSubmit(answer, toRemember(rememberChoice));
	};

	return (
		<>
			<form id={formId} onSubmit={handleSubmit} className="flex flex-col gap-4">
				{description && <p>{description}</p>}
				{fields.map((field, position) => (
					<Input
						key={field.id}
						label={field.label}
						type={field.isSecret ? 'password' : 'text'}
						value={answers[position] ?? ''}
						onChange={changeAnswer(position)}
						autoComplete="off"
					/>
				))}
				{canRemember(request) && (
					<RadioOptions
						name="remember"
						label={t`Remember the passphrase`}
						options={rememberOptions}
						value={rememberChoice}
						onChange={changeRememberChoice}
					/>
				)}
			</form>
			<ModalFooter>
				<Button variant="outline" color="neutral" onClick={onCancel}>
					{t`Cancel`}
				</Button>
				<Button type="submit" form={formId}>
					{submitLabel}
				</Button>
			</ModalFooter>
		</>
	);
}
