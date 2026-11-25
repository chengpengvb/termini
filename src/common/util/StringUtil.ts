export function isNil(v: unknown): v is null | undefined {
  return v == null;
}

export function isNotNil<T>(v: T | null | undefined): v is T {
  return v != null;
}

export function isEmpty(s: string | null | undefined): s is undefined | null {
  return isNil(s) || s.trim() === '';
}

export function isNotEmpty(s: string | null | undefined): s is string {
  return isNotNil(s) && s.trim() !== '';
}

