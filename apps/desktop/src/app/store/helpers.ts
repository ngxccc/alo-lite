import type { Thread } from "@/shared/types/ipc";

let toastSeq = 1;

export function nextToast(message: string): { id: number; message: string } {
  toastSeq += 1;
  return { id: toastSeq, message };
}

export function updateThreadMessage(
  threads: Thread[],
  threadId: string,
  updater: (t: Thread) => Thread,
): Thread[] {
  return threads.map((t) => (t.id === threadId ? updater(t) : t));
}
