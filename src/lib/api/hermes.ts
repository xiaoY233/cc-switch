import { invoke } from "@tauri-apps/api/core";
import type {
  HermesMemoryKind,
  HermesMemoryLimits,
  HermesModelConfig,
} from "@/types";
import { remoteApi, type ManagementTarget } from "@/lib/api/remote";

const LOCAL_TARGET: ManagementTarget = { type: "local" };

/**
 * Hermes Agent configuration API (CC Switch side).
 *
 * CC Switch intentionally keeps its Hermes surface minimal — deep configuration
 * (model, agent behavior, env vars, skills, cron, logs, analytics) lives in
 * the Hermes Web UI at http://127.0.0.1:9119. CC Switch only reads the `model`
 * section to highlight the active provider and launches the Hermes Web UI for
 * everything else. Writes to `model` happen implicitly via
 * `apply_switch_defaults` when the user switches providers.
 */
export const hermesApi = {
  async getModelConfig(): Promise<HermesModelConfig | null> {
    return await invoke("get_hermes_model_config");
  },

  /**
   * Probe the local Hermes Web UI and open it in the system browser.
   * Optional `path` lets callers deep-link to specific pages like `/config`.
   */
  async openWebUI(path?: string): Promise<void> {
    await invoke("open_hermes_web_ui", { path: path ?? null });
  },

  /** Open the preferred terminal and run `hermes dashboard` (non-blocking). */
  async launchDashboard(): Promise<void> {
    await invoke("launch_hermes_dashboard");
  },

  /**
   * Read one of Hermes' memory blobs (`MEMORY.md` or `USER.md`). Returns an
   * empty string when the file hasn't been created yet.
   */
  async getMemory(
    kind: HermesMemoryKind,
    target: ManagementTarget = LOCAL_TARGET,
  ): Promise<string> {
    if (target.type === "remote") {
      return remoteApi.getHermesMemory(target.profile, kind, target.secret);
    }
    return await invoke("get_hermes_memory", { kind });
  },

  /** Atomically overwrite a Hermes memory file. */
  async setMemory(
    kind: HermesMemoryKind,
    content: string,
    target: ManagementTarget = LOCAL_TARGET,
  ): Promise<void> {
    if (target.type === "remote") {
      await remoteApi.setHermesMemory(
        target.profile,
        kind,
        content,
        target.secret,
      );
      return;
    }
    await invoke("set_hermes_memory", { kind, content });
  },

  /**
   * Character budgets + enable flags for both memory blobs, read from
   * config.yaml with Hermes defaults as fallback.
   */
  async getMemoryLimits(
    target: ManagementTarget = LOCAL_TARGET,
  ): Promise<HermesMemoryLimits> {
    if (target.type === "remote") {
      return remoteApi.getHermesMemoryLimits(target.profile, target.secret);
    }
    return await invoke("get_hermes_memory_limits");
  },

  /**
   * Toggle the on/off flag for one memory blob. Other fields in the `memory:`
   * section (budgets, external provider config) are preserved.
   */
  async setMemoryEnabled(
    kind: HermesMemoryKind,
    enabled: boolean,
    target: ManagementTarget = LOCAL_TARGET,
  ): Promise<void> {
    if (target.type === "remote") {
      await remoteApi.setHermesMemoryEnabled(
        target.profile,
        kind,
        enabled,
        target.secret,
      );
      return;
    }
    await invoke("set_hermes_memory_enabled", { kind, enabled });
  },
};
