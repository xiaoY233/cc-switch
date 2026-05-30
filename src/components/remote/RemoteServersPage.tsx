import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  CheckCircle2,
  KeyRound,
  Pencil,
  LockKeyhole,
  Plus,
  Server,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { ListItemRow } from "@/components/common/ListItemRow";
import {
  remoteApi,
  type RemoteAuthMethod,
  type RemoteConnectionSecret,
  type RemoteHostProfile,
} from "@/lib/api";
import { RemoteHealthPanel } from "./RemoteHealthPanel";
import { RemoteHostDialog } from "./RemoteHostDialog";

export function RemoteServersPage({
  profiles,
  activeProfileId,
  activeSecret,
  secrets,
  onProfileSaved,
  onProfileActivated,
  onProfilesChanged,
}: {
  profiles: RemoteHostProfile[];
  activeProfileId?: string;
  activeSecret?: RemoteConnectionSecret;
  secrets?: Record<string, RemoteConnectionSecret>;
  onProfileSaved: (
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ) => void;
  onProfileActivated?: (profileId: string | null) => void;
  onProfilesChanged: (profiles: RemoteHostProfile[]) => void;
}) {
  const { t } = useTranslation();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingProfile, setEditingProfile] =
    useState<RemoteHostProfile | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(
    activeProfileId ?? null,
  );

  const selectedProfile = useMemo(
    () =>
      profiles.find(
        (profile) => profile.id === (selectedId ?? activeProfileId),
      ),
    [profiles, selectedId, activeProfileId],
  );
  const selectedSecret =
    selectedProfile && secrets
      ? secrets[selectedProfile.id]
      : selectedProfile?.id === activeProfileId
        ? activeSecret
        : undefined;

  const openCreateDialog = () => {
    setEditingProfile(null);
    setDialogOpen(true);
  };

  const openEditDialog = (profile: RemoteHostProfile) => {
    setEditingProfile(profile);
    setSelectedId(profile.id);
    onProfileActivated?.(profile.id);
    setDialogOpen(true);
  };

  const handleSave = async (
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ) => {
    const saved = await remoteApi.saveProfile(profile);
    const next = await remoteApi.listProfiles();
    onProfilesChanged(next);
    onProfileSaved(saved, secret);
    setSelectedId(saved.id);
    onProfileActivated?.(saved.id);
    toast.success(t("remote.saved", { defaultValue: "远程服务器已保存" }));
  };

  const handleDelete = async (id: string) => {
    await remoteApi.deleteProfile(id);
    const next = await remoteApi.listProfiles();
    onProfilesChanged(next);
    if (selectedId === id || activeProfileId === id) {
      const nextId = next[0]?.id ?? null;
      setSelectedId(nextId);
      onProfileActivated?.(nextId);
    }
    toast.success(t("remote.deleted", { defaultValue: "远程服务器已删除" }));
  };

  const handleSelect = (profile: RemoteHostProfile) => {
    setSelectedId(profile.id);
    onProfileActivated?.(profile.id);
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden px-6">
      <div className="mb-4 flex shrink-0 items-center justify-between rounded-xl px-1 py-3">
        <div className="min-w-0">
          <div className="text-sm text-muted-foreground">
            {t("remote.subtitle", {
              defaultValue: "只保存连接信息；管理操作在现有页面按目标执行。",
            })}
          </div>
        </div>
        <Button size="sm" onClick={openCreateDialog}>
          <Plus className="mr-2 h-4 w-4" />
          {t("remote.addServer", { defaultValue: "新增服务器" })}
        </Button>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-hidden lg:grid-cols-[360px_minmax(0,1fr)]">
        <section className="glass flex min-h-0 flex-col overflow-hidden rounded-xl border border-white/10">
          <div className="flex h-11 shrink-0 items-center justify-between border-b border-border-default px-4">
            <div className="flex min-w-0 items-center gap-2">
              <Server className="h-4 w-4 shrink-0 text-muted-foreground" />
              <span className="truncate text-sm font-semibold">
                {t("remote.connections", { defaultValue: "连接" })}
              </span>
            </div>
            <span className="text-xs text-muted-foreground">
              {profiles.length}
            </span>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto">
            {profiles.length === 0 ? (
              <div className="p-3">
                <button
                  type="button"
                  onClick={openCreateDialog}
                  className="flex w-full items-center gap-3 rounded-lg border border-dashed border-border-default px-4 py-3 text-left text-sm text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
                >
                  <Plus className="h-4 w-4" />
                  {t("remote.addServer", { defaultValue: "新增服务器" })}
                </button>
              </div>
            ) : (
              profiles.map((profile, index) => {
                const active = (selectedId ?? activeProfileId) === profile.id;
                return (
                  <ListItemRow
                    key={profile.id}
                    isLast={index === profiles.length - 1}
                    className={active ? "bg-primary/5" : undefined}
                  >
                    <button
                      type="button"
                      onClick={() => handleSelect(profile)}
                      className="flex min-w-0 flex-1 items-center gap-3 text-left"
                    >
                      <AuthIcon authMethod={profile.authMethod} />
                      <div className="min-w-0 flex-1">
                        <div className="flex min-w-0 items-center gap-2">
                          <span className="truncate text-sm font-medium text-foreground">
                            {profile.name}
                          </span>
                          {active && (
                            <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-primary" />
                          )}
                        </div>
                        <p className="mt-0.5 truncate text-xs text-muted-foreground">
                          {profile.username}@{profile.host}:{profile.port}
                        </p>
                      </div>
                      <span className="hidden shrink-0 text-xs text-muted-foreground sm:inline">
                        {authLabel(profile.authMethod, t)}
                      </span>
                    </button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 opacity-0 group-hover:opacity-100"
                      onClick={() => openEditDialog(profile)}
                      title={t("common.edit", { defaultValue: "编辑" })}
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 opacity-0 group-hover:opacity-100"
                      onClick={() => void handleDelete(profile.id)}
                      title={t("common.delete", { defaultValue: "删除" })}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </ListItemRow>
                );
              })
            )}
          </div>
        </section>

        <div className="grid min-h-0 content-start gap-4 overflow-y-auto pb-24">
          <RemoteHealthPanel
            profile={selectedProfile}
            secret={selectedSecret}
          />

          <section className="glass overflow-hidden rounded-xl border border-white/10">
            <div className="flex h-11 items-center border-b border-border-default px-4">
              <h2 className="text-sm font-semibold">
                {t("remote.details", { defaultValue: "连接详情" })}
              </h2>
            </div>
            <div className="grid gap-0 sm:grid-cols-2">
              <Detail
                label={t("remote.fields.name", { defaultValue: "名称" })}
                value={selectedProfile?.name}
              />
              <Detail
                label={t("remote.address", { defaultValue: "地址" })}
                value={formatAddress(selectedProfile)}
              />
              <Detail
                label={t("remote.fields.authentication", {
                  defaultValue: "认证方式",
                })}
                value={
                  selectedProfile
                    ? authLabel(selectedProfile.authMethod, t)
                    : undefined
                }
              />
              <Detail
                label={t("remote.fields.helperPath", {
                  defaultValue: "Helper 路径",
                })}
                value={selectedProfile?.helperPath}
              />
            </div>
          </section>
        </div>
      </div>

      <RemoteHostDialog
        open={dialogOpen}
        onOpenChange={(open) => {
          setDialogOpen(open);
          if (!open) setEditingProfile(null);
        }}
        initialProfile={editingProfile ?? undefined}
        initialSecret={
          editingProfile && secrets ? secrets[editingProfile.id] : undefined
        }
        onSave={handleSave}
      />
    </div>
  );
}

function AuthIcon({ authMethod }: { authMethod: RemoteAuthMethod }) {
  const className = "h-4 w-4 shrink-0 text-muted-foreground";
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
    <div className="min-w-0 border-b border-border-default px-4 py-3 sm:odd:border-r">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 truncate text-sm font-medium text-foreground">
        {value ?? "-"}
      </div>
    </div>
  );
}

function authLabel(
  authMethod: RemoteAuthMethod,
  t: (key: string, options?: any) => string,
) {
  if (authMethod.type === "password") {
    return t("remote.auth.password", { defaultValue: "密码" });
  }
  if (authMethod.type === "keyFile") {
    return t("remote.auth.keyFile", { defaultValue: "密钥文件" });
  }
  return t("remote.auth.sshAgent", { defaultValue: "SSH Agent" });
}

function formatAddress(profile?: RemoteHostProfile) {
  if (!profile) return undefined;
  return `${profile.username}@${profile.host}:${profile.port}`;
}
