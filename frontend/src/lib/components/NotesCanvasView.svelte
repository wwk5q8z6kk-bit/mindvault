<script lang="ts">
	import { onMount } from 'svelte';
	import { listNodes } from '$lib/api/nodes';
	import type { KnowledgeNode } from '$lib/api/types';
	import { pushToast } from '$lib/stores/toast';
	import {
		applyJsonCanvasLayout,
		exportToJsonCanvas,
		type JsonCanvasDocument
	} from '$lib/canvas/jsonCanvas';
	import { kindColor, kindLabel } from '$lib/utils/kind-helpers';

	type CanvasCard = {
		id: string;
		title: string;
		kind: string;
		content: string;
		x: number;
		y: number;
		width: number;
		height: number;
		color: string;
	};

	let cards: CanvasCard[] = [];
	let loading = true;
	let dragging: string | null = null;
	let dragOffset = { x: 0, y: 0 };
	let canvasEl: HTMLDivElement | null = null;
	let pan = { x: 0, y: 0 };
	let isPanning = false;
	let panStart = { x: 0, y: 0 };
	let zoom = 1;
	let showGrid = localStorage.getItem('mv_canvas_grid') !== 'false';
	let snapToGrid = localStorage.getItem('mv_canvas_snap') === 'true';
	let gridSize = 20;
	let selectedCard: string | null = null;
	let canvasFileInput: HTMLInputElement | null = null;

	onMount(async () => {
		try {
			const nodes = await listNodes({ limit: 50 });
			const cols = Math.ceil(Math.sqrt(nodes.length));
			cards = nodes.map((node, i) => ({
				id: node.id,
				title: node.title || 'Untitled',
				kind: node.kind,
				content: (node.content ?? '').slice(0, 200),
				x: (i % cols) * 280 + 40,
				y: Math.floor(i / cols) * 200 + 40,
				width: 240,
				height: 160,
				color: kindColor(node.kind)
			}));
			// Load saved positions
			const saved = localStorage.getItem('mv_canvas_positions');
			if (saved) {
				try {
					const positions: Record<string, { x: number; y: number }> = JSON.parse(saved);
					for (const card of cards) {
						if (positions[card.id]) {
							card.x = positions[card.id].x;
							card.y = positions[card.id].y;
						}
					}
				} catch { /* ignore corrupt data */ }
			}
			loading = false;
		} catch {
			loading = false;
			pushToast('Failed to load canvas', 'danger');
		}
	});

	function savePositions() {
		const positions: Record<string, { x: number; y: number }> = {};
		for (const card of cards) {
			positions[card.id] = { x: card.x, y: card.y };
		}
		localStorage.setItem('mv_canvas_positions', JSON.stringify(positions));
	}

	function handleCardMouseDown(event: MouseEvent, cardId: string) {
		if (event.button !== 0) return;
		event.stopPropagation();
		dragging = cardId;
		selectedCard = cardId;
		const card = cards.find((c) => c.id === cardId);
		if (card) {
			dragOffset.x = event.clientX / zoom - card.x;
			dragOffset.y = event.clientY / zoom - card.y;
		}
	}

	function handleCanvasMouseDown(event: MouseEvent) {
		if (event.button === 0 && !dragging) {
			selectedCard = null;
		}
		if (event.button === 1 || (event.button === 0 && event.altKey)) {
			isPanning = true;
			panStart = { x: event.clientX - pan.x, y: event.clientY - pan.y };
		}
	}

	function handleMouseMove(event: MouseEvent) {
		if (dragging) {
			const card = cards.find((c) => c.id === dragging);
			if (card) {
				let newX = event.clientX / zoom - dragOffset.x;
				let newY = event.clientY / zoom - dragOffset.y;
				if (snapToGrid) {
					newX = Math.round(newX / gridSize) * gridSize;
					newY = Math.round(newY / gridSize) * gridSize;
				}
				card.x = newX;
				card.y = newY;
				cards = [...cards];
			}
		}
		if (isPanning) {
			pan = {
				x: event.clientX - panStart.x,
				y: event.clientY - panStart.y
			};
		}
	}

	function handleMouseUp() {
		if (dragging) {
			savePositions();
			dragging = null;
		}
		isPanning = false;
	}

	function handleWheel(event: WheelEvent) {
		event.preventDefault();
		const delta = event.deltaY > 0 ? -0.05 : 0.05;
		zoom = Math.max(0.3, Math.min(2, zoom + delta));
	}

	function autoArrange() {
		const cols = Math.ceil(Math.sqrt(cards.length));
		cards = cards.map((card, i) => ({
			...card,
			x: (i % cols) * 280 + 40,
			y: Math.floor(i / cols) * 200 + 40
		}));
		savePositions();
		pan = { x: 0, y: 0 };
		zoom = 1;
	}

	function resetView() {
		pan = { x: 0, y: 0 };
		zoom = 1;
	}

	function toggleGrid() {
		showGrid = !showGrid;
		localStorage.setItem('mv_canvas_grid', String(showGrid));
	}

	function toggleSnap() {
		snapToGrid = !snapToGrid;
		localStorage.setItem('mv_canvas_snap', String(snapToGrid));
	}

	function cardLink(card: CanvasCard): string {
		if (card.kind === 'task') return `/tasks?task=${card.id}`;
		if (card.kind === 'fact') return `/notes?note=${card.id}`;
		return `/search?q=${encodeURIComponent(card.title)}`;
	}

	function downloadJsonCanvas() {
		const doc = exportToJsonCanvas(cards);
		const blob = new Blob([JSON.stringify(doc, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `mindvault-${new Date().toISOString().slice(0, 10)}.canvas`;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);
		pushToast('Canvas exported.', 'success');
	}

	async function handleCanvasImport(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		try {
			const raw = await file.text();
			const parsed = JSON.parse(raw) as JsonCanvasDocument;
			if (!parsed || !Array.isArray(parsed.nodes)) {
				throw new Error('Invalid JSON Canvas file');
			}
			const result = applyJsonCanvasLayout(cards, parsed);
			cards = result.cards;
			savePositions();
			pushToast(`Canvas import complete (${result.matched} matched, ${result.skipped} skipped).`, 'success');
		} catch {
			pushToast('Canvas import failed. Check the file format.', 'danger');
		} finally {
			if (canvasFileInput) canvasFileInput.value = '';
		}
	}
</script>

<svelte:window on:mousemove={handleMouseMove} on:mouseup={handleMouseUp} />

<div class="flex flex-col gap-3">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Canvas</h2>
			<p class="text-xs text-slate-400">{cards.length} cards &middot; Alt+drag to pan &middot; Scroll to zoom</p>
		</div>
		<div class="flex items-center gap-2">
			<label class="rounded-lg border border-slate-700 px-2.5 py-1 text-[10px] text-slate-300 hover:bg-slate-800">
				Import .canvas
				<input
					type="file"
					accept=".canvas,.json"
					class="hidden"
					bind:this={canvasFileInput}
					on:change={handleCanvasImport}
				/>
			</label>
			<button
				class="rounded-lg border border-slate-700 px-2.5 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
				on:click={downloadJsonCanvas}
			>
				Export .canvas
			</button>
			<label class="flex items-center gap-1.5 text-[10px] text-slate-400">
				<input type="checkbox" checked={showGrid} on:change={toggleGrid} class="rounded border-slate-600" />
				Grid
			</label>
			<label class="flex items-center gap-1.5 text-[10px] text-slate-400">
				<input type="checkbox" checked={snapToGrid} on:change={toggleSnap} class="rounded border-slate-600" />
				Snap
			</label>
			<button
				class="rounded-lg border border-slate-700 px-2.5 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
				on:click={autoArrange}
			>
				Auto-arrange
			</button>
			<button
				class="rounded-lg border border-slate-700 px-2.5 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
				on:click={resetView}
			>
				Reset view
			</button>
			<span class="text-[10px] text-slate-500">{Math.round(zoom * 100)}%</span>
		</div>
	</div>

	{#if loading}
		<div class="flex h-[70vh] items-center justify-center rounded-2xl border border-slate-800 bg-slate-950">
			<p class="text-xs text-slate-400">Loading canvas...</p>
		</div>
	{:else}
		<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
		<div
			class="relative h-[70vh] overflow-hidden rounded-2xl border border-slate-800/60 bg-slate-950"
			bind:this={canvasEl}
			on:mousedown={handleCanvasMouseDown}
			on:wheel|preventDefault={handleWheel}
			role="application"
			aria-label="Canvas workspace"
		>
			{#if showGrid}
				<svg class="absolute inset-0 h-full w-full pointer-events-none" style="opacity: 0.08">
					<defs>
						<pattern id="grid" width={gridSize * zoom} height={gridSize * zoom} patternUnits="userSpaceOnUse"
							x={pan.x % (gridSize * zoom)} y={pan.y % (gridSize * zoom)}>
							<path d="M {gridSize * zoom} 0 L 0 0 0 {gridSize * zoom}" fill="none" stroke="white" stroke-width="0.5" />
						</pattern>
					</defs>
					<rect width="100%" height="100%" fill="url(#grid)" />
				</svg>
			{/if}

			<div
				class="absolute inset-0"
				style="transform: translate({pan.x}px, {pan.y}px) scale({zoom}); transform-origin: 0 0;"
			>
				{#each cards as card (card.id)}
					<div
						class="absolute select-none rounded-xl border bg-slate-900/90 shadow-lg transition-shadow {selectedCard === card.id
							? 'border-sky-500/60 shadow-sky-500/20'
							: 'border-slate-700/60 hover:border-slate-600'}"
						style="left: {card.x}px; top: {card.y}px; width: {card.width}px; min-height: {card.height}px;"
						on:mousedown={(e) => handleCardMouseDown(e, card.id)}
						role="button"
						tabindex="0"
						on:keydown={(e) => { if (e.key === 'Enter') window.location.href = cardLink(card); }}
						on:dblclick={() => { window.location.href = cardLink(card); }}
					>
						<div class="flex items-center gap-2 border-b border-slate-700/40 px-3 py-2">
							<div class="h-2.5 w-2.5 rounded-full" style="background-color: {card.color}"></div>
							<span class="text-[9px] font-medium uppercase tracking-wider" style="color: {card.color}">{kindLabel(card.kind)}</span>
						</div>
						<div class="p-3">
							<h4 class="text-xs font-semibold text-white leading-tight">{card.title}</h4>
							{#if card.content}
								<p class="mt-1.5 text-[10px] leading-relaxed text-slate-400 line-clamp-4">{card.content}</p>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
