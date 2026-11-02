// What "Add your first person" puts in an empty project: one person to
// change, and the next steps as comments to uncomment.

export const STARTER_PERSON_ID = "me"

export const STARTER_SNIPPET = `# Your family tree starts with one person. Change the name and the
# details, then add the people around them.
person me  name:"Your name"  gender:other  born:1990

# Two people marry with a \`marriage\` line, and each of their children
# names that marriage with \`birth\`. Remove the # signs below to try it.
#
# person partner  name:"Your partner"  gender:other
# marriage m_me_partner me partner  start:2015
# person child  name:"Your child"  gender:other  born:2018
#   birth m_me_partner
`

/**
 * The text to add at the end of a file holding `text` so the snippet
 * starts on a line of its own, after a blank line.
 */
export function starterInsertion(text: string): string {
  if (text.trim() === "") return STARTER_SNIPPET
  const separator = text.endsWith("\n\n")
    ? ""
    : text.endsWith("\n")
      ? "\n"
      : "\n\n"
  return separator + STARTER_SNIPPET
}
