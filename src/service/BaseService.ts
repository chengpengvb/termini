import type { BaseApp } from '../common/core/BaseApp';

export interface BaseService extends BaseApp {
  getId(): string;
  onLogout(): void;
}

