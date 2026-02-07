<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { db, type Flashcard } from '$lib/db';
	import { assistTransform } from '$lib/api/assist';
	import { notesStore, loadNotes } from '$lib/stores/notes';
	import { pushToast } from '$lib/stores/toast';

	let cards: Flashcard[] = [];
	let dueCards: Flashcard[] = [];
	let currentCard: Flashcard | null = null;
	let showAnswer = false;
	let mode: 'review' | 'browse' | 'generate' = 'review';
	let generating = false;
	let selectedNoteId = '';
	let stats = { total: 0, due: 0, reviewed: 0 };

	// Editing state
	let editingCardId: string | null = null;
	let editFront = '';
	let editBack = '';

	$: stats = {
		total: cards.length,
		due: dueCards.length,
		reviewed: cards.filter((c) => c.lastReviewedAt).length
	};

	function handleKeydown(e: KeyboardEvent) {
		if (mode !== 'review' || !currentCard) return;
		const target = e.target as HTMLElement;
		if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT') return;

		if (e.key === ' ') {
			e.preventDefault();
			if (!showAnswer) {
				showAnswer = true;
			}
		} else if (showAnswer) {
			if (e.key === '1') {
				rateCard(0);
			} else if (e.key === '2') {
				rateCard(2);
			} else if (e.key === '3') {
				rateCard(3);
			} else if (e.key === '4') {
				rateCard(5);
			}
		}
	}

	onMount(async () => {
		await loadCards();
		await loadNotes();
		startReview();
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		if (typeof window !== 'undefined') {
			window.removeEventListener('keydown', handleKeydown);
		}
	});

	async function loadCards() {
		cards = await db.flashcards.toArray();
		const now = new Date().toISOString();
		dueCards = cards
			.filter((c) => c.nextReviewAt <= now)
			.sort((a, b) => a.nextReviewAt.localeCompare(b.nextReviewAt));
	}

	function startReview() {
		mode = 'review';
		showAnswer = false;
		currentCard = dueCards[0] ?? null;
	}

	// SM-2 Algorithm
	function sm2(card: Flashcard, quality: number): Partial<Flashcard> {
		// quality: 0-5 (0=complete blackout, 5=perfect)
		let { easeFactor, interval, repetitions } = card;

		if (quality >= 3) {
			if (repetitions === 0) {
				interval = 1;
			} else if (repetitions === 1) {
				interval = 6;
			} else {
				interval = Math.round(interval * easeFactor);
			}
			repetitions++;
		} else {
			repetitions = 0;
			interval = 1;
		}

		easeFactor = easeFactor + (0.1 - (5 - quality) * (0.08 + (5 - quality) * 0.02));
		if (easeFactor < 1.3) easeFactor = 1.3;

		const nextDate = new Date();
		nextDate.setDate(nextDate.getDate() + interval);

		return {
			interval,
			repetitions,
			easeFactor,
			nextReviewAt: nextDate.toISOString(),
			lastReviewedAt: new Date().toISOString()
		};
	}

	async function rateCard(quality: number) {
		if (!currentCard) return;
		const updates = sm2(currentCard, quality);
		await db.flashcards.update(currentCard.id, updates);
		await loadCards();
		showAnswer = false;
		currentCard = dueCards[0] ?? null;
		if (!currentCard) {
			pushToast('Review complete! All cards done.', 'success');
		}
	}

	async function generateFromNote(noteId: string) {
		if (!noteId || generating) return;
		generating = true;
		try {
			const note = $notesStore.find((n) => n.id === noteId);
			if (!note) return;

			const prompt = `Generate 3-5 flashcards from this note. Each flashcard should test a key concept.
Format EXACTLY as:
Q: [question]
A: [answer]

Note title: ${note.title ?? 'Untitled'}
Content: ${(note.markdown ?? '').slice(0, 1500)}`;

			const result = await assistTransform({ text: prompt, mode: 'refine' });
			const pairs = parseFlashcards(result.transformed_text);

			for (const pair of pairs) {
				const card: Flashcard = {
					id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
					front: pair.front,
					back: pair.back,
					sourceNodeId: noteId,
					sourceTitle: note.title ?? 'Untitled',
					interval: 0,
					repetitions: 0,
					easeFactor: 2.5,
					nextReviewAt: new Date().toISOString(),
					created_at: new Date().toISOString()
				};
				await db.flashcards.put(card);
			}

			await loadCards();
			pushToast(`Generated ${pairs.length} flashcards`, 'success');
			selectedNoteId = '';
		} catch {
			pushToast('Failed to generate flashcards', 'danger');
		} finally {
			generating = false;
		}
	}

	function parseFlashcards(text: string): Array<{ front: string; back: string }> {
		const pairs: Array<{ front: string; back: string }> = [];
		const lines = text.split('\n');
		let front = '';

		for (const line of lines) {
			const qMatch = line.match(/^Q:\s*(.+)/i);
			const aMatch = line.match(/^A:\s*(.+)/i);
			if (qMatch) {
				front = qMatch[1].trim();
			} else if (aMatch && front) {
				pairs.push({ front, back: aMatch[1].trim() });
				front = '';
			}
		}
		return pairs;
	}

	async function addManualCard(front: string, back: string) {
		const card: Flashcard = {
			id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
			front,
			back,
			interval: 0,
			repetitions: 0,
			easeFactor: 2.5,
			nextReviewAt: new Date().toISOString(),
			created_at: new Date().toISOString()
		};
		await db.flashcards.put(card);
		await loadCards();
		pushToast('Card added', 'success');
	}

	async function deleteCard(id: string) {
		await db.flashcards.delete(id);
		await loadCards();
		if (currentCard?.id === id) {
			currentCard = dueCards[0] ?? null;
		}
		pushToast('Card deleted', 'success');
	}

	function startEdit(card: Flashcard) {
		editingCardId = card.id;
		editFront = card.front;
		editBack = card.back;
	}

	function cancelEdit() {
		editingCardId = null;
		editFront = '';
		editBack = '';
	}

	async function saveEdit(cardId: string) {
		if (!editFront.trim() || !editBack.trim()) return;
		await db.flashcards.update(cardId, { front: editFront.trim(), back: editBack.trim() });
		await loadCards();
		editingCardId = null;
		editFront = '';
		editBack = '';
		pushToast('Card updated', 'success');
	}

	let manualFront = '';
	let manualBack = '';
</script>

<div class="mx-auto max-w-3xl">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Flashcards</h2>
			<p class="text-xs text-slate-400">
				{stats.total} cards &middot; {stats.due} due &middot; Spaced repetition (SM-2)
			</p>
		</div>
		<div class="flex rounded-lg border border-slate-700 bg-slate-800 p-0.5">
			{#each [
				{ key: 'review', label: 'Review' },
				{ key: 'browse', label: 'Browse' },
				{ key: 'generate', label: 'Generate' }
			] as tab (tab.key)}
				<button
					class="rounded-md px-3 py-1 text-[10px] font-medium transition {mode === tab.key
						? 'bg-sky-500/20 text-sky-300'
						: 'text-slate-400 hover:text-white'}"
					on:click={() => { mode = tab.key as typeof mode; if (tab.key === 'review') startReview(); }}
				>
					{tab.label}
				</button>
			{/each}
		</div>
	</div>

	{#if mode === 'review'}
		<div class="mt-6">
			{#if dueCards.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-800 p-12 text-center">
					<div class="flex justify-center">
						<div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-500/10 text-emerald-300">
							<svg class="h-7 w-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 13l4 4L19 7" />
							</svg>
						</div>
					</div>
					<h3 class="mt-3 text-sm font-semibold text-white">All caught up!</h3>
					<p class="mt-1 text-xs text-slate-400">No cards due for review. Generate new cards or check back later.</p>
				</div>
			{:else if currentCard}
				<div class="rounded-2xl border border-slate-800/60 bg-slate-900/40 p-8">
					{#if currentCard.sourceTitle}
						<p class="text-[10px] text-slate-500">From: {currentCard.sourceTitle}</p>
					{/if}
					<div class="mt-2 text-center">
						<h3 class="text-lg font-medium text-white">{currentCard.front}</h3>
					</div>

					{#if showAnswer}
						<div class="mt-6 rounded-xl border border-sky-500/20 bg-sky-500/5 p-4 text-center">
							<p class="text-sm text-sky-100">{currentCard.back}</p>
						</div>

						<div class="mt-6">
							<p class="mb-2 text-center text-[10px] text-slate-500">How well did you know this?</p>
							<div class="flex justify-center gap-2">
								{#each [
									{ q: 0, label: 'Forgot', color: 'border-red-500/30 text-red-300 hover:bg-red-500/10' },
									{ q: 2, label: 'Hard', color: 'border-amber-500/30 text-amber-300 hover:bg-amber-500/10' },
									{ q: 3, label: 'Good', color: 'border-sky-500/30 text-sky-300 hover:bg-sky-500/10' },
									{ q: 5, label: 'Easy', color: 'border-emerald-500/30 text-emerald-300 hover:bg-emerald-500/10' }
								] as rating (rating.q)}
									<button
										class="rounded-lg border px-4 py-2 text-xs font-medium transition {rating.color}"
										on:click={() => rateCard(rating.q)}
									>
										{rating.label}
									</button>
								{/each}
							</div>
							<p class="mt-2 text-center text-[9px] text-slate-600">
								Keyboard: 1 Forgot / 2 Hard / 3 Good / 4 Easy
							</p>
						</div>
					{:else}
						<div class="mt-6 text-center">
							<button
								class="rounded-lg bg-sky-500 px-6 py-2 text-sm font-semibold text-white hover:bg-sky-400"
								on:click={() => { showAnswer = true; }}
							>
								Show Answer
							</button>
							<p class="mt-2 text-[9px] text-slate-600">
								Press Space to flip
							</p>
						</div>
					{/if}

					<p class="mt-4 text-center text-[10px] text-slate-600">
						Card {dueCards.indexOf(currentCard) + 1} of {dueCards.length} due
					</p>
				</div>
			{/if}
		</div>
	{:else if mode === 'browse'}
		<div class="mt-4 flex flex-col gap-2">
			{#if cards.length === 0}
				<p class="text-xs text-slate-400">No flashcards yet. Generate some from your notes.</p>
			{:else}
				{#each cards as card (card.id)}
					<div class="rounded-xl border border-slate-800/60 bg-slate-900/40 p-3">
						{#if editingCardId === card.id}
							<div class="flex flex-col gap-2">
								<input
									class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white placeholder-slate-500"
									placeholder="Question (front)"
									bind:value={editFront}
								/>
								<input
									class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white placeholder-slate-500"
									placeholder="Answer (back)"
									bind:value={editBack}
								/>
								<div class="flex gap-2">
									<button
										class="rounded-lg bg-sky-500 px-3 py-1 text-[10px] font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
										disabled={!editFront.trim() || !editBack.trim()}
										on:click={() => saveEdit(card.id)}
									>
										Save
									</button>
									<button
										class="rounded-lg border border-slate-700 px-3 py-1 text-[10px] text-slate-400 hover:text-white"
										on:click={cancelEdit}
									>
										Cancel
									</button>
								</div>
							</div>
						{:else}
							<div class="flex items-start justify-between gap-2">
								<div class="min-w-0 flex-1">
									<p class="text-xs font-medium text-white">{card.front}</p>
									<p class="mt-1 text-[11px] text-slate-400">{card.back}</p>
									{#if card.sourceTitle}
										<p class="mt-1 text-[9px] text-slate-600">Source: {card.sourceTitle}</p>
									{/if}
								</div>
								<div class="flex items-center gap-2">
									<span class="text-[9px] text-slate-500">
										{card.repetitions}x reviewed
									</span>
									<button
										class="text-[10px] text-sky-400 hover:text-sky-300"
										on:click={() => startEdit(card)}
									>
										Edit
									</button>
									<button
										class="text-[10px] text-red-400 hover:text-red-300"
										on:click={() => deleteCard(card.id)}
									>
										Delete
									</button>
								</div>
							</div>
						{/if}
					</div>
				{/each}
			{/if}
		</div>
	{:else}
		<div class="mt-4 flex flex-col gap-4">
			<!-- Generate from note -->
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<h3 class="text-sm font-semibold text-white">Generate from Note</h3>
				<p class="mt-1 text-[10px] text-slate-400">AI creates flashcards from your note content.</p>
				<div class="mt-3 flex gap-2">
					<select
						class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
						bind:value={selectedNoteId}
						aria-label="Select a note"
					>
						<option value="">Select a note...</option>
						{#each $notesStore as note (note.id)}
							<option value={note.id}>{note.title ?? 'Untitled'}</option>
						{/each}
					</select>
					<button
						class="rounded-lg bg-gradient-to-r from-sky-500 to-violet-500 px-4 py-2 text-xs font-semibold text-white hover:from-sky-400 hover:to-violet-400 disabled:opacity-50"
						on:click={() => generateFromNote(selectedNoteId)}
						disabled={generating || !selectedNoteId}
					>
						{generating ? 'Generating...' : 'Generate'}
					</button>
				</div>
			</div>

			<!-- Manual card -->
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<h3 class="text-sm font-semibold text-white">Add Manually</h3>
				<div class="mt-3 flex flex-col gap-2">
					<input
						class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white placeholder-slate-500"
						placeholder="Question (front)"
						bind:value={manualFront}
					/>
					<input
						class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white placeholder-slate-500"
						placeholder="Answer (back)"
						bind:value={manualBack}
					/>
					<button
						class="self-start rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
						disabled={!manualFront.trim() || !manualBack.trim()}
						on:click={() => {
							addManualCard(manualFront.trim(), manualBack.trim());
							manualFront = '';
							manualBack = '';
						}}
					>
						Add Card
					</button>
				</div>
			</div>
		</div>
	{/if}
</div>
