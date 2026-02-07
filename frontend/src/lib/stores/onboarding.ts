import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

const STORAGE_KEY = 'mv_onboarding';

export type OnboardingStep =
	| 'welcome'
	| 'create-note'
	| 'create-task'
	| 'shortcuts'
	| 'complete';

export interface OnboardingState {
	completed: boolean;
	currentStep: OnboardingStep;
	noteCreated: boolean;
	taskCreated: boolean;
	skipped: boolean;
}

const defaultState: OnboardingState = {
	completed: false,
	currentStep: 'welcome',
	noteCreated: false,
	taskCreated: false,
	skipped: false
};

function loadState(): OnboardingState {
	if (!browser) return defaultState;
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (stored) {
			return { ...defaultState, ...JSON.parse(stored) };
		}
	} catch {
		// ignore
	}
	return defaultState;
}

function saveState(state: OnboardingState) {
	if (!browser) return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
	} catch {
		// ignore
	}
}

function createOnboardingStore() {
	const { subscribe, set, update } = writable<OnboardingState>(loadState());

	return {
		subscribe,

		nextStep() {
			update(state => {
				const steps: OnboardingStep[] = ['welcome', 'create-note', 'create-task', 'shortcuts', 'complete'];
				const currentIndex = steps.indexOf(state.currentStep);
				const nextIndex = Math.min(currentIndex + 1, steps.length - 1);
				const newState = { ...state, currentStep: steps[nextIndex] };

				if (newState.currentStep === 'complete') {
					newState.completed = true;
				}

				saveState(newState);
				return newState;
			});
		},

		prevStep() {
			update(state => {
				const steps: OnboardingStep[] = ['welcome', 'create-note', 'create-task', 'shortcuts', 'complete'];
				const currentIndex = steps.indexOf(state.currentStep);
				const prevIndex = Math.max(currentIndex - 1, 0);
				const newState = { ...state, currentStep: steps[prevIndex] };
				saveState(newState);
				return newState;
			});
		},

		markNoteCreated() {
			update(state => {
				const newState = { ...state, noteCreated: true };
				saveState(newState);
				return newState;
			});
		},

		markTaskCreated() {
			update(state => {
				const newState = { ...state, taskCreated: true };
				saveState(newState);
				return newState;
			});
		},

		skip() {
			update(state => {
				const newState = {
					...state,
					completed: true,
					skipped: true,
					currentStep: 'complete' as OnboardingStep
				};
				saveState(newState);
				return newState;
			});
		},

		reset() {
			const newState = { ...defaultState };
			saveState(newState);
			set(newState);
		}
	};
}

export const onboarding = createOnboardingStore();

export const shouldShowOnboarding = derived(
	onboarding,
	($onboarding) => !$onboarding.completed && !$onboarding.skipped
);
