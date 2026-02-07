import { writable } from 'svelte/store';
import type { Note } from '$lib/api/notes';
import { updateNote } from '$lib/api/notes';
import { ensureDailyNote, listDailyNotes } from '$lib/api/daily-notes';

export const currentDailyNote = writable<Note | null>(null);
export const recentDailyNotes = writable<Note[]>([]);

export async function loadDailyNote(date?: string): Promise<void> {
	const { note } = await ensureDailyNote(date);
	currentDailyNote.set(note);
}

export async function loadRecentDailyNotes(): Promise<void> {
	const notes = await listDailyNotes(14);
	recentDailyNotes.set(notes);
}

export async function saveDailyNote(noteId: string, content: string): Promise<void> {
	await updateNote(noteId, { markdown: content });
}
