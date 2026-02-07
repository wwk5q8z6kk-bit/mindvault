/**
 * History module exports.
 */

export {
	HistoryStack,
	createHistoryStack
} from './stack';

export type {
	HistoryStackOptions
} from './stack';

export {
	createTransaction,
	composeTransactions,
	insertTextTransaction,
	deleteTextTransaction,
	replaceSelectionTransaction,
	insertNodeTransaction,
	deleteNodeTransaction,
	setAttrTransaction,
	toggleMarkTransaction,
	splitNodeTransaction,
	mergeNodesTransaction
} from './transaction';

export type {
	TransactionBuilder,
	AppliedTransaction
} from './transaction';
