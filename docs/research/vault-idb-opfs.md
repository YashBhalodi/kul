# Origin-private vault: IndexedDB vs the Origin Private File System

Research for [issue #331](https://github.com/YashBhalodi/kul/issues/331). This note compares the two origin-private storage primitives a browser app can use as a **vault**. It does **not** pick a mechanism; that decision is [issue #335](https://github.com/YashBhalodi/kul/issues/335).

Captured 2026-09-17 from the primary sources listed below.

## Vocabulary

These words are used as in the ticket, not as synonyms of WHATWG Storage terms:

- **Kul project** — a directory containing one `kul.yml` plus sibling `*.kul` files. The manifest is required; discovery is directory-scoped and does not walk subdirectories. Defined normatively in [`spec/14-project-manifest.md`](../../spec/14-project-manifest.md).
- **Vault** — the origin-private store the app owns. The user does not pick a folder; the app holds the bytes.
- **Shelf** — the list of vault-backed Kul projects.

The [Storage Standard](https://storage.spec.whatwg.org/) uses **storage shelf** for a different thing: the per-origin container that holds storage buckets. This note says **storage shelf** (spec) vs **shelf** (product) when both appear.

## Sources

Primary sources only. Compat numbers are from Can I Use / MDN (BCD-backed). Vendor policy that MDN already summarises is cited from MDN; the Storage and File System standards are cited for the normative model.

- [Storage Standard](https://storage.spec.whatwg.org/) (WHATWG, living; last updated 2026-03-15 at fetch time)
- [Indexed Database API 3.0](https://www.w3.org/TR/IndexedDB/)
- [File System Standard](https://fs.spec.whatwg.org/) (WHATWG) — current definition of the origin-private **bucket file system** (`navigator.storage.getDirectory()`)
- [File System Access](https://wicg.github.io/file-system-access/) (WICG) — user-visible pickers that *extend* the File System Standard; OPFS itself lived here before the 2022 move to WHATWG ([WICG/file-system-access#342](https://github.com/WICG/file-system-access/issues/342))
- [Clear Site Data](https://w3c.github.io/webappsec-clear-site-data/#header)
- MDN: [Storage quotas and eviction criteria](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria), [StorageManager.persist()](https://developer.mozilla.org/en-US/docs/Web/API/StorageManager/persist), [StorageManager.estimate()](https://developer.mozilla.org/en-US/docs/Web/API/StorageManager/estimate), [StorageManager.getDirectory()](https://developer.mozilla.org/en-US/docs/Web/API/StorageManager/getDirectory), [IndexedDB API](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API), [IDBTransaction](https://developer.mozilla.org/en-US/docs/Web/API/IDBTransaction), [Origin private file system](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system), [File System API](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API), [FileSystemFileHandle.createWritable()](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemFileHandle/createWritable), [FileSystemFileHandle.createSyncAccessHandle()](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemFileHandle/createSyncAccessHandle), [Blob()](https://developer.mozilla.org/en-US/docs/Web/API/Blob/Blob), [URL.createObjectURL()](https://developer.mozilla.org/en-US/docs/Web/API/URL/createObjectURL_static), [`<a download>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/a#download), [Clear-Site-Data](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Clear-Site-Data), [structured clone algorithm](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Structured_clone_algorithm)
- Can I Use: [IndexedDB](https://caniuse.com/indexeddb), [StorageManager.persist](https://caniuse.com/mdn-api_storagemanager_persist), [StorageManager.getDirectory](https://caniuse.com/mdn-api_storagemanager_getdirectory), [createWritable](https://caniuse.com/mdn-api_filesystemfilehandle_createwritable), [createSyncAccessHandle](https://caniuse.com/mdn-api_filesystemfilehandle_createsyncaccesshandle), [File System Access (user-visible)](https://caniuse.com/native-filesystem-api)

## Shared origin-storage facts

IndexedDB and OPFS are not two isolated disks. They are two **storage endpoints** on the same origin-keyed local bucket.

The Storage Standard isolates local data by **storage key** (today: origin). A **storage shelf** (spec) holds a default **storage bucket**. That bucket has a **mode** of `"best-effort"` (initial) or `"persistent"`, and a **bottle** per endpoint (`"indexedDB"`, and — defined by the File System Standard — `"fileSystem"`). ([Storage Standard §§4.1–4.5](https://storage.spec.whatwg.org/#storage-endpoints); [File System Standard §3](https://fs.spec.whatwg.org/#local-filesystems))

MDN lists IndexedDB and OPFS as technologies that store data in the browser under that origin quota. ([MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria))

### Quota

Quota is per origin, not per API. `navigator.storage.estimate()` returns an implementation-defined **usage** and **quota** for the storage shelf (spec); the values are padded / approximate and must not be a function of free disk (fingerprinting). ([Storage Standard §6 and `estimate()`](https://storage.spec.whatwg.org/#usage-and-quota); [MDN, estimate()](https://developer.mozilla.org/en-US/docs/Web/API/StorageManager/estimate))

Exceeding quota on IndexedDB, Cache, or OPFS fails with `QuotaExceededError`. ([MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria); [File System Standard](https://fs.spec.whatwg.org/) notes quota only applies to the bucket file system)

Browser-reported origin caps (MDN; calculated from **total** disk, so an origin may not actually be able to fill them):

| Engine | Best-effort origin cap | Persistent origin cap |
| --- | --- | --- |
| Firefox | Smaller of 10% of profile disk and 10 GiB **group** limit (same eTLD+1) | 50% of disk, capped at 8 TiB; no group limit |
| Chromium (Chrome, Edge) | Up to 60% of total disk | Same 60% |
| WebKit browser apps (Safari 17+ / macOS 14 / iOS 17+) | Around 60% of total disk; overall browser cap 80% of disk across origins | Same shape; persist is still a mode, not a larger number |

([MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria))

Older Safari gave an origin 1 GiB then prompted. Embedded WKWebView (not a default-browser app) is ~15% of disk unless the site is a Home Screen / Dock web app. ([MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria))

### Best-effort vs `navigator.storage.persist()`

Default mode is **best-effort**: data lasts while the origin is under quota, the device has space, and the user has not deleted it. ([MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria); [Storage Standard §4.5](https://storage.spec.whatwg.org/#storage-buckets))

`navigator.storage.persist()` requests the `"persistent-storage"` permission and, if granted, sets the **default bucket** mode to `"persistent"`. The method is Window-only (not Workers), secure-context, and resolves `true`/`false`. The browser may refuse. ([Storage Standard §§5, 8](https://storage.spec.whatwg.org/#persistence-permission); [MDN, persist()](https://developer.mozilla.org/en-US/docs/Web/API/StorageManager/persist))

Grant behaviour (MDN):

- **Firefox** shows a permission popup.
- **Safari and most Chromium browsers** auto-approve or deny from interaction history; no prompt.

([MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria))

Persistence is **origin-wide**, not per API. Granting it protects IndexedDB *and* OPFS (and other local bottles) from the user agent's automatic clearing policy. The user agent still must not clear a persistent bucket without involvement from the origin or the user. ([Storage Standard §5](https://storage.spec.whatwg.org/#persistence-permission); [Storage Standard §7.1](https://storage.spec.whatwg.org/#storage-pressure))

`persist()` does **not** survive the user clearing site data. That *is* user involvement. See [Clearing site data](#clearing-site-data-neither-survives).

Can I Use: `StorageManager.persist` is in Chrome 55+, Firefox 57+, Safari 15.2+ (global ~96%). ([caniuse persist](https://caniuse.com/mdn-api_storagemanager_persist))

### Eviction

Under storage pressure, user agents should clear **best-effort** local buckets (MDN: LRU by origin). Persistent origins are skipped. Eviction of an origin deletes **all** of its stored data at once — IndexedDB and Cache (and, same rule, OPFS), not a subset — so a hybrid vault cannot keep one primitive after the other is evicted. ([Storage Standard §7](https://storage.spec.whatwg.org/#management); [MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria))

Safari **also** proactively deletes script-created origin data when Intelligent Tracking Prevention is on and the origin has had no user interaction (click/tap) in the last seven days of browser use. Server-set cookies are exempt; vault bytes are not. ([MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria))

Private browsing typically uses different quotas and drops stored data when the session ends. Some browsers throw if `getDirectory()` is called there. ([MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria); [MDN, getDirectory()](https://developer.mozilla.org/en-US/docs/Web/API/StorageManager/getDirectory))

### Clearing site data (neither survives)

User agents should offer a UI to clear storage for a website and should not distinguish “network” from “storage” in that UI. ([Storage Standard §7.2](https://storage.spec.whatwg.org/#user-interface-guidelines))

MDN is explicit for OPFS: **clearing storage data for the site deletes the OPFS**. ([MDN, OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system))

The `Clear-Site-Data: "storage"` response header tells the user agent to remove DOM storage for the origin, including IndexedDB (`IDBFactory.deleteDatabase` per database) and File System API data. `"*"` covers all current and future types. ([MDN, Clear-Site-Data](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Clear-Site-Data); [Clear Site Data spec](https://w3c.github.io/webappsec-clear-site-data/#header))

Neither primitive can hide from that. A vault that only exists origin-privately is gone after “Clear site data”, `Clear-Site-Data: "storage"`, or equivalent settings. Persistence-permission only changes **automatic** eviction.

## IndexedDB: structured records

IndexedDB is a transactional, same-origin object store for structured values, including files/blobs. Values are any [structured-cloneable](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Structured_clone_algorithm) object — strings, `Object`/`Array`, `Blob`, `File`, and also `FileSystemHandle` / `FileSystemFileHandle` / `FileSystemDirectoryHandle`. Records are stored **by value**, not by reference. ([MDN, IndexedDB](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API); [IndexedDB 3.0 §2, values](https://www.w3.org/TR/IndexedDB/); [MDN, File System API](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API); [MDN, structured clone](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Structured_clone_algorithm))

It is not a file system. There are no directories, no filenames the OS would recognise, and no `File` on disk unless the app puts a `Blob`/`File` (or a handle) in a record.

### Transactions and durability

Every read and write runs in a transaction. A transaction is an **atomic and durable** set of accesses; it can update many records (or fail as a whole). Modes: `"readonly"`, `"readwrite"`, `"versionchange"`. Overlapping `"readwrite"` scopes are serialised. ([IndexedDB 3.0 §2.7](https://www.w3.org/TR/IndexedDB/); [MDN, IDBTransaction](https://developer.mozilla.org/en-US/docs/Web/API/IDBTransaction))

IndexedDB 3.0 adds a **durability hint** on `IDBDatabase.transaction()`: `"strict"` (consider commit only after changes are on a persistent medium), `"relaxed"` (OS write is enough), `"default"`. The spec encourages `"strict"` when data loss outweighs cost. Support note in the spec: Chrome/Edge 82, Firefox 126, Safari 15. ([IndexedDB 3.0 §2.7](https://www.w3.org/TR/IndexedDB/))

Firefox 40+ fires `complete` after telling the OS to write, possibly before a disk flush, unless a non-standard `readwriteflush` mode is used. That is a documented durability relaxation, not a spec violation of `"default"`. ([MDN, IDBTransaction](https://developer.mozilla.org/en-US/docs/Web/API/IDBTransaction))

Quota-exceeded and I/O errors abort the transaction. ([MDN, IDBTransaction](https://developer.mozilla.org/en-US/docs/Web/API/IDBTransaction))

### Compat

Can I Use: IndexedDB is global ~97%. Current Chrome (23+), Firefox (16+), Safari 15+ (Safari 14.1 / iOS 14.5 marked partial). ([caniuse IndexedDB](https://caniuse.com/indexeddb))

For a 2026 app this is the older, more universal of the two primitives.

## OPFS: real files, origin-private

The origin-private file system is the File System Standard’s **bucket file system**: `navigator.storage.getDirectory()` returns a `FileSystemDirectoryHandle` for a directory the site may use **without a picker**. Access algorithms on that tree return `"granted"`. Contents are typically on disk but **not user-visible** and not expected to appear under the names the app uses. ([File System Standard, abstract and §3](https://fs.spec.whatwg.org/); [MDN, OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system))

This is the WICG OPFS idea after the move out of File System Access. WICG File System Access now adds **user-visible** pickers (`showOpenFilePicker`, `showSaveFilePicker`, `showDirectoryPicker`) on top of the same handle types. Those pickers are a different product surface (user-owned folders), not the vault. ([WICG File System Access](https://wicg.github.io/file-system-access/); [WICG#342](https://github.com/WICG/file-system-access/issues/342))

OPFS is subject to the same origin quota as IndexedDB. `estimate()` reports the shared usage. No extra permission prompt. ([MDN, OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system))

### Files vs records

From the root handle the app creates directories and files (`getDirectoryHandle` / `getFileHandle`, `{ create: true }`), lists with the async iterator, reads via `getFile()` → `File`/`Blob` (`Blob.text()` for UTF-8), deletes with `remove()` / `removeEntry()`. ([MDN, OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system))

That is a **directory tree**. A Kul project’s on-disk shape (`kul.yml` + sibling `*.kul`) can be stored as that shape.

### Writes, atomicity, durability — not transactions

OPFS has **no** IndexedDB-style transaction across files.

**Main-thread / async path — `createWritable()`.** Changes are not visible on the handle until the stream is closed. Typical implementation: write a temporary file, then replace. The File System Standard’s close algorithm “atomically updates the contents of the file on disk” for that one file. `keepExistingData` copies the existing file into the temp first. Multiple writers default to `"siloed"` (last close wins). ([MDN, createWritable()](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemFileHandle/createWritable); [File System Standard §2.3.2](https://fs.spec.whatwg.org/#api-filesystemfilehandle-createwritable))

**Worker-only sync path — `createSyncAccessHandle()`.** Dedicated workers, OPFS files only. In-place `read`/`write`/`truncate`, exclusive lock by default. `flush()` pushes modifications to the file; `close()` does **not** promise device durability unless `flush()` ran first. ([MDN, createSyncAccessHandle()](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemFileHandle/createSyncAccessHandle); [File System Standard §2.6](https://fs.spec.whatwg.org/#api-filesystemsyncaccesshandle))

Recursive directory removal “can fail non-atomically”: some children gone, others left. ([File System Standard, `removeEntry`](https://fs.spec.whatwg.org/))

Saving a Kul project (manifest + several `.kul` files) is therefore several independent file operations. A crash between them can leave a half-written project. IndexedDB can put the same set in one `"readwrite"` transaction.

### Compat (the real gaps)

`getDirectory()` (OPFS root): Chrome 86+, Firefox 111+, Safari 15.2+; global ~95%. ([caniuse getDirectory](https://caniuse.com/mdn-api_storagemanager_getdirectory); [MDN, OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system) — Baseline, widely available since March 2023)

`createSyncAccessHandle()`: Chrome 102+, Firefox 111+, Safari 15.2+; dedicated worker only. ([caniuse createSyncAccessHandle](https://caniuse.com/mdn-api_filesystemfilehandle_createsyncaccesshandle); [MDN](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemFileHandle/createSyncAccessHandle))

`createWritable()`: Chrome 86+, Firefox 111+, **Safari 26.0+** (not 15.2–18.7). MDN marks this Baseline 2025, newly available September 2025. ([caniuse createWritable](https://caniuse.com/mdn-api_filesystemfilehandle_createwritable); [MDN, createWritable()](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemFileHandle/createWritable))

So:

- **Chrome / Edge:** OPFS read+async write since 86; sync handles since 102.
- **Firefox:** OPFS (root, writable, sync handle) since 111. No user-visible File System Access pickers ([caniuse native-filesystem-api](https://caniuse.com/native-filesystem-api) — Firefox and Safari unsupported).
- **Safari 15.2–25 / iOS 15.2–18:** OPFS root and worker `createSyncAccessHandle` exist; **main-thread `createWritable` does not**. A vault that writes OPFS on Safari in that range must use a dedicated worker. Safari releases are OS-tied; “Safari 26” is not every installed Safari in the field.

User-visible pickers (`showSaveFilePicker`, etc.) are Chromium-only (~31% global) and are **not** the vault. ([caniuse native-filesystem-api](https://caniuse.com/native-filesystem-api))

## How each stores a shelf and a Kul project

A Kul project is `kul.yml` + sibling `*.kul` ([spec §14](../../spec/14-project-manifest.md)). The shelf is the list of those projects. Both primitives can hold that; they hold it differently.

### IndexedDB

Natural shape: object stores, not folders.

One working mapping (illustrative, not a design):

- `projects` — one record per shelf entry: `{ id, title, updatedAt, … }`. The shelf **is** this object store (cursor / `getAll()`).
- `files` — one record per file: `{ projectId, name, body }` where `body` is a string or `Blob`/`File` (IndexedDB must store those types). Composite index `(projectId, name)` reconstructs a project.

A `"readwrite"` transaction whose scope is both stores can create/update/delete a project and all of its files **atomically**. That matches “save this Kul project” and “remove this shelf entry” as one failure domain. ([IndexedDB 3.0 §2.7](https://www.w3.org/TR/IndexedDB/))

The bytes are records. There is no `kul.yml` path until export reconstitutes names.

### OPFS

Natural shape: a directory per Kul project.

One working mapping:

- Root: one directory per project id (or slug).
- Inside: `kul.yml` and the sibling `*.kul` files, same names as the language requires.

The shelf **is** the root listing (`for await` of `directoryHandle.entries()`), optionally plus a sidecar file if the app wants metadata the directory names do not carry. ([MDN, OPFS listing](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system))

Reads return real `File` objects (`getFile()`). Writes are per file (`createWritable` or worker `createSyncAccessHandle`). There is no API that commits `kul.yml` and three `.kul` files as one atomic unit.

Handles are structured-cloneable into IndexedDB, so a hybrid (OPFS bytes + IDB shelf index) is expressible. Eviction and “Clear site data” still wipe **both** bottles together. ([MDN, File System API](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API); [MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria))

## Export (download the bytes)

Export is not a property of the vault primitive. Both end at the same download APIs:

1. Read each file’s bytes (IDB: record `body`; OPFS: `fileHandle.getFile()` → `Blob`/`File`).
2. Package: one `Blob` per file, or one `Blob` the app concatenates (a zip is an application format, not a platform primitive). [`new Blob(parts, { type })`](https://developer.mozilla.org/en-US/docs/Web/API/Blob/Blob) concatenates strings / buffers / other Blobs.
3. [`URL.createObjectURL(blob)`](https://developer.mozilla.org/en-US/docs/Web/API/URL/createObjectURL_static) plus [`<a download="filename">`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/a#download). `download` works for same-origin, `blob:`, and `data:` URLs.

OPFS already hands back `File` objects (name + bytes), so a per-file download is a thin wrap. IndexedDB must restore names from record keys.

`showSaveFilePicker` / `showDirectoryPicker` can write a user-visible copy. That is File System Access, Chromium-only, permissioned, and **outside** the vault. Firefox and Safari do not implement those pickers. ([WICG File System Access](https://wicg.github.io/file-system-access/); [caniuse native-filesystem-api](https://caniuse.com/native-filesystem-api))

A full-shelf export is “iterate the shelf, read every project’s files, build Blobs, trigger downloads” on either primitive.

## Size posture

This product stores **small text**, not 10k-person graphs as blobs.

In-repo examples (`examples/` as of this note):

- Every `kul.yml` is 11 bytes (`kul: "0.1"`).
- Largest single `.kul` (family-across-a-century) is 6 596 bytes.
- The multi-file project is 11 + 805 + 745 + 414 = 1 975 bytes of source.
- All example `.kul` + `kul.yml` files together are 22 904 bytes.

A shelf of “many families” at that scale is kilobytes to low megabytes. Even 1 000 such Kul projects is on the order of tens of megabytes — far below Firefox’s 10 GiB best-effort group cap or Chromium/WebKit’s ~60%-of-disk origin cap. ([MDN, quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria))

Quota and “can it hold large binaries?” do **not** distinguish the two primitives for this vault. The distinguishing facts are API shape (records + transactions vs directories + files), write atomicity, and Safari’s `createWritable` gap.

## Leftover tradeoff for [Decide: vault mechanism and durability](https://github.com/YashBhalodi/kul/issues/335)

Issue #335 has to choose a vault mechanism and a durability story. This research does not choose. The leftover is:

**Records-and-a-transaction versus files-that-look-like-a-Kul-project**, on a shared origin bucket that is best-effort unless `persist()` is granted, and that **dies if the user clears site data** either way.

Concretely, #335 still has to weigh:

1. **Shape.** IndexedDB stores a shelf as indexed records and a Kul project as a set of values; OPFS can store the shelf as directories and each Kul project as `kul.yml` + sibling `*.kul`. Export to a download is the same Blob + `download` path from both.

2. **Atomic save.** IndexedDB can commit a project’s files in one transaction (and can take a `"strict"` durability hint). OPFS can atomically replace **one** file (`createWritable` close / `flush`); it cannot atomically replace a multi-file Kul project. A crash mid-save is a defined IDB abort; on OPFS it is a torn directory.

3. **Safari / Firefox / Chrome.** IndexedDB is everywhere this app will run. OPFS root exists from Safari 15.2 and Firefox 111, but **Safari cannot `createWritable` until 26**. An OPFS vault that must write on older Safari needs a dedicated-worker `createSyncAccessHandle` path. Firefox/Safari also lack user-visible file pickers; those must not be mistaken for the vault or for export.

4. **Durability vs user clear.** `navigator.storage.persist()` is the only standard knob against automatic eviction (Firefox prompt; Safari/Chromium heuristics). It is origin-wide. Safari’s seven-day ITP eviction makes that knob material for a vault that is the only copy of a family’s source. **Neither primitive survives “Clear site data.”** If #335 needs survival across that, the answer is not a different origin-private API — it is a copy the user owns (download, or a later user-visible-folder decision).

5. **Size.** Not a selector. Kul projects are small text; many families still sit far under origin quota.

A hybrid (OPFS files + IDB shelf / handles) is spec-legal and still one origin bucket: persist, eviction, and site-data clear treat it as one pile.
