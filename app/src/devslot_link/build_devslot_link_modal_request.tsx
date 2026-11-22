import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';
import { createRef } from 'react';

import { DevslotLinkConfirmContent } from '#/devslot_link/devslot_link_confirm_content';
import { DevslotLinkProblemContent } from '#/devslot_link/devslot_link_problem_content';
import type { ModalCallRequest } from '#/shared/use_modal_call';
import type { DevslotLinkRequest } from '#/state/ui_store';

function buildProblemRequest(
	requestId: number,
	message: string,
	i18n: I18n
): ModalCallRequest {
	const closeButtonRef = createRef<HTMLButtonElement>();
	return {
		id: `devslot-link:${requestId}`,
		props: {
			title: i18n._(msg`This link cannot be opened`),
			role: 'alertdialog',
			size: 'sm',
			initialFocusRef: closeButtonRef,
			children: (
				<DevslotLinkProblemContent
					requestId={requestId}
					message={message}
					closeButtonRef={closeButtonRef}
				/>
			),
		},
	};
}

/**
 * The lib modal of a link: the question for a valid link to a host of the SSH
 * configuration, an explanation otherwise. `hosts` is the list the app already
 * loaded, so nothing leaves this computer before the user answers.
 */
export function buildDevslotLinkModalRequest(
	{ id, link }: Readonly<DevslotLinkRequest>,
	hosts: readonly string[],
	i18n: I18n
): ModalCallRequest {
	if (link.kind === 'invalid') {
		return buildProblemRequest(
			id,
			i18n._(
				msg`This is not a valid Tunneler devslot link. A devslot link looks like tunneler://devslot/host/instance, with an SSH host of your configuration and the name of a devslot instance.`
			),
			i18n
		);
	}
	const { host, instance } = link;
	if (!hosts.includes(host)) {
		return buildProblemRequest(
			id,
			i18n._(
				msg`The host ${host} is not in your SSH configuration (ssh_config). Add it there, then open the link again.`
			),
			i18n
		);
	}
	const cancelButtonRef = createRef<HTMLButtonElement>();
	return {
		id: `devslot-link:${id}`,
		props: {
			title: i18n._(msg`Open a devslot instance`),
			role: 'alertdialog',
			size: 'md',
			initialFocusRef: cancelButtonRef,
			children: (
				<DevslotLinkConfirmContent
					requestId={id}
					host={host}
					instance={instance}
					cancelButtonRef={cancelButtonRef}
				/>
			),
		},
	};
}
