import { useMemo, useState } from "react";
import {
  CheckCircle2,
  KeyRound,
  LockKeyhole,
  Plus,
  Server,
  ShieldCheck,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { RemoteAuthMethod, RemoteHostProfile } from "@/lib/api";
import { RemoteHealthPanel } from "./RemoteHealthPanel";
import { RemoteHostDialog } from "./RemoteHostDialog";
import { RemoteProvidersPanel } from "./RemoteProvidersPanel";

export function RemoteServersPage() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [profiles, setProfiles] = useState<RemoteHostProfile[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const selectedProfile = useMemo(
    () => profiles.find((profile) => profile.id === selectedId),
    [profiles, selectedId],
  );

  const handleSave = (profile: RemoteHostProfile) => {
    setProfiles((current) => [profile, ...current]);
    setSelectedId(profile.id);
  };

  return (
    <div className="h-full overflow-y-auto px-6 pb-8 pt-4">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold">Remote servers</h2>
        </div>
        <Button size="sm" onClick={() => setDialogOpen(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Add server
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[320px_minmax(0,1fr)]">
        <section className="rounded-xl border border-border bg-card">
          <div className="flex items-center justify-between border-b px-4 py-3">
            <div className="flex items-center gap-2">
              <Server className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm font-semibold">Connections</span>
            </div>
            <span className="text-xs text-muted-foreground">
              {profiles.length}
            </span>
          </div>

          <div className="grid gap-2 p-3">
            {profiles.length === 0 ? (
              <button
                type="button"
                onClick={() => setDialogOpen(true)}
                className="flex items-center gap-3 rounded-lg border border-dashed p-4 text-left text-sm text-muted-foreground transition-colors hover:bg-accent/50 hover:text-foreground"
              >
                <Plus className="h-4 w-4" />
                Add server
              </button>
            ) : (
              profiles.map((profile) => (
                <button
                  key={profile.id}
                  type="button"
                  onClick={() => setSelectedId(profile.id)}
                  className={cn(
                    "flex items-start gap-3 rounded-lg border p-3 text-left transition-colors",
                    selectedId === profile.id
                      ? "border-primary bg-primary/10"
                      : "border-border bg-background hover:bg-accent/50",
                  )}
                >
                  <AuthIcon authMethod={profile.authMethod} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-medium">
                        {profile.name}
                      </span>
                      {selectedId === profile.id && (
                        <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-primary" />
                      )}
                    </div>
                    <p className="mt-0.5 truncate text-xs text-muted-foreground">
                      {profile.username}@{profile.host}:{profile.port}
                    </p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {authLabel(profile.authMethod)}
                    </p>
                  </div>
                </button>
              ))
            )}
          </div>
        </section>

        <div className="grid content-start gap-4">
          <RemoteHealthPanel profile={selectedProfile} />
          <RemoteProvidersPanel profile={selectedProfile} />

          <section className="rounded-xl border border-border bg-card">
            <div className="border-b px-4 py-3">
              <h2 className="text-sm font-semibold">Connection details</h2>
            </div>
            <div className="grid gap-3 p-4 sm:grid-cols-2">
              <Detail label="Name" value={selectedProfile?.name} />
              <Detail label="Address" value={formatAddress(selectedProfile)} />
              <Detail
                label="Authentication"
                value={
                  selectedProfile
                    ? authLabel(selectedProfile.authMethod)
                    : undefined
                }
              />
              <Detail label="Helper" value={selectedProfile?.helperPath} />
            </div>
          </section>
        </div>
      </div>

      <RemoteHostDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onSave={handleSave}
      />
    </div>
  );
}

function AuthIcon({ authMethod }: { authMethod: RemoteAuthMethod }) {
  const className = "mt-0.5 h-4 w-4 shrink-0 text-muted-foreground";
  if (authMethod.type === "password") {
    return <LockKeyhole className={className} />;
  }
  if (authMethod.type === "keyFile") {
    return <KeyRound className={className} />;
  }
  return <ShieldCheck className={className} />;
}

function Detail({ label, value }: { label: string; value?: string }) {
  return (
    <div className="rounded-lg border bg-background px-3 py-2">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 truncate text-sm font-medium">{value ?? "-"}</div>
    </div>
  );
}

function authLabel(authMethod: RemoteAuthMethod) {
  if (authMethod.type === "password") return "Password";
  if (authMethod.type === "keyFile") return "Key file";
  return "SSH agent";
}

function formatAddress(profile?: RemoteHostProfile) {
  if (!profile) return undefined;
  return `${profile.username}@${profile.host}:${profile.port}`;
}
