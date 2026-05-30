import { Activity, RefreshCw, Terminal } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { RemoteHostProfile } from "@/lib/api";

export function RemoteHealthPanel({
  profile,
}: {
  profile?: RemoteHostProfile;
}) {
  return (
    <section className="rounded-xl border border-border bg-card">
      <div className="flex items-center justify-between border-b px-4 py-3">
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-semibold">Health</span>
          <Badge variant="outline">Not checked</Badge>
        </div>
        <Button size="sm" variant="outline" disabled={!profile}>
          <RefreshCw className="mr-2 h-4 w-4" />
          Check
        </Button>
      </div>
      <div className="grid grid-cols-3 gap-3 p-4">
        <Metric label="Host" value={profile?.host ?? "-"} />
        <Metric label="Helper" value={profile?.helperPath ?? "-"} />
        <Metric label="Platform" value="-" />
      </div>
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
