import type { Context, ReactNode } from "react";
import { setLanguage } from "@/common/i18n/i18n.ts";
import { createServices } from "./createServices.ts";
import { ServiceContainer } from "./ServiceContainer.ts";
import type { ConfigService } from "@/service/ConfigService.ts";
import { createContext, useContext, useEffect, useState } from "react";
import { ConfigServiceImpl } from "@/service/impl/ConfigServiceImpl.ts";
import { BaseApiServiceImpl } from "@/service/impl/BaseApiServiceImpl.ts";

const stash = globalThis as {
  terminiServiceRuntime?: ServiceRuntime;
  terminiServiceContext?: Context<ServiceContextValue | null>;
};

async function startServices(container: ServiceContainer): Promise<void> {
  for (const service of container.getAllServices()) {
    try {
      await service.start();
    } catch (e) {
      console.error("[ServicesProvider] service start failed:", e);
    }
  }
}

async function initLanguage(container: ServiceContainer): Promise<void> {
  try {
    const config = await container.resolve<ConfigService>(ConfigServiceImpl).selectConfig();
    setLanguage(config?.system_language);
  } catch (e) {
    console.error("[ServicesProvider] language init failed:", e);
  }
}

async function stopServices(container: ServiceContainer): Promise<void> {
  for (const service of container.getAllServices()) {
    try {
      await service.quit();
    } catch (e) {
      console.error("[ServicesProvider] service quit failed:", e);
    }
  }
}

interface ServiceContextValue {
  container: ServiceContainer;
}

interface ServiceRuntime {
  container: ServiceContainer;
  startPromise: Promise<void>;
}

function resolveModuleContext(): Context<ServiceContextValue | null> {
  const context = stash.terminiServiceContext;
  if (context) {
    return context;
  }
  const nextContext = createContext<ServiceContextValue | null>(null);
  stash.terminiServiceContext = nextContext;
  return nextContext;
}

function resolveModuleRuntime(): ServiceRuntime {
  const runtime = stash.terminiServiceRuntime;
  if (runtime) {
    return runtime;
  }
  const container = createServices();
  const nextRuntime = {
    container,
    startPromise: BaseApiServiceImpl.restoreApiToken().then(() => startServices(container)).then(() => initLanguage(container)),
  };
  stash.terminiServiceRuntime = nextRuntime;
  return nextRuntime;
}

const moduleContext = resolveModuleContext();
const moduleRuntime = resolveModuleRuntime();
const moduleContainer = moduleRuntime.container;
const moduleStartPromise = moduleRuntime.startPromise;
const ServiceContext = moduleContext;

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    void stopServices(moduleRuntime.container);
    stash.terminiServiceRuntime = undefined;
  });
}

interface ServicesProviderProps {
  children: ReactNode;
}

export function ServicesProvider({ children }: ServicesProviderProps) {
  const [ready, setReady] = useState<boolean>(true);

  useEffect(() => {
    const cancelled = { current: false };
    void moduleStartPromise.then(() => {
      if (!cancelled.current) {
        setReady(true);
      }
    });
    return () => {
      cancelled.current = true;
    };
  }, []);

  return (
    <ServiceContext.Provider value={{ container: moduleContainer }}>
      {ready ? children : null}
    </ServiceContext.Provider>
  );
}

export function useServiceContainer(): ServiceContainer {
  const ctx = useContext(ServiceContext);
  if (!ctx) {
    throw new Error("useServiceContainer must be used within ServicesProvider");
  }
  return ctx.container;
}

export function getServiceContainer(): ServiceContainer {
  return moduleContainer;
}

