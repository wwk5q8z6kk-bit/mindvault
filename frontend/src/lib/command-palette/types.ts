export type CommandAction = {
	id: string;
	title: string;
	subtitle?: string;
	keywords?: string[];
	icon?: string;
	group?: string;
	closeOnRun?: boolean;
	isAvailable?: (ctx: CommandContext) => boolean;
	handler: (ctx: CommandContext) => Promise<void> | void;
};

export type CommandContext = {
	query: string;
	selectedTaskId: string | null;
	selectedNoteId: string | null;
	openTaskModal: (mode: 'create' | 'edit', taskId?: string | null) => void;
	focusQuickAdd: () => void;
	setQuery: (value: string) => void;
	closePalette: () => void;
	navigate: (path: string) => Promise<void> | void;
	refreshData: () => Promise<void> | void;
	updateTaskStatus: (taskId: string, status: string) => Promise<void> | void;
	addTaskLabel: (taskId: string, label: string) => Promise<void> | void;
	selectTask: (taskId: string) => void;
	searchFts: (query: string) => Promise<SearchResult[]>;
	toast: (message: string, variant?: 'info' | 'success' | 'warning' | 'danger') => void;
	tasks: Array<{ id: string; title: string; status: string }>;
};

export type SearchResult = {
	object_type: string;
	object_id: string;
	content: string;
};

export type RankedAction = {
	action: CommandAction;
	score: number;
};
