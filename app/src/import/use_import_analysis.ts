import { useState } from 'react';

import type { ImportPreviewDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';

export type ImportAnalysis = { preview: ImportPreviewDto; revision: number };

type UseImportAnalysisReturn = {
	/** `revision` changes with every successful analysis, to reset what depends on the preview. */
	analysis: ImportAnalysis | null;
	isAnalyzing: boolean;
	summary: ReturnType<typeof useFormErrorSummary>;
	analyzeScript: (text: string) => void;
	analyzeSshConfig: () => void;
};

export function useImportAnalysis(): UseImportAnalysisReturn {
	const gateway = useDaemonGateway();
	const [analysis, setAnalysis] = useState<ImportAnalysis | null>(null);
	const [isAnalyzing, setIsAnalyzing] = useState(false);
	const summary = useFormErrorSummary();

	const analyze = async (load: () => Promise<ImportPreviewDto>) => {
		if (isAnalyzing) return;
		setIsAnalyzing(true);
		summary.clearFailure();
		try {
			const preview = await load();
			setAnalysis((previous) => ({
				preview,
				revision: (previous?.revision ?? 0) + 1,
			}));
		} catch (rejection) {
			setAnalysis(null);
			summary.reportFailure(rejection);
		} finally {
			setIsAnalyzing(false);
		}
	};

	const analyzeScript = (text: string) =>
		void analyze(() => gateway.importScript(text));
	const analyzeSshConfig = () => void analyze(() => gateway.importSshConfig());

	return { analysis, isAnalyzing, summary, analyzeScript, analyzeSshConfig };
}
