import { useState } from "react";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { RemoteHealthPanel } from "./RemoteHealthPanel";
import { RemoteHostDialog } from "./RemoteHostDialog";
import { RemoteProvidersPanel } from "./RemoteProvidersPanel";

export function RemoteServersPage() {
  const [dialogOpen, setDialogOpen] = useState(false);

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between border-b px-4 py-3">
        <h1 className="text-base font-semibold">Remote Servers</h1>
        <Button size="sm" onClick={() => setDialogOpen(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Add
        </Button>
      </header>
      <RemoteHealthPanel />
      <RemoteProvidersPanel />
      <div className="flex flex-1 items-center justify-center p-6">
        <div className="rounded-md border px-6 py-5 text-center">
          <p className="text-sm font-medium">No remote server selected</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Remote state stays on the selected host.
          </p>
        </div>
      </div>
      <RemoteHostDialog open={dialogOpen} onOpenChange={setDialogOpen} />
    </div>
  );
}
