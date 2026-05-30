import { useEffect, useState } from "react";
import type { FormEvent, ReactNode } from "react";
import { KeyRound, LockKeyhole, ShieldCheck } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import type { RemoteAuthMethod, RemoteHostProfile } from "@/lib/api";

type RemoteAuthMode = RemoteAuthMethod["type"];

const AUTH_OPTIONS: Array<{
  type: RemoteAuthMode;
  label: string;
  icon: LucideIcon;
}> = [
  { type: "sshAgent", label: "Agent", icon: ShieldCheck },
  { type: "keyFile", label: "Key file", icon: KeyRound },
  { type: "password", label: "Password", icon: LockKeyhole },
];

export function RemoteHostDialog({
  open,
  onOpenChange,
  onSave,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (profile: RemoteHostProfile) => void;
}) {
  const [name, setName] = useState("Development server");
  const [host, setHost] = useState("");
  const [port, setPort] = useState("22");
  const [username, setUsername] = useState("");
  const [helperPath, setHelperPath] = useState("~/.local/bin/cc-switch");
  const [authMode, setAuthMode] = useState<RemoteAuthMode>("sshAgent");
  const [keyPath, setKeyPath] = useState("~/.ssh/id_ed25519");
  const [password, setPassword] = useState("");

  useEffect(() => {
    if (!open) return;
    setName("Development server");
    setHost("");
    setPort("22");
    setUsername("");
    setHelperPath("~/.local/bin/cc-switch");
    setAuthMode("sshAgent");
    setKeyPath("~/.ssh/id_ed25519");
    setPassword("");
  }, [open]);

  const buildAuthMethod = (): RemoteAuthMethod => {
    if (authMode === "keyFile") {
      return { type: "keyFile", path: keyPath };
    }
    if (authMode === "password") {
      return { type: "password" };
    }
    return { type: "sshAgent" };
  };

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const now = Date.now();
    onSave({
      id: `remote-${now}`,
      name: name.trim() || host.trim() || "Remote server",
      host: host.trim(),
      port: Number(port) || 22,
      username: username.trim(),
      authMethod: buildAuthMethod(),
      helperPath: helperPath.trim() || "~/.local/bin/cc-switch",
      createdAt: now,
      updatedAt: now,
    });
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Remote server</DialogTitle>
          </DialogHeader>

          <div className="grid gap-5 px-6 py-5">
            <div className="grid grid-cols-2 gap-3">
              <Field label="Name">
                <Input value={name} onChange={(e) => setName(e.target.value)} />
              </Field>
              <Field label="Host">
                <Input
                  value={host}
                  onChange={(e) => setHost(e.target.value)}
                  placeholder="10.0.0.10"
                  required
                />
              </Field>
              <Field label="Username">
                <Input
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder="deploy"
                  required
                />
              </Field>
              <Field label="Port">
                <Input
                  value={port}
                  onChange={(e) => setPort(e.target.value)}
                  inputMode="numeric"
                  required
                />
              </Field>
            </div>

            <Field label="Helper path">
              <Input
                value={helperPath}
                onChange={(e) => setHelperPath(e.target.value)}
              />
            </Field>

            <div className="grid gap-2">
              <Label>Authentication</Label>
              <div className="grid grid-cols-3 gap-2">
                {AUTH_OPTIONS.map((option) => {
                  const Icon = option.icon;
                  const active = authMode === option.type;
                  return (
                    <button
                      key={option.type}
                      type="button"
                      onClick={() => setAuthMode(option.type)}
                      className={cn(
                        "flex h-10 items-center justify-center gap-2 rounded-lg border text-sm transition-colors",
                        active
                          ? "border-primary bg-primary/10 text-primary"
                          : "border-border bg-card text-muted-foreground hover:bg-accent/50 hover:text-foreground",
                      )}
                    >
                      <Icon className="h-4 w-4" />
                      {option.label}
                    </button>
                  );
                })}
              </div>
            </div>

            {authMode === "keyFile" && (
              <Field label="SSH key path">
                <Input
                  value={keyPath}
                  onChange={(e) => setKeyPath(e.target.value)}
                  required
                />
              </Field>
            )}

            {authMode === "password" && (
              <Field label="Password">
                <Input
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  type="password"
                  required
                />
              </Field>
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button type="submit">Save</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-2">
      <Label>{label}</Label>
      {children}
    </div>
  );
}
