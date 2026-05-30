import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { promptsApi, type Prompt, type AppId } from "@/lib/api";
import type { ManagementTarget } from "@/lib/api/remote";

const LOCAL_TARGET: ManagementTarget = { type: "local" };

export function usePromptActions(
  appId: AppId,
  target: ManagementTarget = LOCAL_TARGET,
) {
  const { t } = useTranslation();
  const [prompts, setPrompts] = useState<Record<string, Prompt>>({});
  const [loading, setLoading] = useState(false);
  const [currentFileContent, setCurrentFileContent] = useState<string | null>(
    null,
  );

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const data = await promptsApi.getPrompts(appId, target);
      setPrompts(data);

      // 同时加载当前文件内容
      try {
        const content = await promptsApi.getCurrentFileContent(appId, target);
        setCurrentFileContent(content);
      } catch (error) {
        setCurrentFileContent(null);
      }
    } catch (error) {
      toast.error(t("prompts.loadFailed"));
    } finally {
      setLoading(false);
    }
  }, [appId, target, t]);

  const savePrompt = useCallback(
    async (id: string, prompt: Prompt) => {
      try {
        await promptsApi.upsertPrompt(appId, id, prompt, target);
        await reload();
        toast.success(t("prompts.saveSuccess"), { closeButton: true });
      } catch (error) {
        toast.error(t("prompts.saveFailed"));
        throw error;
      }
    },
    [appId, reload, target, t],
  );

  const deletePrompt = useCallback(
    async (id: string) => {
      try {
        await promptsApi.deletePrompt(appId, id, target);
        await reload();
        toast.success(t("prompts.deleteSuccess"), { closeButton: true });
      } catch (error) {
        toast.error(t("prompts.deleteFailed"));
        throw error;
      }
    },
    [appId, reload, target, t],
  );

  const enablePrompt = useCallback(
    async (id: string) => {
      try {
        await promptsApi.enablePrompt(appId, id, target);
        await reload();
        toast.success(t("prompts.enableSuccess"), { closeButton: true });
      } catch (error) {
        toast.error(t("prompts.enableFailed"));
        throw error;
      }
    },
    [appId, reload, target, t],
  );

  const toggleEnabled = useCallback(
    async (id: string, enabled: boolean) => {
      // Optimistic update
      const previousPrompts = prompts;

      // 如果要启用当前提示词，先禁用其他所有提示词
      if (enabled) {
        const updatedPrompts = Object.keys(prompts).reduce(
          (acc, key) => {
            acc[key] = {
              ...prompts[key],
              enabled: key === id,
            };
            return acc;
          },
          {} as Record<string, Prompt>,
        );
        setPrompts(updatedPrompts);
      } else {
        setPrompts((prev) => ({
          ...prev,
          [id]: {
            ...prev[id],
            enabled: false,
          },
        }));
      }

      try {
        if (enabled) {
          await promptsApi.enablePrompt(appId, id, target);
          toast.success(t("prompts.enableSuccess"), { closeButton: true });
        } else {
          // 禁用提示词 - 需要后端支持
          await promptsApi.upsertPrompt(
            appId,
            id,
            {
              ...prompts[id],
              enabled: false,
            },
            target,
          );
          toast.success(t("prompts.disableSuccess"), { closeButton: true });
        }
        await reload();
      } catch (error) {
        // Rollback on failure
        setPrompts(previousPrompts);
        toast.error(
          enabled ? t("prompts.enableFailed") : t("prompts.disableFailed"),
        );
        throw error;
      }
    },
    [appId, prompts, reload, target, t],
  );

  const importFromFile = useCallback(async () => {
    try {
      const id = await promptsApi.importFromFile(appId, target);
      await reload();
      toast.success(t("prompts.importSuccess"), { closeButton: true });
      return id;
    } catch (error) {
      toast.error(t("prompts.importFailed"));
      throw error;
    }
  }, [appId, reload, target, t]);

  return {
    prompts,
    loading,
    currentFileContent,
    reload,
    savePrompt,
    deletePrompt,
    enablePrompt,
    toggleEnabled,
    importFromFile,
  };
}
