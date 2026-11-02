import { cn } from "cn"
import { useId, useRef, useState } from "react"

import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
  InputGroupText,
} from "@/components/ui/input-group"
import {
  describeFileNameProblem,
  fileNameProblem,
  fileStem,
  withKulExtension,
} from "@/store/file-names"

export interface FileNameInputProps {
  /** The accessible name, such as "New file name". */
  label: string
  /** The project's file names, which the new name must not clash with. */
  existing: ReadonlyArray<string>
  /** The file being renamed; its name fills the input. */
  renaming?: string
  onSubmit: (name: string) => void
  onCancel: () => void
  className?: string
}

/**
 * An inline input for a file name. `.kul` is appended for you; the name is
 * checked as you type, and Enter accepts it, Escape gives up. Leaving the
 * field accepts a valid change and drops anything else.
 */
export function FileNameInput({
  label,
  existing,
  renaming,
  onSubmit,
  onCancel,
  className,
}: FileNameInputProps) {
  const initial = renaming === undefined ? "" : fileStem(renaming)
  const [value, setValue] = useState(initial)
  const [attempted, setAttempted] = useState(false)
  const errorId = useId()
  // Enter, Escape and leaving the field each end the input; only the first
  // one counts.
  const ended = useRef(false)
  const submit = (accepted: string) => {
    if (ended.current) return
    ended.current = true
    onSubmit(accepted)
  }
  const cancel = () => {
    if (ended.current) return
    ended.current = true
    onCancel()
  }

  const name = withKulExtension(value)
  const problem = fileNameProblem(name, existing, renaming)
  // An empty field is only a problem once someone tries to accept it.
  const shown = problem === "empty" && !attempted ? undefined : problem
  const unchanged = renaming !== undefined && name === renaming

  function accept() {
    if (unchanged) cancel()
    else if (problem) setAttempted(true)
    else submit(name)
  }

  return (
    <div className={cn("flex flex-col gap-1", className)}>
      <InputGroup className="h-7 bg-background">
        <InputGroupInput
          aria-label={label}
          aria-invalid={shown !== undefined}
          aria-describedby={shown ? errorId : undefined}
          autoFocus
          spellCheck={false}
          autoComplete="off"
          value={value}
          placeholder="name"
          className="h-7 px-2 font-mono text-[0.8125rem]"
          onFocus={(event) => event.currentTarget.select()}
          onChange={(event) => {
            setValue(event.currentTarget.value)
            setAttempted(false)
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault()
              accept()
            } else if (event.key === "Escape") {
              event.preventDefault()
              event.stopPropagation()
              cancel()
            }
          }}
          onBlur={() => {
            if (!problem && !unchanged) submit(name)
            else cancel()
          }}
        />
        <InputGroupAddon align="inline-end">
          <InputGroupText className="font-mono text-xs">.kul</InputGroupText>
        </InputGroupAddon>
      </InputGroup>
      {shown && (
        <p
          id={errorId}
          role="alert"
          className="px-0.5 text-xs text-destructive"
        >
          {describeFileNameProblem(shown)}
        </p>
      )}
    </div>
  )
}
