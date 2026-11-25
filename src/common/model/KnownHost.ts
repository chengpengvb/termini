export interface KnownHost {
  id: string;
  port: number;
  host: string;
  sha256: string;
  trusted_at: number;
  algorithm: string;
}
