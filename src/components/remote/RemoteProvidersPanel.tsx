import { RefreshCw, ServerCog } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { RemoteHostProfile } from "@/lib/api";

export function RemoteProvidersPanel({
  profile,
}: {
  profile?: RemoteHostProfile;
}) {
  return (
    <section className="rounded-xl border border-border bg-card">
      <div className="flex items-center justify-between border-b px-4 py-3">
        <div className="flex items-center gap-2">
          <ServerCog className="h-4 w-4 text-muted-foreground" />
          <h2 className="text-sm font-semibold">Remote providers</h2>
        </div>
        <Button size="sm" variant="outline" disabled={!profile}>
          <RefreshCw className="mr-2 h-4 w-4" />
          Refresh
        </Button>
      </div>
      <div className="p-4">
        <div className="rounded-lg border bg-background p-3 text-sm text-muted-foreground">
          {profile ? "No providers loaded" : "No host selected"}
        </div>
      </div>
    </section>
  );
}
