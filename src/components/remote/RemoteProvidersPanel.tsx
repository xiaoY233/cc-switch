import { Button } from "@/components/ui/button";

export function RemoteProvidersPanel() {
  return (
    <section className="flex flex-col gap-3 border-b p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold">Remote providers</h2>
        <Button size="sm" variant="outline">
          Refresh
        </Button>
      </div>
      <div className="rounded-md border p-3 text-sm text-muted-foreground">
        Provider data will load from the selected remote server.
      </div>
    </section>
  );
}
