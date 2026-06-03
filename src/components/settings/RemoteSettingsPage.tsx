import { useCallback, useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { Database, Download, Loader2, RefreshCw, Server } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ImportExportSection } from "@/components/settings/ImportExportSection";
import {
  TOOL_DISPLAY_NAMES,
  TOOL_NAMES,
  ToolEnvironmentSection,
  type ToolLifecycleAction,
  type ToolName,
} from "@/components/settings/ToolEnvironmentSection";
import { useImportExport } from "@/hooks/useImportExport";
import { remoteApi } from "@/lib/api";
import type {
  ManagementTarget,
  RemoteHealth,
  RemoteToolVersion,
} from "@/lib/api";
import { isUpdateAvailable } from "@/lib/version";
import { extractErrorMessage } from "@/utils/errorUtils";

interface RemoteSettingsPageProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImportSuccess?: () => void | Promise<void>;
  defaultTab?: string;
  target: Extract<ManagementTarget, { type: "remote" }>;
}

function coerceRemoteTab(tab: string | undefined): string {
  if (tab === "advanced") return "data";
  if (tab === "about") return "environment";
  return tab === "data" || tab === "environment" ? tab : "environment";
}

export function RemoteSettingsPage({
  open,
  onImportSuccess,
  defaultTab = "environment",
  target,
}: RemoteSettingsPageProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState(coerceRemoteTab(defaultTab));

  const [health, setHealth] = useState<RemoteHealth | null>(null);
  const [isCheckingHealth, setIsCheckingHealth] = useState(false);
  const [isInstallingHelper, setIsInstallingHelper] = useState(false);
  const [toolVersions, setToolVersions] = useState<RemoteToolVersion[]>([]);
  const [isLoadingTools, setIsLoadingTools] = useState(false);
  const [toolActions, setToolActions] = useState<
    Partial<Record<ToolName, ToolLifecycleAction>>
  >({});
  const [batchAction, setBatchAction] = useState<ToolLifecycleAction | null>(
    null,
  );
  const [activeRemoteTask, setActiveRemoteTask] = useState<string | null>(null);

  const importExport = useImportExport({ onImportSuccess, target });

  const toolsCapability = health?.capabilities.includes("tools") ?? false;
  const helperReady = Boolean(health?.reachable && health.helperInstalled);
  const toolsDisabled = !helperReady || !toolsCapability;
  const toolsDisabledMessage = !helperReady
    ? t("remote.settings.environment.helperRequired", {
        defaultValue: "请先完成健康检查并安装可用的远程 Helper。",
      })
    : t("remote.settings.environment.helperUnsupported", {
        defaultValue:
          "当前远程 Helper 不支持环境检查更新。请安装包含 tools capability 的新版 Helper。",
      });

  const toolVersionByName = useMemo(
    () => new Map(toolVersions.map((tool) => [tool.name, tool])),
    [toolVersions],
  );

  const updatableTools = useMemo(
    () =>
      TOOL_NAMES.filter((name) => {
        const tool = toolVersionByName.get(name);
        return Boolean(
          tool?.version &&
            tool.latest_version &&
            isUpdateAvailable(tool.version, tool.latest_version),
        );
      }),
    [toolVersionByName],
  );

  const loadHealth = useCallback(async () => {
    setIsCheckingHealth(true);
    setActiveRemoteTask(
      t("remote.settings.tasks.checkHealth", {
        defaultValue: "正在通过 SSH 检查远程 Helper 状态...",
      }),
    );
    try {
      const result = await remoteApi.checkHealth(target.profile, target.secret);
      setHealth(result);
      return result;
    } catch (error) {
      console.error("[RemoteSettingsPage] Failed to check health", error);
      setHealth({
        reachable: false,
        helperInstalled: false,
        capabilities: [],
        lastError: extractErrorMessage(error),
      });
      return null;
    } finally {
      setIsCheckingHealth(false);
      setActiveRemoteTask(null);
    }
  }, [t, target.profile, target.secret]);

  const loadToolVersions = useCallback(async () => {
    setIsLoadingTools(true);
    setActiveRemoteTask(
      t("remote.settings.tasks.loadTools", {
        defaultValue: "正在读取远程工具版本和更新信息...",
      }),
    );
    try {
      const result = await remoteApi.getToolVersions(
        target.profile,
        [...TOOL_NAMES],
        target.secret,
      );
      setToolVersions(result);
      return result;
    } catch (error) {
      console.error("[RemoteSettingsPage] Failed to load tools", error);
      toast.error(
        t("remote.settings.environment.loadToolsFailed", {
          defaultValue: "远程环境检查失败",
        }),
        { description: extractErrorMessage(error) },
      );
      return [];
    } finally {
      setIsLoadingTools(false);
      setActiveRemoteTask(null);
    }
  }, [t, target.profile, target.secret]);

  useEffect(() => {
    if (!open) return;
    setActiveTab(coerceRemoteTab(defaultTab));
    void loadHealth();
  }, [defaultTab, loadHealth, open]);

  const installHelper = async () => {
    setIsInstallingHelper(true);
    setActiveRemoteTask(
      t("remote.settings.tasks.installHelper", {
        defaultValue: "正在安装远程 Helper：连接 SSH、下载二进制并验证能力...",
      }),
    );
    try {
      const result = await remoteApi.installHelper(
        target.profile,
        target.secret,
      );
      setHealth(result);
      toast.success(
        t("remote.health.installSuccess", {
          defaultValue: "远程 Helper 已安装",
        }),
      );
    } catch (error) {
      console.error("[RemoteSettingsPage] Failed to install helper", error);
      toast.error(
        t("remote.health.installFailed", {
          defaultValue: "安装远程 Helper 失败: {{error}}",
          error: extractErrorMessage(error),
        }),
      );
    } finally {
      setIsInstallingHelper(false);
      setActiveRemoteTask(null);
    }
  };

  const runToolAction = async (
    toolNames: ToolName[],
    action: ToolLifecycleAction,
  ) => {
    if (toolNames.length === 0) return;
    const isBatch = toolNames.length > 1;
    if (isBatch) setBatchAction(action);

    const failures: string[] = [];
    let succeeded = 0;
    for (const toolName of toolNames) {
      setActiveRemoteTask(
        t("remote.settings.tasks.runToolAction", {
          defaultValue: "正在远程{{action}}{{tool}}...",
          action:
            action === "install"
              ? t("settings.toolInstall")
              : t("settings.toolUpdate"),
          tool: TOOL_DISPLAY_NAMES[toolName],
        }),
      );
      setToolActions((prev) => ({ ...prev, [toolName]: action }));
      try {
        await remoteApi.runToolLifecycleAction(
          target.profile,
          [toolName],
          action,
          target.secret,
        );
        succeeded += 1;
        await loadToolVersions();
      } catch (error) {
        console.error(
          `[RemoteSettingsPage] Failed to ${action} ${toolName}`,
          error,
        );
        failures.push(
          `${TOOL_DISPLAY_NAMES[toolName]}: ${extractErrorMessage(error)}`,
        );
      } finally {
        setToolActions((prev) => {
          const next = { ...prev };
          delete next[toolName];
          return next;
        });
      }
    }

    if (isBatch) setBatchAction(null);
    setActiveRemoteTask(null);
    const actionLabel =
      action === "install"
        ? t("settings.toolInstall")
        : t("settings.toolUpdate");
    if (failures.length === 0) {
      toast.success(
        t("settings.toolActionDone", {
          count: succeeded,
          action: actionLabel,
        }),
      );
    } else if (succeeded === 0) {
      toast.error(t("settings.toolActionFailed"), {
        description: failures.join("\n"),
      });
    } else {
      toast.warning(
        t("settings.toolActionPartial", {
          succeeded,
          failed: failures.length,
        }),
        { description: failures.join("\n") },
      );
    }
  };

  return (
    <div className="flex flex-col h-full overflow-hidden px-6">
      <span data-testid="settings-target" className="sr-only">
        remote
      </span>
      <Tabs
        value={activeTab}
        onValueChange={setActiveTab}
        className="flex flex-col h-full"
      >
        <TabsList className="grid w-full grid-cols-2 mb-6 glass rounded-lg">
          <TabsTrigger value="environment">
            {t("remote.settings.tabs.environment", {
              defaultValue: "远程环境",
            })}
          </TabsTrigger>
          <TabsTrigger value="data">
            {t("remote.settings.tabs.data", { defaultValue: "数据" })}
          </TabsTrigger>
        </TabsList>

        <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden pr-2">
          <TabsContent value="environment" className="space-y-4 mt-0 pb-4">
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.3 }}
              className="space-y-4"
            >
              <RemoteHealthSection
                health={health}
                isChecking={isCheckingHealth}
                isInstalling={isInstallingHelper}
                onRefresh={() => {
                  void loadHealth();
                }}
                onInstallHelper={installHelper}
                profileName={target.profile.name || target.profile.host}
              />
              <RemoteTaskBanner message={activeRemoteTask} />
              <ToolEnvironmentSection
                title={t("remote.settings.environment.toolsTitle", {
                  defaultValue: "远程环境检查更新",
                })}
                description={t("remote.settings.environment.toolsDescription", {
                  defaultValue:
                    "检查远程服务器上的 Claude Code、Codex、Gemini、OpenCode、OpenClaw 和 Hermes",
                })}
                disabled={toolsDisabled}
                disabledMessage={toolsDisabledMessage}
                toolVersions={toolVersions}
                isLoading={isLoadingTools}
                toolActions={toolActions}
                batchAction={batchAction}
                updatableToolNames={updatableTools}
                isAnyBusy={
                  Boolean(batchAction) || Object.keys(toolActions).length > 0
                }
                onRefresh={() => {
                  void loadToolVersions();
                }}
                onRunToolAction={runToolAction}
              />
            </motion.div>
          </TabsContent>

          <TabsContent value="data" className="space-y-4 mt-0 pb-4">
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.3 }}
              className="space-y-4"
            >
              <Accordion
                type="multiple"
                defaultValue={["data"]}
                className="space-y-4"
              >
                <AccordionItem
                  value="data"
                  className="rounded-xl glass-card overflow-hidden"
                >
                  <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                    <div className="flex items-center gap-3">
                      <Database className="h-5 w-5 text-blue-500" />
                      <div className="text-left">
                        <h3 className="text-base font-semibold">
                          {t("settings.advanced.data.title")}
                        </h3>
                        <p className="text-sm text-muted-foreground font-normal">
                          {t("remote.settings.data.description", {
                            defaultValue:
                              "导入或导出当前远程服务器自己的 CC Switch 数据",
                          })}
                        </p>
                      </div>
                    </div>
                  </AccordionTrigger>
                  <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                    <ImportExportSection
                      status={importExport.status}
                      selectedFile={importExport.selectedFile}
                      errorMessage={importExport.errorMessage}
                      backupId={importExport.backupId}
                      isImporting={importExport.isImporting}
                      onSelectFile={importExport.selectImportFile}
                      onImport={importExport.importConfig}
                      onExport={importExport.exportConfig}
                      onClear={importExport.clearSelection}
                    />
                  </AccordionContent>
                </AccordionItem>
              </Accordion>
            </motion.div>
          </TabsContent>
        </div>
      </Tabs>
    </div>
  );
}

interface RemoteHealthSectionProps {
  health: RemoteHealth | null;
  isChecking: boolean;
  isInstalling: boolean;
  profileName: string;
  onRefresh: () => void | Promise<void>;
  onInstallHelper: () => void | Promise<void>;
}

function RemoteHealthSection({
  health,
  isChecking,
  isInstalling,
  profileName,
  onRefresh,
  onInstallHelper,
}: RemoteHealthSectionProps) {
  const { t } = useTranslation();
  const helperReady = Boolean(health?.reachable && health.helperInstalled);
  const capabilitySummary = formatRemoteCapabilitySummary(health, t);
  return (
    <div className="space-y-3">
      <div className="flex flex-col gap-3 px-1 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex min-w-0 items-center gap-3">
          <Server className="h-5 w-5 text-primary shrink-0" />
          <div className="min-w-0">
            <h3 className="text-base font-semibold truncate">
              {t("remote.settings.environment.host", {
                defaultValue: "远程主机",
              })}
            </h3>
            <p className="text-sm text-muted-foreground truncate">
              {profileName}
            </p>
          </div>
          <Badge variant={helperReady ? "default" : "destructive"}>
            {helperReady
              ? t("remote.status.connected", { defaultValue: "已连接" })
              : t("remote.status.unavailable", { defaultValue: "不可用" })}
          </Badge>
        </div>
        <div className="flex shrink-0 gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => void onRefresh()}
            disabled={isChecking || isInstalling}
          >
            {isChecking ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="mr-2 h-4 w-4" />
            )}
            {t("common.refresh")}
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => void onInstallHelper()}
            disabled={isInstalling || isChecking}
          >
            {isInstalling ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Download className="mr-2 h-4 w-4" />
            )}
            {t("remote.health.installHelper", {
              defaultValue: "安装 Helper",
            })}
          </Button>
        </div>
      </div>
      <div className="overflow-hidden rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 shadow-sm">
        <div className="grid gap-px bg-border/50 sm:grid-cols-3">
          <InfoCell
            label={t("remote.health.helperVersion", {
              defaultValue: "Helper 版本",
            })}
            value={health?.helperVersion ?? "-"}
          />
          <InfoCell
            label={t("remote.health.platform", { defaultValue: "系统" })}
            value={health?.platform ?? "-"}
          />
          <InfoCell
            label={t("remote.health.capabilities", {
              defaultValue: "远程功能支持",
            })}
            value={capabilitySummary}
          />
        </div>
        {health?.lastError ? (
          <div className="border-t border-border/50 px-4 py-3 text-sm text-destructive">
            {health.lastError}
          </div>
        ) : null}
      </div>
    </div>
  );
}

function formatRemoteCapabilitySummary(
  health: RemoteHealth | null,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (!health?.reachable || !health.helperInstalled) {
    return "-";
  }
  const count = health.capabilities.length;
  if (count === 0) {
    return t("remote.health.noCapabilities", {
      defaultValue: "未返回功能支持信息",
    });
  }
  return t("remote.health.supportedCapabilities", {
    defaultValue: "已支持 {{count}} 项",
    count,
  });
}

function InfoCell({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-background/70 px-4 py-3 min-w-0">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 truncate text-sm font-medium">{value}</div>
    </div>
  );
}

function RemoteTaskBanner({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <div className="flex items-center gap-2 rounded-xl border border-primary/20 bg-primary/10 px-4 py-3 text-sm text-primary">
      <Loader2 className="h-4 w-4 shrink-0 animate-spin" />
      <span className="min-w-0 truncate">{message}</span>
    </div>
  );
}
