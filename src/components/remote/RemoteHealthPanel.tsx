import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

export function RemoteHealthPanel() {
  return (
    <section className="flex items-center justify-between border-b px-4 py-3">
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium">Helper status</span>
        <Badge variant="outline">Not checked</Badge>
      </div>
      <Button size="sm" variant="outline">
        Check
      </Button>
    </section>
  );
}
