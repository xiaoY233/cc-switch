import { invoke } from "@tauri-apps/api/core";
import type {
  McpServer,
  McpServersMap,
  OpenClawAgentsDefaults,
  OpenClawDefaultModel,
  OpenClawEnvConfig,
  OpenClawToolsConfig,
  OpenClawWriteOutcome,
  Provider,
} from "@/types";
import type { AppId } from "./types";
import type { SwitchResult } from "./providers";
import type { Prompt } from "./prompts";
import type {
  DiscoverableSkill,
  ImportSkillSelection,
  InstalledSkill,
  SkillBackupEntry,
  SkillRepo,
  SkillUninstallResult,
  SkillUpdateInfo,
  UnmanagedSkill,
} from "./skills";

export type RemoteAuthMethod =
  | { type: "sshAgent" }
  | { type: "keyFile"; path: string }
  | { type: "password" };

export interface RemoteHostProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  authMethod: RemoteAuthMethod;
  helperPath: string;
  createdAt: number;
  updatedAt: number;
}

export interface RemoteConnectionSecret {
  password?: string;
}

export interface RemoteHealth {
  reachable: boolean;
  helperInstalled: boolean;
  helperVersion?: string;
  platform?: "linux" | "macos" | "unknown";
  capabilities: string[];
  lastError?: string;
}

export type ManagementTarget =
  | { type: "local" }
  | {
      type: "remote";
      profile: RemoteHostProfile;
      secret?: RemoteConnectionSecret;
    };

export const remoteApi = {
  listProfiles(): Promise<RemoteHostProfile[]> {
    return invoke<RemoteHostProfile[]>("remote_list_profiles");
  },

  saveProfile(profile: RemoteHostProfile): Promise<RemoteHostProfile> {
    return invoke<RemoteHostProfile>("remote_save_profile", { profile });
  },

  deleteProfile(id: string): Promise<boolean> {
    return invoke<boolean>("remote_delete_profile", { id });
  },

  validateProfile(profile: RemoteHostProfile): Promise<boolean> {
    return invoke<boolean>("remote_validate_profile", { profile });
  },

  buildStatusCommand(profile: RemoteHostProfile): Promise<string[]> {
    return invoke<string[]>("remote_build_status_command", { profile });
  },

  buildHelperInstallCommand(profile: RemoteHostProfile): Promise<string[]> {
    return invoke<string[]>("remote_build_helper_install_command", { profile });
  },

  checkHealth(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<RemoteHealth> {
    return invoke<RemoteHealth>("remote_check_health", { profile, secret });
  },

  installHelper(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<RemoteHealth> {
    return invoke<RemoteHealth>("remote_install_helper", { profile, secret });
  },

  getProviders(
    profile: RemoteHostProfile,
    app: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<Record<string, Provider>> {
    return invoke<Record<string, Provider>>("remote_get_providers", {
      profile,
      app,
      secret,
    });
  },

  getCurrentProvider(
    profile: RemoteHostProfile,
    app: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<string> {
    return invoke<string>("remote_get_current_provider", {
      profile,
      app,
      secret,
    });
  },

  switchProvider(
    profile: RemoteHostProfile,
    app: AppId,
    id: string,
    secret?: RemoteConnectionSecret,
  ): Promise<SwitchResult> {
    return invoke<SwitchResult>("remote_switch_provider", {
      profile,
      app,
      id,
      secret,
    });
  },

  addProvider(
    profile: RemoteHostProfile,
    app: AppId,
    provider: Provider,
    addToLive?: boolean,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_add_provider", {
      profile,
      app,
      provider,
      addToLive,
      secret,
    });
  },

  updateProvider(
    profile: RemoteHostProfile,
    app: AppId,
    provider: Provider,
    originalId?: string,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_update_provider", {
      profile,
      app,
      provider,
      originalId,
      secret,
    });
  },

  deleteProvider(
    profile: RemoteHostProfile,
    app: AppId,
    id: string,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_delete_provider", {
      profile,
      app,
      id,
      secret,
    });
  },

  importProviders(
    profile: RemoteHostProfile,
    app: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_import_providers", {
      profile,
      app,
      secret,
    });
  },

  getOpenClawDefaultModel(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<OpenClawDefaultModel | null> {
    return invoke<OpenClawDefaultModel | null>(
      "remote_get_openclaw_default_model",
      {
        profile,
        secret,
      },
    );
  },

  setOpenClawDefaultModel(
    profile: RemoteHostProfile,
    model: OpenClawDefaultModel,
    secret?: RemoteConnectionSecret,
  ): Promise<OpenClawWriteOutcome> {
    return invoke<OpenClawWriteOutcome>("remote_set_openclaw_default_model", {
      profile,
      model,
      secret,
    });
  },

  getOpenClawEnv(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<OpenClawEnvConfig> {
    return invoke<OpenClawEnvConfig>("remote_get_openclaw_env", {
      profile,
      secret,
    });
  },

  setOpenClawEnv(
    profile: RemoteHostProfile,
    env: OpenClawEnvConfig,
    secret?: RemoteConnectionSecret,
  ): Promise<OpenClawWriteOutcome> {
    return invoke<OpenClawWriteOutcome>("remote_set_openclaw_env", {
      profile,
      env,
      secret,
    });
  },

  getOpenClawTools(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<OpenClawToolsConfig> {
    return invoke<OpenClawToolsConfig>("remote_get_openclaw_tools", {
      profile,
      secret,
    });
  },

  setOpenClawTools(
    profile: RemoteHostProfile,
    tools: OpenClawToolsConfig,
    secret?: RemoteConnectionSecret,
  ): Promise<OpenClawWriteOutcome> {
    return invoke<OpenClawWriteOutcome>("remote_set_openclaw_tools", {
      profile,
      tools,
      secret,
    });
  },

  getOpenClawAgentsDefaults(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<OpenClawAgentsDefaults | null> {
    return invoke<OpenClawAgentsDefaults | null>(
      "remote_get_openclaw_agents_defaults",
      {
        profile,
        secret,
      },
    );
  },

  setOpenClawAgentsDefaults(
    profile: RemoteHostProfile,
    defaults: OpenClawAgentsDefaults,
    secret?: RemoteConnectionSecret,
  ): Promise<OpenClawWriteOutcome> {
    return invoke<OpenClawWriteOutcome>(
      "remote_set_openclaw_agents_defaults",
      {
        profile,
        defaults,
        secret,
      },
    );
  },

  getMcpServers(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<McpServersMap> {
    return invoke<McpServersMap>("remote_get_mcp_servers", {
      profile,
      secret,
    });
  },

  upsertMcpServer(
    profile: RemoteHostProfile,
    server: McpServer,
    secret?: RemoteConnectionSecret,
  ): Promise<void> {
    return invoke<void>("remote_upsert_mcp_server", {
      profile,
      server,
      secret,
    });
  },

  deleteMcpServer(
    profile: RemoteHostProfile,
    id: string,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_delete_mcp_server", {
      profile,
      id,
      secret,
    });
  },

  toggleMcpApp(
    profile: RemoteHostProfile,
    serverId: string,
    app: AppId,
    enabled: boolean,
    secret?: RemoteConnectionSecret,
  ): Promise<void> {
    return invoke<void>("remote_toggle_mcp_app", {
      profile,
      serverId,
      app,
      enabled,
      secret,
    });
  },

  importMcpFromApps(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<number> {
    return invoke<number>("remote_import_mcp_from_apps", {
      profile,
      secret,
    });
  },

  getPrompts(
    profile: RemoteHostProfile,
    app: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<Record<string, Prompt>> {
    return invoke<Record<string, Prompt>>("remote_get_prompts", {
      profile,
      app,
      secret,
    });
  },

  upsertPrompt(
    profile: RemoteHostProfile,
    app: AppId,
    id: string,
    prompt: Prompt,
    secret?: RemoteConnectionSecret,
  ): Promise<void> {
    return invoke<void>("remote_upsert_prompt", {
      profile,
      app,
      id,
      prompt,
      secret,
    });
  },

  deletePrompt(
    profile: RemoteHostProfile,
    app: AppId,
    id: string,
    secret?: RemoteConnectionSecret,
  ): Promise<void> {
    return invoke<void>("remote_delete_prompt", {
      profile,
      app,
      id,
      secret,
    });
  },

  enablePrompt(
    profile: RemoteHostProfile,
    app: AppId,
    id: string,
    secret?: RemoteConnectionSecret,
  ): Promise<void> {
    return invoke<void>("remote_enable_prompt", {
      profile,
      app,
      id,
      secret,
    });
  },

  importPromptFromFile(
    profile: RemoteHostProfile,
    app: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<string> {
    return invoke<string>("remote_import_prompt_from_file", {
      profile,
      app,
      secret,
    });
  },

  getCurrentPromptFileContent(
    profile: RemoteHostProfile,
    app: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<string | null> {
    return invoke<string | null>("remote_get_current_prompt_file_content", {
      profile,
      app,
      secret,
    });
  },

  getInstalledSkills(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<InstalledSkill[]> {
    return invoke<InstalledSkill[]>("remote_get_installed_skills", {
      profile,
      secret,
    });
  },

  getSkillBackups(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<SkillBackupEntry[]> {
    return invoke<SkillBackupEntry[]>("remote_get_skill_backups", {
      profile,
      secret,
    });
  },

  deleteSkillBackup(
    profile: RemoteHostProfile,
    backupId: string,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_delete_skill_backup", {
      profile,
      backupId,
      secret,
    });
  },

  installSkillUnified(
    profile: RemoteHostProfile,
    skill: DiscoverableSkill,
    currentApp: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<InstalledSkill> {
    return invoke<InstalledSkill>("remote_install_skill_unified", {
      profile,
      skill,
      currentApp,
      secret,
    });
  },

  uninstallSkillUnified(
    profile: RemoteHostProfile,
    id: string,
    secret?: RemoteConnectionSecret,
  ): Promise<SkillUninstallResult> {
    return invoke<SkillUninstallResult>("remote_uninstall_skill_unified", {
      profile,
      id,
      secret,
    });
  },

  restoreSkillBackup(
    profile: RemoteHostProfile,
    backupId: string,
    currentApp: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<InstalledSkill> {
    return invoke<InstalledSkill>("remote_restore_skill_backup", {
      profile,
      backupId,
      currentApp,
      secret,
    });
  },

  toggleSkillApp(
    profile: RemoteHostProfile,
    id: string,
    app: AppId,
    enabled: boolean,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_toggle_skill_app", {
      profile,
      id,
      app,
      enabled,
      secret,
    });
  },

  scanUnmanagedSkills(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<UnmanagedSkill[]> {
    return invoke<UnmanagedSkill[]>("remote_scan_unmanaged_skills", {
      profile,
      secret,
    });
  },

  importSkillsFromApps(
    profile: RemoteHostProfile,
    imports: ImportSkillSelection[],
    secret?: RemoteConnectionSecret,
  ): Promise<InstalledSkill[]> {
    return invoke<InstalledSkill[]>("remote_import_skills_from_apps", {
      profile,
      imports,
      secret,
    });
  },

  discoverAvailableSkills(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<DiscoverableSkill[]> {
    return invoke<DiscoverableSkill[]>("remote_discover_available_skills", {
      profile,
      secret,
    });
  },

  checkSkillUpdates(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<SkillUpdateInfo[]> {
    return invoke<SkillUpdateInfo[]>("remote_check_skill_updates", {
      profile,
      secret,
    });
  },

  updateSkill(
    profile: RemoteHostProfile,
    id: string,
    secret?: RemoteConnectionSecret,
  ): Promise<InstalledSkill> {
    return invoke<InstalledSkill>("remote_update_skill", {
      profile,
      id,
      secret,
    });
  },

  getSkillRepos(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<SkillRepo[]> {
    return invoke<SkillRepo[]>("remote_get_skill_repos", {
      profile,
      secret,
    });
  },

  addSkillRepo(
    profile: RemoteHostProfile,
    repo: SkillRepo,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_add_skill_repo", {
      profile,
      repo,
      secret,
    });
  },

  removeSkillRepo(
    profile: RemoteHostProfile,
    owner: string,
    name: string,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_remove_skill_repo", {
      profile,
      owner,
      name,
      secret,
    });
  },
};
