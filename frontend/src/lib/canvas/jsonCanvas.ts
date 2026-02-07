export type CanvasCardBase = {
	id: string;
	title: string;
	content: string;
	x: number;
	y: number;
	width: number;
	height: number;
	color?: string;
};

export type JsonCanvasNode = {
	id: string;
	type: 'text' | 'file' | 'link' | 'group';
	x: number;
	y: number;
	width: number;
	height: number;
	text?: string;
	color?: string;
};

export type JsonCanvasEdge = {
	id: string;
	fromNode: string;
	toNode: string;
	fromSide?: 'top' | 'right' | 'bottom' | 'left';
	toSide?: 'top' | 'right' | 'bottom' | 'left';
	label?: string;
	color?: string;
};

export type JsonCanvasDocument = {
	nodes: JsonCanvasNode[];
	edges: JsonCanvasEdge[];
};

function normalizeTitle(value: string): string {
	return value.trim().toLowerCase();
}

export function extractTitleFromText(text: string): string | null {
	if (!text) return null;
	const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
	if (lines.length === 0) return null;
	const first = lines[0];
	const heading = first.match(/^#{1,6}\s+(.*)$/);
	if (heading && heading[1]) return heading[1].trim();
	return first.length > 0 ? first : null;
}

export function exportToJsonCanvas(cards: CanvasCardBase[]): JsonCanvasDocument {
	const nodes: JsonCanvasNode[] = cards.map((card) => ({
		id: card.id,
		type: 'text',
		x: card.x,
		y: card.y,
		width: card.width,
		height: card.height,
		text: `# ${card.title}\n\n${card.content ?? ''}`.trim(),
		color: card.color
	}));
	return { nodes, edges: [] };
}

export function applyJsonCanvasLayout<T extends CanvasCardBase>(
	cards: T[],
	doc: JsonCanvasDocument
): { cards: T[]; matched: number; skipped: number } {
	const byId = new Map<string, T>();
	const byTitle = new Map<string, T>();
	for (const card of cards) {
		byId.set(card.id, card);
		byTitle.set(normalizeTitle(card.title), card);
	}

	let matched = 0;
	let skipped = 0;
	const matchedIds = new Set<string>();
	const updated = cards.map((card) => ({ ...card })) as T[];
	const updatedById = new Map(updated.map((card) => [card.id, card]));

	for (const node of doc.nodes) {
		const targetById = updatedById.get(node.id);
		if (targetById && !matchedIds.has(targetById.id)) {
			targetById.x = node.x;
			targetById.y = node.y;
			targetById.width = node.width;
			targetById.height = node.height;
			matchedIds.add(targetById.id);
			matched += 1;
			continue;
		}

		const inferredTitle = node.text ? extractTitleFromText(node.text) : null;
		if (inferredTitle) {
			const targetByTitle = byTitle.get(normalizeTitle(inferredTitle));
			if (targetByTitle && !matchedIds.has(targetByTitle.id)) {
				const updatedCard = updatedById.get(targetByTitle.id);
				if (updatedCard) {
					updatedCard.x = node.x;
					updatedCard.y = node.y;
					updatedCard.width = node.width;
					updatedCard.height = node.height;
					matchedIds.add(updatedCard.id);
					matched += 1;
					continue;
				}
			}
		}

		skipped += 1;
	}

	return { cards: updated, matched, skipped };
}
