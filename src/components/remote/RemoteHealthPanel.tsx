import { Activity, RefreshCw, Terminal } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  remoteApi,
  type RemoteConnectionSecret,
  type RemoteHealth,
  type RemoteHostProfile,
} from "@/lib/api";

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

  return (
    <section className="rounded-xl border border-border bg-card">
      <div className="flex items-center justify-between border-b px-4 py-3">
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
        <Button
          size="sm"
          variant="outline"
          disabled={!profile || checking}
          onClick={() => void handleCheck()}
        >
          <RefreshCw className="mr-2 h-4 w-4" />
          {checking
            ? t("common.loading", { defaultValue: "加载中" })
            : t("remote.health.check", { defaultValue: "检查" })}
        </Button>
      </div>
      <div className="grid grid-cols-3 gap-3 p-4">
        <Metric
          label={t("remote.fields.host", { defaultValue: "主机" })}
          value={profile?.host ?? "-"}
        />
        <Metric
          label={t("remote.fields.helperPath", { defaultValue: "Helper 路径" })}
          value={health?.helperVersion ?? profile?.helperPath ?? "-"}
        />
        <Metric
          label={t("remote.health.platform", { defaultValue: "平台" })}
          value={health?.platform ?? "-"}
        />
      </div>
      {health?.lastError && (
        <div className="border-t px-4 py-3 text-xs text-destructive">
          {health.lastError}
        </div>
      )}
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border bg-background px-3 py-2">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Terminal className="h-3.5 w-3.5" />
        {label}
      </div>
      <div className="mt-1 truncate text-sm font-medium">{value}</div>
    </div>
  );
}
