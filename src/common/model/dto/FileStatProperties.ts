import type { FileEntry } from "./FileEntry";

export interface DirStatProperties {
  totalSize: number;
  fileCount: number;
  dirCount: number;
}

export interface FileStatProperties {
  path: string;
  entry: FileEntry;
  dir?: DirStatProperties | null;
}
