import {
  useQuery,
  type UseQueryResult,
  keepPreviousData,
} from "@tanstack/react-query";
import {
  providersApi,
  settingsApi,
  usageApi,
  sessionsApi,
  type AppId,
  type ManagementTarget,
} from "@/lib/api";
import type {
  Provider,
  Settings,
  UsageResult,
  SessionMeta,
  SessionMessage,
} from "@/types";
import { usageKeys } from "@/lib/query/usage";
import { isRemotePasswordRequiredError } from "@/utils/errorUtils";

const targetKey = (target: ManagementTarget) =>
  target.type === "remote" ? `remote:${target.profile.id}` : "local";

const sortProviders = (
  providers: Record<string, Provider>,
): Record<string, Provider> => {
  const sortedEntries = Object.values(providers)
    .sort((a, b) => {
      const indexA = a.sortIndex ?? Number.MAX_SAFE_INTEGER;
      const indexB = b.sortIndex ?? Number.MAX_SAFE_INTEGER;
      if (indexA !== indexB) {
        return indexA - indexB;
      }

      const timeA = a.createdAt ?? 0;
      const timeB = b.createdAt ?? 0;
      if (timeA === timeB) {
        return a.name.localeCompare(b.name, "zh-CN");
      }
      return timeA - timeB;
    })
    .map((provider) => [provider.id, provider] as const);

  return Object.fromEntries(sortedEntries);
};

export interface ProvidersQueryData {
  providers: Record<string, Provider>;
  currentProviderId: string;
}

interface ProviderQueryReaders {
  getState?: typeof providersApi.getState;
  getAll: typeof providersApi.getAll;
  getCurrent: typeof providersApi.getCurrent;
}

export async function loadProvidersQueryData(
  appId: AppId,
  target: ManagementTarget,
  readers: ProviderQueryReaders = providersApi,
): Promise<ProvidersQueryData> {
  let providers: Record<string, Provider> = {};
  let currentProviderId = "";

  if (target.type === "remote" && readers.getState) {
    const state = await readers.getState(appId, target);
    return {
      providers: sortProviders(state.providers),
      currentProviderId: state.currentProviderId,
    };
  }

  try {
    providers = await readers.getAll(appId, target);
  } catch (error) {
    if (target.type === "remote") {
      throw error;
    }
    console.error("获取供应商列表失败:", error);
  }

  try {
    currentProviderId = await readers.getCurrent(appId, target);
  } catch (error) {
    if (target.type === "remote") {
      throw error;
    }
    console.error("获取当前供应商失败:", error);
  }

  return {
    providers: sortProviders(providers),
    currentProviderId,
  };
}

export interface UseProvidersQueryOptions {
  isProxyRunning?: boolean; // 代理服务是否运行中
  target?: ManagementTarget;
}

export const useProvidersQuery = (
  appId: AppId,
  options?: UseProvidersQueryOptions,
): UseQueryResult<ProvidersQueryData> => {
  const { isProxyRunning = false, target = { type: "local" } } = options || {};
  const key = targetKey(target);

  return useQuery({
    queryKey: ["providers", appId, key],
    placeholderData: keepPreviousData,
    staleTime: target.type === "remote" ? 30_000 : 0,
    refetchOnWindowFocus: target.type === "local",
    // 当代理服务运行时，每 10 秒刷新一次供应商列表
    // 这样可以自动反映后端熔断器自动禁用代理目标的变更
    refetchInterval: target.type === "local" && isProxyRunning ? 10000 : false,
    retry: (failureCount, error) => {
      if (target.type === "remote" && isRemotePasswordRequiredError(error)) {
        return false;
      }
      return failureCount < 1;
    },
    queryFn: async () => {
      return loadProvidersQueryData(appId, target);
    },
  });
};

export const useSettingsQuery = (): UseQueryResult<Settings> => {
  return useQuery({
    queryKey: ["settings"],
    queryFn: async () => settingsApi.get(),
  });
};

export interface UseUsageQueryOptions {
  enabled?: boolean;
  autoQueryInterval?: number; // 自动查询间隔（分钟），0 表示禁用
}

export const useUsageQuery = (
  providerId: string,
  appId: AppId,
  options?: UseUsageQueryOptions,
) => {
  const { enabled = true, autoQueryInterval = 0 } = options || {};

  // 计算 staleTime：如果有自动刷新间隔，使用该间隔；否则默认 5 分钟
  // 这样可以避免切换 app 页面时重复触发查询
  const staleTime =
    autoQueryInterval > 0
      ? autoQueryInterval * 60 * 1000 // 与刷新间隔保持一致
      : 5 * 60 * 1000; // 默认 5 分钟

  const query = useQuery<UsageResult>({
    queryKey: usageKeys.script(providerId, appId),
    queryFn: async () => usageApi.query(providerId, appId),
    enabled: enabled && !!providerId,
    refetchInterval:
      autoQueryInterval > 0
        ? Math.max(autoQueryInterval, 1) * 60 * 1000 // 最小1分钟
        : false,
    refetchIntervalInBackground: true, // 后台也继续定时查询
    refetchOnWindowFocus: false,
    retry: false,
    staleTime, // 使用动态计算的缓存时间
    gcTime: 10 * 60 * 1000, // 缓存保留 10 分钟（组件卸载后）
  });

  return {
    ...query,
    lastQueriedAt: query.dataUpdatedAt || null,
  };
};

export const useSessionsQuery = (
  target: ManagementTarget = { type: "local" },
) => {
  return useQuery<SessionMeta[]>({
    queryKey: ["sessions", targetKey(target)],
    queryFn: async () => sessionsApi.list(target),
    staleTime: 30 * 1000,
  });
};

export const useSessionMessagesQuery = (
  providerId?: string,
  sourcePath?: string,
  target: ManagementTarget = { type: "local" },
) => {
  return useQuery<SessionMessage[]>({
    queryKey: ["sessionMessages", targetKey(target), providerId, sourcePath],
    queryFn: async () =>
      sessionsApi.getMessages(providerId!, sourcePath!, target),
    enabled: Boolean(providerId && sourcePath),
    staleTime: 30 * 1000,
  });
};
