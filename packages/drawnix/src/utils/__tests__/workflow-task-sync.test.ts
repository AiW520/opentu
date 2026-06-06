import { describe, expect, it } from 'vitest';
import type { WorkflowMessageData } from '../../types/chat.types';
import { TaskStatus, TaskType, type Task } from '../../types/task.types';
import { applyTaskUpdateToWorkflowMessage } from '../workflow-task-sync';

function createTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task-2',
    type: TaskType.IMAGE,
    status: TaskStatus.PROCESSING,
    params: {
      prompt: '生成皮特格里芬。',
      workflowId: 'wf-1',
      batchId: 'wf_batch_wf-1',
      batchIndex: 2,
    },
    createdAt: 1,
    updatedAt: 2,
    ...overrides,
  };
}

function createImageWorkflow(
  overrides: Partial<WorkflowMessageData> = {}
): WorkflowMessageData {
  return {
    id: 'wf-1',
    name: '图片生成',
    generationType: 'image',
    prompt: '生成皮特格里芬。',
    count: 3,
    status: 'failed',
    steps: [1, 2, 3].map((index) => ({
      id: `step-${index}`,
      description: `生成图片 (${index}/3)`,
      status: 'failed' as const,
      mcp: 'generate_image',
      args: {
        prompt: '生成皮特格里芬。',
      },
      result: {
        taskId: `task-${index}`,
      },
      error: '任务执行失败',
      options: {
        batchId: 'wf_batch_wf-1',
        batchIndex: index,
        batchTotal: 3,
      },
    })),
    ...overrides,
  };
}

describe('workflow-task-sync', () => {
  it('moves a failed retried image step back to running and clears stale error', () => {
    const workflow = createImageWorkflow();
    const updated = applyTaskUpdateToWorkflowMessage(
      workflow,
      createTask({ status: TaskStatus.PROCESSING })
    );

    expect(updated?.steps[1].status).toBe('running');
    expect(updated?.steps[1].error).toBeUndefined();
    expect(updated?.steps[1].result).toMatchObject({ taskId: 'task-2' });
  });

  it('matches a retry completion by workflow batch slot when the failed step has no taskId', () => {
    const workflow = createImageWorkflow({
      steps: createImageWorkflow().steps.map((step) =>
        step.id === 'step-2' ? { ...step, result: undefined } : step
      ),
    });

    const updated = applyTaskUpdateToWorkflowMessage(
      workflow,
      createTask({
        status: TaskStatus.COMPLETED,
        result: {
          url: '/__aitu_cache__/image/task-2.png',
          format: 'png',
          size: 123,
        },
      })
    );

    expect(updated?.steps[1].status).toBe('completed');
    expect(updated?.steps[1].error).toBeUndefined();
    expect(updated?.steps[1].result).toMatchObject({
      taskId: 'task-2',
      result: {
        url: '/__aitu_cache__/image/task-2.png',
      },
    });
  });

  it('marks an image workflow completed when queue retry completes the last failed step', () => {
    const workflow = createImageWorkflow({
      steps: createImageWorkflow().steps.map((step) =>
        step.id === 'step-2'
          ? step
          : {
              ...step,
              status: 'completed',
              error: undefined,
              result: { taskId: `task-${step.options?.batchIndex}` },
            }
      ),
    });

    const updated = applyTaskUpdateToWorkflowMessage(
      workflow,
      createTask({
        status: TaskStatus.COMPLETED,
        result: {
          url: '/__aitu_cache__/image/task-2.png',
          format: 'png',
          size: 123,
        },
      })
    );

    expect(updated?.status).toBe('completed');
    expect(updated?.postProcessingStatus).toBe('completed');
    expect(updated?.steps.every((step) => step.status === 'completed')).toBe(
      true
    );
  });

  it('does not downgrade an already completed image step on a stale processing event', () => {
    const workflow = createImageWorkflow({
      status: 'completed',
      postProcessingStatus: 'completed',
      steps: createImageWorkflow().steps.map((step) => ({
        ...step,
        status: 'completed',
        error: undefined,
      })),
    });

    const updated = applyTaskUpdateToWorkflowMessage(
      workflow,
      createTask({ status: TaskStatus.PROCESSING })
    );

    expect(updated?.steps[1].status).toBe('completed');
    expect(updated?.postProcessingStatus).toBe('completed');
  });
});
