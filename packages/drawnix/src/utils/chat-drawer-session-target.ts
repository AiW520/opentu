export type WorkflowSessionTarget =
  | { mode: 'append'; sessionId: string }
  | { mode: 'create' };

export function resolveWorkflowSessionTarget(
  activeSessionId: string | null,
  appendToCurrentSession?: boolean
): WorkflowSessionTarget {
  if (appendToCurrentSession && activeSessionId) {
    return { mode: 'append', sessionId: activeSessionId };
  }

  return { mode: 'create' };
}
