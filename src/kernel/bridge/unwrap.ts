import type { AppError } from "../../bindings";

export type { AppError };

type CommandResult<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };

/** An `AppError` from Rust, thrown so TanStack Query treats it as a failed query/mutation. */
export class BridgeError extends Error {
  readonly code: string;

  constructor(error: AppError) {
    super(error.message);
    this.code = error.code;
  }
}

export async function unwrap<T>(call: Promise<CommandResult<T>>): Promise<T> {
  const result = await call;
  if (result.status === "error") throw new BridgeError(result.error);
  return result.data;
}
