export type PreferenceFeedbackStatus = "idle" | "pending" | "saved" | "error";

export interface PreferenceFeedback {
  status: PreferenceFeedbackStatus;
  message: string | null;
}

export type PreferenceFeedbackMap = Record<string, PreferenceFeedback>;

export const IDLE_PREFERENCE_FEEDBACK: PreferenceFeedback = {
  status: "idle",
  message: null,
};

export function preferenceFeedbackFor(
  feedback: PreferenceFeedbackMap,
  operationKey: string,
): PreferenceFeedback {
  return feedback[operationKey] ?? IDLE_PREFERENCE_FEEDBACK;
}

export function updatePreferenceFeedback(
  feedback: PreferenceFeedbackMap,
  operationKey: string,
  status: PreferenceFeedbackStatus,
  message: string | null = null,
): PreferenceFeedbackMap {
  return {
    ...feedback,
    [operationKey]: { status, message },
  };
}

export function isPreferenceOperationPending(
  feedback: PreferenceFeedbackMap,
  operationKey: string,
): boolean {
  return preferenceFeedbackFor(feedback, operationKey).status === "pending";
}

export interface PreferenceOperationGate {
  finish(operationKey: string): void;
  isActive(operationKey: string): boolean;
  tryStart(operationKey: string): boolean;
}

export function createPreferenceOperationGate(): PreferenceOperationGate {
  const activeOperations = new Set<string>();
  return {
    tryStart(operationKey) {
      if (activeOperations.has(operationKey)) {
        return false;
      }
      activeOperations.add(operationKey);
      return true;
    },
    finish(operationKey) {
      activeOperations.delete(operationKey);
    },
    isActive(operationKey) {
      return activeOperations.has(operationKey);
    },
  };
}
