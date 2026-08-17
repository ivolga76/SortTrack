export type GatewayErrorCode =
  | "STALE_STATE"
  | "INVALID_INPUT"
  | "NOT_FOUND"
  | "UNAVAILABLE"
  | "FILESYSTEM"
  | "DATABASE"
  | "SERIALIZATION"
  | "TASK_FAILED"
  | "UNKNOWN";

export class GatewayError extends Error {
  readonly code: GatewayErrorCode;
  readonly operation: string | null;
  readonly retryable: boolean;

  constructor(code: GatewayErrorCode, message: string, operation: string | null = null) {
    super(message);
    this.name = "GatewayError";
    this.code = code;
    this.operation = operation;
    this.retryable = code === "UNAVAILABLE" || code === "TASK_FAILED";
  }

  override toString(): string {
    return this.message;
  }
}

function messageFromUnknown(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (typeof error === "object" && error !== null && "message" in error && typeof error.message === "string") return error.message;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

const gatewayErrorCodes = new Set<GatewayErrorCode>(["STALE_STATE", "INVALID_INPUT", "NOT_FOUND", "UNAVAILABLE", "FILESYSTEM", "DATABASE", "SERIALIZATION", "TASK_FAILED", "UNKNOWN"]);

function codeFromUnknown(error: unknown, message: string): GatewayErrorCode {
  if (typeof error === "object" && error !== null && "code" in error && typeof error.code === "string" && gatewayErrorCodes.has(error.code as GatewayErrorCode)) {
    return error.code as GatewayErrorCode;
  }
  return classifyGatewayError(message);
}

export function classifyGatewayError(message: string): GatewayErrorCode {
  const normalized = message.toLocaleLowerCase("ru-RU");
  if (/изменил|изменился|изменилась|stale|changed after preview|expected state/.test(normalized)) return "STALE_STATE";
  if (/не найден|not found/.test(normalized)) return "NOT_FOUND";
  if (/unavailable|недоступ|desktop-приложени/.test(normalized)) return "UNAVAILABLE";
  if (/database error|sqlite|баз[аы] данных/.test(normalized)) return "DATABASE";
  if (/filesystem error|permission|access denied|файловой систем|доступ к файл/.test(normalized)) return "FILESYSTEM";
  if (/serialization error|json/.test(normalized)) return "SERIALIZATION";
  if (/background task|task failed|фонов.*останов/.test(normalized)) return "TASK_FAILED";
  if (/invalid|должен|сначала|нет изменений|rejected|отклон/.test(normalized)) return "INVALID_INPUT";
  return "UNKNOWN";
}

export function normalizeGatewayError(error: unknown, operation: string | null = null): GatewayError {
  if (error instanceof GatewayError) return error;
  const message = messageFromUnknown(error);
  return new GatewayError(codeFromUnknown(error, message), message, operation);
}

export function gatewayError(message: string, operation: string | null = null): GatewayError {
  return normalizeGatewayError(message, operation);
}
