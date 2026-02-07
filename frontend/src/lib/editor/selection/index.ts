/**
 * Selection module exports.
 */

export {
	SelectionManager,
	createSelectionManager,
	domToModelPath,
	domRangeToPosition,
	domSelectionToModel,
	modelPathToDomElement,
	modelPositionToDOM,
	applySelectionToDOM
} from './manager';

export type {
	CompositionState,
	SelectionManagerOptions
} from './manager';
