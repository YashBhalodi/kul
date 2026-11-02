import { cn } from "cn"

/** Kul's mark: two spouses joined by a marriage, with a child below. */
export function KulMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 32 32"
      aria-hidden="true"
      className={cn("size-6 text-primary", className)}
    >
      <rect
        x="2"
        y="3"
        width="10"
        height="9"
        rx="2.5"
        fill="currentColor"
        opacity="0.9"
      />
      <rect
        x="20"
        y="3"
        width="10"
        height="9"
        rx="2.5"
        fill="currentColor"
        opacity="0.9"
      />
      <path
        d="M12 7.5h8M16 7.5V20"
        stroke="currentColor"
        strokeWidth="2.5"
        strokeLinecap="round"
        fill="none"
      />
      <rect x="11" y="20" width="10" height="9" rx="2.5" fill="currentColor" />
    </svg>
  )
}

/** The mark and the "Kul" wordmark. */
export function KulLogo({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-2 text-base font-semibold tracking-tight",
        className
      )}
    >
      <KulMark />
      <span>Kul</span>
    </span>
  )
}
