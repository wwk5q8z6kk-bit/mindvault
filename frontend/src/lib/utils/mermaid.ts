let mermaidModule: Promise<typeof import('mermaid')> | null = null;

async function loadMermaid() {
	if (!mermaidModule) {
		mermaidModule = import('mermaid');
	}
	const mod = await mermaidModule;
	return mod.default ?? mod;
}

export async function renderMermaid(container: HTMLElement): Promise<void> {
	const mermaid = await loadMermaid();
	mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' });
	const nodes = container.querySelectorAll<HTMLElement>('.mermaid');
	if (!nodes.length) return;
	await mermaid.run({ nodes });
}
