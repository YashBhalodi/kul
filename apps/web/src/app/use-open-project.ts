import { useNavigate } from "@tanstack/react-router"
import { useCallback } from "react"

/** Navigate to a project's page. */
export function useOpenProject(): (
  id: string,
  options?: { replace?: boolean }
) => void {
  const navigate = useNavigate()
  return useCallback(
    (id, options) =>
      void navigate({
        to: "/app/$projectId/",
        params: { projectId: id },
        replace: options?.replace,
      }),
    [navigate]
  )
}
