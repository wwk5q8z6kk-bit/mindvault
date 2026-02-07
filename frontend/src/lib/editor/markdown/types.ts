/**
 * Markdown parser types.
 * Defines tokens, AST nodes, and extension interfaces.
 */

// ============================================================================
// Token Types
// ============================================================================

export type TokenType =
	// Block tokens
	| 'heading'
	| 'paragraph'
	| 'code_block'
	| 'blockquote'
	| 'hr'
	| 'list'
	| 'list_item'
	| 'table'
	| 'table_row'
	| 'table_cell'
	| 'html_block'
	// Inline tokens
	| 'text'
	| 'bold'
	| 'italic'
	| 'code'
	| 'strikethrough'
	| 'link'
	| 'image'
	| 'wiki_link'
	| 'math_inline'
	| 'math_block'
	| 'line_break'
	| 'soft_break'
	// Special
	| 'document'
	| 'task_item';

export interface Token {
	type: TokenType;
	raw: string;
	text?: string;
	tokens?: Token[];
	// Block-specific
	depth?: number; // heading level
	lang?: string; // code block language
	ordered?: boolean; // list type
	start?: number; // ordered list start
	checked?: boolean; // task item
	// Link/image specific
	href?: string;
	title?: string;
	alt?: string;
	// Wiki-link specific
	target?: string;
	alias?: string;
	// Table specific
	header?: boolean;
	align?: ('left' | 'center' | 'right' | null)[];
	cells?: Token[][];
}

// ============================================================================
// AST Node Types
// ============================================================================

export type NodeType =
	| 'document'
	| 'heading'
	| 'paragraph'
	| 'code_block'
	| 'blockquote'
	| 'horizontal_rule'
	| 'list'
	| 'list_item'
	| 'task_item'
	| 'table'
	| 'table_row'
	| 'table_cell'
	| 'text'
	| 'bold'
	| 'italic'
	| 'code'
	| 'strikethrough'
	| 'link'
	| 'image'
	| 'wiki_link'
	| 'math_inline'
	| 'math_block'
	| 'line_break'
	| 'soft_break'
	| 'html';

export interface ASTNode {
	type: NodeType;
	children?: ASTNode[];
	// Text content
	value?: string;
	// Heading
	level?: number;
	// Code block
	lang?: string;
	// List
	ordered?: boolean;
	start?: number;
	// Task item
	checked?: boolean;
	// Link/Image
	url?: string;
	title?: string;
	alt?: string;
	// Wiki-link
	target?: string;
	alias?: string;
	// Table
	align?: ('left' | 'center' | 'right' | null)[];
	header?: boolean;
	// Position tracking for round-trip
	position?: {
		start: { line: number; column: number; offset: number };
		end: { line: number; column: number; offset: number };
	};
}

// ============================================================================
// Parser Options
// ============================================================================

export interface ParserOptions {
	/** Enable GitHub Flavored Markdown extensions */
	gfm?: boolean;
	/** Enable wiki-link syntax [[target|alias]] */
	wikiLinks?: boolean;
	/** Enable math syntax $...$ and $$...$$ */
	math?: boolean;
	/** Enable task lists */
	taskLists?: boolean;
	/** Enable tables */
	tables?: boolean;
	/** Custom block extensions */
	blockExtensions?: BlockExtension[];
	/** Custom inline extensions */
	inlineExtensions?: InlineExtension[];
}

export const defaultParserOptions: ParserOptions = {
	gfm: true,
	wikiLinks: true,
	math: true,
	taskLists: true,
	tables: true,
	blockExtensions: [],
	inlineExtensions: []
};

// ============================================================================
// Serializer Options
// ============================================================================

export interface SerializerOptions {
	/** Use soft breaks instead of hard breaks */
	softBreak?: string;
	/** Bullet character for unordered lists */
	bullet?: '-' | '*' | '+';
	/** Emphasis character */
	emphasis?: '_' | '*';
	/** Strong character */
	strong?: '__' | '**';
	/** Code block style */
	codeBlockStyle?: 'fenced' | 'indented';
	/** Fence character */
	fence?: '```' | '~~~';
	/** Horizontal rule style */
	horizontalRule?: '---' | '***' | '___';
	/** List item indent */
	listItemIndent?: 'one' | 'tab' | 'mixed';
}

export const defaultSerializerOptions: SerializerOptions = {
	softBreak: '\n',
	bullet: '-',
	emphasis: '*',
	strong: '**',
	codeBlockStyle: 'fenced',
	fence: '```',
	horizontalRule: '---',
	listItemIndent: 'one'
};

// ============================================================================
// Extension Interfaces
// ============================================================================

export interface BlockExtension {
	name: string;
	/** Match the start of a block */
	match: (line: string, nextLine?: string) => boolean;
	/** Parse the block, return tokens and lines consumed */
	parse: (lines: string[], startIndex: number) => { token: Token; linesConsumed: number } | null;
	/** Serialize the token back to markdown */
	serialize?: (node: ASTNode) => string;
}

export interface InlineExtension {
	name: string;
	/** Priority (higher = earlier) */
	priority?: number;
	/** Regex pattern to match */
	pattern: RegExp;
	/** Parse the match into a token */
	parse: (match: RegExpMatchArray) => Token | null;
	/** Serialize the token back to markdown */
	serialize?: (node: ASTNode) => string;
}

// ============================================================================
// HTML Conversion Types
// ============================================================================

export interface HTMLConverterOptions {
	/** Sanitize HTML output */
	sanitize?: boolean;
	/** Custom element renderers */
	renderers?: Partial<Record<NodeType, (node: ASTNode) => string>>;
	/** Class prefix for elements */
	classPrefix?: string;
}

export const defaultHTMLConverterOptions: HTMLConverterOptions = {
	sanitize: true,
	renderers: {},
	classPrefix: 'mv-'
};
