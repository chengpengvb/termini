import { Container } from "inversify";
import { BaseService } from "@/service/BaseService.ts";

interface ServiceClass<T extends BaseService> {
  new (...args: any[]): T;
  readonly name: string;
}

export class ServiceContainer {
  private readonly inner: Container;
  private readonly registered: string[] = [];

  constructor() {
    this.inner = new Container();
  }

  bind<T extends BaseService>(ImplClass: ServiceClass<T>): void {
    this.inner.bind<T>(ImplClass).toSelf().inSingletonScope();
    this.inner.bind<T>(ImplClass.name).toService(ImplClass);
    this.registered.push(ImplClass.name);
  }

  resolve<T extends BaseService>(ImplClass: ServiceClass<T>): T {
    const instance = this.inner.get<T>(ImplClass.name);
    if (!instance) {
      throw new Error(`Service [${ImplClass.name}] is not registered`);
    }
    return instance;
  }

  getAllServices(): BaseService[] {
    const list: BaseService[] = [];
    for (const serviceName of this.registered) {
      const instance = this.inner.get<BaseService>(serviceName);
      if (!instance) {
        continue;
      }
      list.push(instance);
    }
    list.sort((a, b) => a.getPhase() - b.getPhase());
    return list;
  }
}

