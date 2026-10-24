import { toast } from "sonner";

/** Show the server's message when it has one, otherwise a plain fallback. */
export function reportError(error: unknown, fallback: string): void {
  const message = error instanceof Error && error.message ? error.message : fallback;
  toast.error(message);
}
