import { Toaster } from "@/components/ui/sonner"
import { useUpdateToast } from "./update-toast"

/** The landing page's and the docs' toaster, with the update toast. */
export default function SiteUpdateToast() {
  useUpdateToast()
  return <Toaster position="bottom-center" />
}
