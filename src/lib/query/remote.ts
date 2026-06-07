import { useMutation, useQuery } from "@tanstack/react-query";
import { remoteApi, type RemoteHostProfile } from "@/lib/api";

export const remoteQueryKeys = {
  all: ["remote"] as const,
  host: (id: string) => ["remote", "host", id] as const,
  session: (id: string) => ["remote", "host", id, "session"] as const,
};

export function useValidateRemoteProfile() {
  return useMutation({
    mutationFn: (profile: RemoteHostProfile) =>
      remoteApi.validateProfile(profile),
  });
}

export function useRemoteSessionStatus(profile?: RemoteHostProfile | null) {
  return useQuery({
    queryKey: profile
      ? remoteQueryKeys.session(profile.id)
      : ["remote", "host", "none", "session"],
    queryFn: () => remoteApi.getSessionStatus(profile!.id),
    enabled: Boolean(profile),
    refetchInterval: profile ? 2_000 : false,
    staleTime: 1_000,
  });
}
