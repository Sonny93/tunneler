import { useLingui } from '@lingui/react/macro';

import { buildDevslotLinkModalRequest } from '#/devslot_link/build_devslot_link_modal_request';
import { useModalCall } from '#/shared/use_modal_call';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';

/** Shows the dialog of the pending `tunneler://` link; a `<ModalProvider />` must be mounted. */
export function DevslotLinkHost(): null {
	const { i18n } = useLingui();
	const request = useUiStore((state) => state.devslotLinkRequest);
	const hosts = useDaemonStore((state) => state.hosts);
	const closeDevslotLink = useUiStore((state) => state.closeDevslotLink);

	const modalRequest =
		request === null
			? null
			: buildDevslotLinkModalRequest(request, hosts, i18n);
	useModalCall(modalRequest, () => {
		if (request !== null) closeDevslotLink(request.id);
	});

	return null;
}
