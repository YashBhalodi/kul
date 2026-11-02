import { expect, test } from "./fixtures"

import {
  chooseFromSwitcher,
  createFile,
  expectFiles,
  openProject,
} from "./support"

test("one tab owns a project; another can take it over", async ({
  context,
}) => {
  const first = await context.newPage()
  await openProject(first)
  const url = first.url()

  const second = await context.newPage()
  await openProject(second, url)
  const secondBanner = second.getByTestId("read-only-banner")
  const firstBanner = first.getByTestId("read-only-banner")
  await expect(secondBanner).toContainText("open in another tab")
  await expect(firstBanner).toBeHidden()
  await expect(second.getByRole("button", { name: "New file" })).toHaveCount(0)
  await expect(second.getByTestId("save-status")).toHaveText("Read-only")

  // The read-only tab shows what the owner writes.
  await createFile(first, "added")
  await expectFiles(second, ["added.kul", "three-generations.kul"])

  // Taking over swaps the roles.
  await secondBanner.getByRole("button", { name: "Use here instead" }).click()
  await expect(secondBanner).toBeHidden()
  await expect(firstBanner).toBeVisible()
  await expect(first.getByRole("button", { name: "New file" })).toHaveCount(0)

  await createFile(second, "from-second")
  await expectFiles(first, [
    "added.kul",
    "from-second.kul",
    "three-generations.kul",
  ])

  // And back.
  await firstBanner.getByRole("button", { name: "Use here instead" }).click()
  await expect(firstBanner).toBeHidden()
  await expect(secondBanner).toBeVisible()

  // The owner deletes the project; the other tab finds it gone.
  await chooseFromSwitcher(first, ["Delete project…"])
  await first.getByRole("button", { name: "Delete project" }).click()
  await expect(second.getByTestId("project-missing")).toBeVisible()
})
