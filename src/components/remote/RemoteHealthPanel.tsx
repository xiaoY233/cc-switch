import {
  Activity,
  AlertTriangle,
  Download,
  RefreshCw,
  Terminal,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  remoteApi,
  type RemoteConnectionSecret,
  type RemoteHealth,
  type RemoteHostProfile,
} from "@/lib/api";

const EXPECTED_REMOTE_CAPABILITIES = [
  {
    id: "providers",
    labelKey: "remote.capabilities.providers",
    defaultLabel: "供应商",
  },
  {
    id: "openclaw",
    labelKey: "remote.capabilities.openclaw",
    defaultLabel: "OpenClaw",
  },
  { id: "mcp", labelKey: "remote.capabilities.mcp", defaultLabel: "MCP" },
  {
    id: "prompts",
    labelKey: "remote.capabilities.prompts",
    defaultLabel: "提示词",
  },
  {
    id: "skills",
    labelKey: "remote.capabilities.skills",
    defaultLabel: "技能",
  },
  {
    id: "import-export",
    labelKey: "remote.capabilities.importExport",
    defaultLabel: "导入 / 导出",
  },
] as const;

export function RemoteHealthPanel({
  profile,
  secret,
}: {
  profile?: RemoteHostProfile;
  secret?: RemoteConnectionSecret;
}) {
  const { t } = useTranslation();
  const [health, setHealth] = useState<RemoteHealth | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const healthCanReportCapabilities =
    health?.reachable === true && health.helperInstalled === true;
  const capabilitySet = new Set(health?.capabilities ?? []);
  const missingCapabilities = healthCanReportCapabilities
    ? EXPECTED_REMOTE_CAPABILITIES.filter(
        (capability) => !capabilitySet.has(capability.id),
      )
    : [];

  const handleCheck = async () => {
    if (!profile) return;
    setChecking(true);
    try {
      const result = await remoteApi.checkHealth(profile, secret);
      setHealth(result);
    } catch (error) {
      setHealth({
        reachable: false,
        helperInstalled: false,
        capabilities: [],
        lastError: String(error),
      });
    } finally {
      setChecking(false);
    }
  };

  const handleInstall = async () => {
    if (!profile) return;
    setInstalling(true);
    try {
      const result = await remoteApi.installHelper(profile, secret);
      setHealth(result);
      toast.success(
        t("remote.health.installSuccess", {
          defaultValue: "远程 Helper 已安装",
        }),
      );
    } catch (error) {
      const message = String(error);
      setHealth({
        reachable: false,
        helperInstalled: false,
        capabilities: [],
        lastError: message,
      });
      toast.error(
        t("remote.health.installFailed", {
          defaultValue: "安装远程 Helper 失败: {{error}}",
          error: message,
        }),
      );
    } finally {
      setInstalling(false);
    }
  };

  return (
    <section className="glass overflow-hidden rounded-xl border border-white/10">
      <div className="flex h-11 items-center justify-between border-b border-border-default px-4">
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-semibold">
            {t("remote.health.title", { defaultValue: "健康检查" })}
          </span>
          <Badge variant={health?.reachable ? "default" : "outline"}>
            {health
              ? health.reachable
                ? t("remote.health.ok", { defaultValue: "可连接" })
                : t("remote.health.failed", { defaultValue: "失败" })
              : t("remote.health.notChecked", { defaultValue: "未检查" })}
          </Badge>
        </div>
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            disabled={!profile || checking || installing}
            onClick={() => void handleInstall()}
          >
            <Download className="mr-2 h-4 w-4" />
            {installing
              ? t("remote.health.installing", { defaultValue: "安装中" })
              : t("remote.health.install", { defaultValue: "安装 Helper" })}
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={!profile || checking || installing}
            onClick={() => void handleCheck()}
          >
            <RefreshCw className="mr-2 h-4 w-4" />
            {checking
              ? t("common.loading", { defaultValue: "加载中" })
              : t("remote.health.check", { defaultValue: "检查" })}
          </Button>
        </div>
      </div>
      <div className="grid grid-cols-1 gap-0 sm:grid-cols-4">
        <Metric
          label={t("remote.fields.host", { defaultValue: "主机" })}
          value={profile?.host ?? "-"}
        />
        <Metric
          label={t("remote.fields.helperPath", { defaultValue: "Helper 路径" })}
          value={profile?.helperPath ?? "-"}
        />
        <Metric
          label={t("remote.health.helperVersion", {
            defaultValue: "Helper 版本",
          })}
          value={health?.helperVersion ?? "-"}
        />
        <Metric
          label={t("remote.health.platform", { defaultValue: "平台" })}
          value={health?.platform ?? "-"}
        />
      </div>
      {health?.lastError && (
        <div className="border-t border-border-default px-4 py-3 text-xs text-destructive">
          {health.lastError}
        </div>
      )}
      {healthCanReportCapabilities && (
        <div className="border-t border-border-default px-4 py-3">
          <div className="mb-2 text-xs font-medium text-muted-foreground">
            {t("remote.health.capabilities", { defaultValue: "能力" })}
          </div>
          <div className="flex flex-wrap gap-2">
            {EXPECTED_REMOTE_CAPABILITIES.map((capability) => {
              const available = capabilitySet.has(capability.id);
              return (
                <Badge
                  key={capability.id}
                  variant={available ? "secondary" : "outline"}
                  className={cn(
                    "border-border-default",
                    !available &&
                      "border-amber-500/40 bg-amber-500/10 text-amber-600 dark:text-amber-300",
                  )}
                >
                  {t(capability.labelKey, {
                    defaultValue: capability.defaultLabel,
                  })}
                </Badge>
              );
            })}
          </div>
          {missingCapabilities.length > 0 && (
            <div className="mt-3 flex items-start gap-2 rounded-lg border border-amber-500/25 bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-200">
              <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              <div>
                <span className="font-medium">
                  {t("remote.health.missingCapabilities", {
                    defaultValue: "缺少能力",
                  })}
                </span>
                <span className="ml-1">
                  {missingCapabilities
                    .map((capability) =>
                      t(capability.labelKey, {
                        defaultValue: capability.defaultLabel,
                      }),
                    )
                    .join(", ")}
                </span>
              </div>
            </div>
          )}
        </div>
      )}
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 border-b border-border-default px-4 py-3 sm:border-b-0 sm:border-r sm:last:border-r-0">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Terminal className="h-3.5 w-3.5" />
        {label}
      </div>
      <div className="mt-1 truncate text-sm font-medium text-foreground">
        {value}
      </div>
    </div>
  );
}
