export const FILE_KIND = {
  FILE: "file",
  OTHER: "other",
  PARENT: "parent",
  SYMLINK: "symlink",
  DIRECTORY: "directory",
} as const;

export interface FileEntry {
  name: string;
  path: string;
  kind: string;
  hidden?: boolean;
  size?: number | null;
  mtime?: number | null;
  uid?: number | null;
  gid?: number | null;
  user?: string | null;
  group?: string | null;
  permissions?: number | null;
}
