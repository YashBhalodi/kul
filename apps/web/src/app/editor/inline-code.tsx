/** Text from the language service, with its `code spans` in the monospace face. */
export function InlineCode({ text }: { text: string }) {
  return (
    <span>
      {text.split("`").map((part, index) =>
        index % 2 === 1 ? (
          <code key={index} className="font-mono text-[0.92em]">
            {part}
          </code>
        ) : (
          part
        )
      )}
    </span>
  )
}
