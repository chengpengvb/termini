import type { BaseService } from './BaseService';

export interface ApiService<T> extends BaseService {
  selectAll(): Promise<T[]>;

  insert(item: T): Promise<T>;

  update(item: T): Promise<T>;

  deleteById(id: string): Promise<void>;

  selectById(id: string): Promise<T | null>;
}

