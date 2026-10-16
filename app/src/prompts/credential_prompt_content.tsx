import { Button, Input, ModalFooter } from '@minimalstuff/ui';
import { useId, useState, type ChangeEvent, type FormEvent } from 'react';

import type { CredentialRequestDto } from '#/daemon/protocol';
import type { CredentialAnswer } from '#/daemon/daemon_gateway';
import {
	buildCredentialAnswer,
	getCredentialDescription,
	getCredentialFields,
	getCredentialSubmitLabel,
} from '#/prompts/credential_prompt_text';

type CredentialPromptContentProps = {
	request: CredentialRequestDto;
	onSubmit: (answer: CredentialAnswer) => void;
	onCancel: () => void;
};

/** Secrets live only in this component's state and vanish when the dialog closes. */
export function CredentialPromptContent({
	request,
	onSubmit,
	onCancel,
}: Readonly<CredentialPromptContentProps>) {
	const formId = useId();
	const fields = getCredentialFields(request);
	const description = getCredentialDescription(request);
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
	const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		onSubmit(buildCredentialAnswer(request, answers));
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
			</form>
			<ModalFooter>
				<Button variant="outline" color="neutral" onClick={onCancel}>
					Annuler
				</Button>
				<Button type="submit" form={formId}>
					{getCredentialSubmitLabel(request)}
				</Button>
			</ModalFooter>
		</>
	);
}
