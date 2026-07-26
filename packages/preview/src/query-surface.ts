// The query surface — the composition root for query chrome.
//
// It owns the {@link SelectionStore} every later slice needs, the paint that
// shows it, the details panel it feeds, the panel-driven walk, the Explore-kin
// list and the paint an answered kin set puts on the tree, the hover lens that
// reads ties off the selection, the attribute filter bar, the editor-sync
// suspension that keeps the two meanings of "highlighted" from co-painting
// (#276 point 9), and the mode boundary that ends all of it on a render
// (ADR-0035, ADR-0046).
//
// `mount.ts` reaches for seven members and no more: `handleCanvasClick` (a
// click inside the rendered SVG), `handleCanvasHover` (the pointer moved over
// it), `endQueryMode` (a render swapped the SVG, which means the document
// changed), `syncHighlight` (an inbound editor highlight, which becomes the
// handle's `highlightEntity`), `refresh` (the locale toggle changed language),
// `occupiedBox` (the ghost badge's jump must steer around the panel) and
// `dispose`. Everything else about the selection is this module's business,
// which is what keeps the chrome's composition in one readable place rather
// than spread across the mount. The remaining members are the two independent
// exits `endQueryMode` composes — `clearSelection` and `clearFilter` — plus
// `selection`, `setSyncSuspended` and `setDimExemption`; `mount.ts` calls none
// of them.

import type {
    DetailLookupResult,
    DetailTarget,
    EntityDetail,
    Member,
    Query,
    QueryEnvelope,
    QueryResult,
    ResolveResult,
} from "./engine-wire.js";
import { isQueryOk } from "./engine-wire.js";
import { type DetailPanel, createDetailPanel } from "./detail-panel.js";
import { createDimRegistry } from "./dim.js";
import type { FilterScope } from "./filter.js";
import { type FilterBar, createFilterBar } from "./filter-bar.js";
import { type HoverLens, createHoverLens } from "./hover-lens.js";
import {
    type HighlightPanZoom,
    type ScreenBox,
    canonicalCardFor,
    panToElement,
} from "./highlight.js";
import type { KinListState } from "./kin-list.js";
import { emptyKinMessage } from "./kin-list.js";
import { paintKinResults } from "./kin-paint.js";
import { KIN_SETS, kinQuery, kinSetById } from "./kin-sets.js";
import type { LocaleController } from "./locale.js";
import {
    type EntitySelection,
    type SelectionStore,
    createSelectionStore,
    paintSelection,
    selectionAnchorPerson,
    selectionAtElement,
} from "./selection.js";
import { type Toaster, createToaster } from "./toast.js";
import type { EntityRef, HostAdapter } from "./types.js";

/** Text of the transient hint shown while editor sync is suspended. */
export const SYNC_SUSPENDED_HINT = "Editor sync paused · Esc to resume";

/**
 * The hover lens's handle in the dim registry's exemption map. The lens itself
 * does not know the registry exists — it publishes a trace through `onTrace` —
 * so the key belongs to the composition that wires the two together.
 */
export const LENS_DIM_EXEMPTION = "lens-trace";

/**
 * Why editor sync is suspended. Two reasons exist — an entity selection
 * registers `"selection"`, an active attribute filter registers `"filter"` —
 * and sync resumes only when the last one lifts. A later surface with a reason
 * of its own registers it through {@link QuerySurface.setSyncSuspended} rather
 * than by widening any condition here (#276 point 9).
 */
export type SyncSuspensionReason = string;

export interface QuerySurfaceOptions {
    /** The rendered-SVG host (`#root`). Selection paint lands inside it. */
    root: HTMLElement;
    /**
     * The float layer's edge dock (`#kul-region-float-dock`) — the details
     * panel's home (ADR-0038, ADR-0042). The dock owns the edge and the inset.
     */
    floatDock: HTMLElement;
    /**
     * The float layer itself (`#kul-region-float`) — the *other* placement the
     * layer carries (ADR-0042), for chrome anchored to a card's screen box
     * rather than to a stage edge. The hover lens's pill lives here.
     */
    floatLayer: HTMLElement;
    /**
     * The flow region (`#kul-region-flow`) — the filter bar's home. A filter is
     * a project-scoped question, so its chrome stands above the canvas in the
     * document flow rather than over it, and is live with nothing selected
     * (#277, ADR-0038). A host without one gets no filter bar.
     */
    flowRegion: HTMLElement | null;
    /** Transient-notification region — the sync hint's home. */
    notifyRegion: HTMLElement | null;
    adapter: HostAdapter;
    /** The batched detail lookup, as the mounted preview exposes it (ADR-0037). */
    lookup(
        targets: DetailTarget[],
    ): Promise<QueryEnvelope<DetailLookupResult> | null>;
    /**
     * One kin-set query, as the mounted preview exposes it. The surface builds
     * the {@link Query} value (ADR-0025's one contract artifact) and the engine
     * evaluates it; nothing here decides membership.
     */
    runKinQuery(query: Query): Promise<QueryEnvelope<QueryResult> | null>;
    /**
     * Evaluate any {@link Query} — the filter bar's two certainty questions go
     * through it. Separate from {@link QuerySurfaceOptions.runKinQuery} because
     * they are two verbs on the WASM surface naming two questions, even though
     * `kul-core` evaluates both on one path (`engine.ts` says so at length).
     */
    runQuery(query: Query): Promise<QueryEnvelope<QueryResult> | null>;
    /** Two-anchor relationship resolution, ego first — what the lens asks (ADR-0028). */
    resolve(
        egoId: string,
        alterId: string,
    ): Promise<QueryEnvelope<ResolveResult> | null>;
    /**
     * The reader's language, as **one** source of truth for every phrased word
     * in the query chrome (ADR-0043).
     *
     * Two consumption idioms hang off it, chosen by lifetime, and the surface
     * hands each consumer the one that fits. Chrome that redraws wholesale — the
     * details panel and its kin list — gets `locale.pack()` read at *draw* time,
     * so ADR-0041's toggle subscription driving `refresh()` is the whole
     * re-phrase wire. Chrome that is a persistent element which must re-phrase
     * *in place* — #302's lens pill — uses `locale.bind`, which owns that
     * element's text, `lang` and phrase metadata for as long as it is up.
     */
    locale: LocaleController;
    getPanZoom(): HighlightPanZoom | null;
    /** Paint + centre an editor-sync highlight. Called only while sync is live. */
    applySyncHighlight(ref: EntityRef | null): void;
}

export interface QuerySurface {
    /**
     * The selection seam. Later slices read `current`, `subscribe` to changes,
     * and `select` to move it; they never reach past it to the DOM.
     */
    readonly selection: SelectionStore;
    /** A click landed inside the rendered SVG: an entity moves it, the canvas clears it. */
    handleCanvasClick(target: Element | null): void;
    /**
     * The pointer moved over the rendered SVG. Cheap on every call; only a
     * settled pointer costs a query (#302).
     *
     * **Two consumers, fanned out here.** The hover lens whispers a
     * relationship while a person is selected; the filter bar whispers a
     * can't-say reason under an unjudgeable card. Both dock a tag under the
     * card the pointer is on, so they must never be up together — and the
     * placement question ADR-0044 handed to #303 is answered by the fan-out
     * itself: while a person is selected the reason stands down, and is *told*
     * to (passed `null`) rather than merely not called, so a reason already
     * open comes off screen. One gesture, one answer (ADR-0045).
     *
     * For a pointer *leaving* the canvas, pass the element it moved onto:
     * either consumer keeps reading when that element is its own tag, so
     * hovering a term or a reason does not dismiss what is being hovered.
     */
    handleCanvasHover(target: Element | null): void;
    /**
     * An inbound editor-sync highlight. Painted only while sync is live; while
     * it is suspended the ref is *held* and replayed the moment the last
     * suspension reason lifts, so resuming does not wait for the reader to
     * move the cursor again.
     */
    syncHighlight(ref: EntityRef | null): void;
    /**
     * **An edit ends query mode.** The one post-render hook `mount.ts` gives
     * query chrome, and what it does is let go: the selection, its panel, any
     * kin paint, the hover lens and the filter all come down, the tree returns
     * to its plain state, and editor sync resumes on its own because the last
     * suspension reason lifts with them (ADR-0035, ADR-0046).
     *
     * It is composed from exactly the two independent exits —
     * {@link QuerySurface.clearSelection} and {@link QuerySurface.clearFilter}
     * — and nothing else. The lens is not a third: it subscribes to the
     * selection seam itself, so it comes down when the selection does, on the
     * same path Esc and a canvas click already take.
     *
     * **Why a render is the trigger.** The rule is about *edits*, and a render
     * is how an edit reaches the webview: the preview re-renders from
     * `onDidChangeTextDocument`, debounced at 300 ms. Two other render paths
     * exist and neither can surprise a reader — see ADR-0046, which checked
     * them rather than trusting the claim.
     *
     * **The accepted cost** is that a one-character typo fix costs the reader
     * their selection, their painted kin set and their filter. Refetching the
     * bundle per render was measured as affordable (≈13 ms per debounced
     * render) and rejected in favour of this: querying and authoring are
     * separate activities, and no query artefact outlives the source it was
     * computed from.
     *
     * Deliberately not `repaintSelection()` and no longer `repaintQueryChrome`:
     * there is one hook, and its name states the rule rather than the gesture
     * that used to be underneath it.
     */
    endQueryMode(): void;
    /**
     * Drop the selection, its panel and its paint. Esc, a canvas click and
     * {@link QuerySurface.endQueryMode} all call it.
     *
     * It is deliberately **not** named `clear()`: it clears one surface, and
     * `selection.clear()` is a documented no-op when nothing is selected, so a
     * general-sounding name would silently encode "query mode == a selection
     * exists", which the filter makes false: it can be active on its own, and
     * {@link QuerySurface.clearFilter} is its own separate exit (ADR-0042).
     */
    clearSelection(): void;
    /**
     * Drop the filter: its sentence, its paint, its dim, its can't-say
     * exemption and its sync suspension. The other half of what
     * {@link QuerySurface.endQueryMode} is composed from, and separate from
     * {@link QuerySurface.clearSelection} because a filter and a selection are
     * independent — either can exist without the other. A no-op when the host
     * supplied no flow region.
     */
    clearFilter(): void;
    /**
     * Register or lift one reason to suspend editor sync. The selection
     * registers `"selection"` and the filter bar `"filter"`, both from inside
     * this module; a later surface with its own reason calls this rather than
     * widening a condition, and the hint follows the reason set rather than the
     * selection.
     */
    setSyncSuspended(reason: SyncSuspensionReason, suspended: boolean): void;
    /**
     * Redraw the open panel from the answer it is already showing — no engine
     * call, no selection change. The lever for anything that changes how the
     * *same* detail reads: ADR-0041's locale toggle subscribes to it, and the
     * kin list's phrased term gloss re-renders through it. A no-op when no
     * panel is open.
     *
     * It rebuilds the panel element, so the panel's scroll position resets.
     * The kin list scrolls inside its own box rather than the panel's, which
     * keeps that reset to the panel's own (short) content.
     */
    refresh(): void;
    /**
     * Publish one source's exempt persons — lifted from every dim source —
     * or withdraw them with `null`.
     *
     * The hover lens traces the persons that justify a relationship, and with a
     * kin set painted those persons are almost always outside the answer, so the
     * sky trace would render under the kin dim's alpha. The lens is what the
     * reader is doing now and the dim is what they did a moment ago, so the lens
     * wins (ADR-0043). The rule lives on the dim registry rather than in any one
     * paint, so a later source inherits it instead of re-deciding it.
     *
     * **Keyed, because there are two.** ADR-0043 held the exemption in a single
     * slot on the reasoning that a pointer is in one place, and named keying as
     * the fix if a second holder appeared. The filter's can't-say set is that
     * holder: it is standing rather than pointer-driven, and it must survive the
     * lens publishing and withdrawing a trace over the top of it (ADR-0045).
     */
    setDimExemption(
        source: string,
        personIds: Iterable<string> | null,
    ): void;
    /** The panel's viewport box while it is open — the region a pan must avoid. */
    occupiedBox(): ScreenBox | null;
    dispose(): void;
}

export function createQuerySurface(options: QuerySurfaceOptions): QuerySurface {
    const {
        root,
        floatDock,
        floatLayer,
        flowRegion,
        notifyRegion,
        adapter,
        lookup,
        runKinQuery,
        runQuery,
        resolve,
        locale,
        getPanZoom,
        applySyncHighlight,
    } = options;

    const selection = createSelectionStore();
    let inFlightPan: { cancel(): void } | null = null;
    // Every selection change invalidates any answer still in flight, so a slow
    // lookup can never open a panel about an entity that is no longer selected.
    let generation = 0;
    // The last thing the editor asked us to highlight. Held while sync is
    // suspended and replayed the moment it resumes.
    let heldSyncRef: EntityRef | null = null;
    // The answer the open panel is showing, kept so it can be redrawn without
    // asking the engine again.
    let shownDetail: EntityDetail | null = null;
    let syncHint: HTMLElement | null = null;
    const suspensions = new Set<SyncSuspensionReason>();
    const toaster: Toaster = createToaster(notifyRegion);

    // --- Explore kin ------------------------------------------------------
    //
    // The list is a property of the *reader*, not of the selection: once opened
    // it stays open as the selection walks (#276's resolution), so `kinOpen`
    // outlives every anchor. Everything else here is per anchor and is dropped
    // the moment the selection moves, because an answer about one person is not
    // an answer about the next.
    let kinOpen = false;
    let kinAnchorId: string | null = null;
    let kinCounts = new Map<string, number>();
    let kinActiveSetId: string | null = null;
    let kinActiveMembers: ReadonlyArray<Member> = [];
    // The count sweep and a row's paint are two independent questions on two
    // independent gestures, and **each guards its own answers**. One shared
    // counter would mean painting a row invalidated the sweep that was still in
    // flight — which is the designed path, not an edge case, because rows are
    // deliberately clickable before counts land. The sweep is therefore guarded
    // on the anchor it asked about (the only thing that can make its answer
    // wrong), and the paint on a counter of its own (a second row click must
    // discard the first row's answer even though the anchor has not moved).
    let kinCountsFor: string | null = null;
    let kinPaintGeneration = 0;
    const dim = createDimRegistry();

    const panel: DetailPanel = createDetailPanel({
        host: floatDock,
        // The panel is the wholesale-redraw idiom: it takes the *pack*, read at
        // draw time, and never the controller. A widget that held the
        // controller could rebind ad hoc and the panel would then have two ways
        // to phrase with two redraw semantics (ADR-0043).
        pack: () => locale.pack(),
        onSelectPerson: walkToPerson,
        onReveal(id) {
            adapter.onRevealRequest({ kind: "entity", id });
        },
        onToggleKin: toggleKinList,
        onSelectKinSet: runKinSet,
    });

    function kinState(): KinListState | null {
        if (kinAnchorId === null) {
            return null;
        }
        return {
            anchorId: kinAnchorId,
            open: kinOpen,
            counts: kinCounts,
            activeSetId: kinActiveSetId,
            activeMembers: kinActiveMembers,
        };
    }

    // The lens subscribes to the selection itself, so it needs no help from
    // here to know its ego moved. `locale.bind` rather than `locale.pack()` is
    // the idiom for a persistent element that must re-phrase in place, which is
    // the split ADR-0043 named: the panel redraws wholesale, the pill does not.
    const lens: HoverLens = createHoverLens({
        root,
        layer: floatLayer,
        selection,
        resolve,
        bindPhrase: (element, descriptor) => locale.bind(element, descriptor),
        onTrace: (personIds) => setDimExemption(LENS_DIM_EXEMPTION, personIds),
    });

    /**
     * Publish one source's exemption and redraw. Two sources hold one at a
     * time — the lens's live trace and the filter's standing can't-say set —
     * and the registry unions them (ADR-0045).
     */
    function setDimExemption(
        source: string,
        personIds: Iterable<string> | null,
    ): void {
        dim.exempt(source, personIds);
        dim.apply(root);
    }

    /**
     * The painted kin set as a filter scope, or `null` when none is painted.
     *
     * This is the composition #277 asked for: switching the scope chip switches
     * the `Query`'s **source**, so `kinOf` + `where` is evaluated as one query
     * and "which of Giuseppe's descendants were born before 1950" is one
     * question rather than two. The population it carries is the answer already
     * on screen — the same members the teal was painted from — so the tally's
     * denominator and the paint can never disagree, and no third call is made
     * for a set the surface is already holding.
     *
     * It is a *function* the bar calls on every draw rather than a value handed
     * over once, because a kin set is painted and dropped without the filter
     * being involved.
     */
    function kinScope(): FilterScope | null {
        const set = kinActiveSetId === null ? null : kinSetById(kinActiveSetId);
        if (!set || kinAnchorId === null) {
            return null;
        }
        return {
            id: set.id,
            // The reader's words for the painted answer: the same set label the
            // row carries, possessed by the same anchor name the empty-set
            // toast uses, so one painted set is named one way on screen.
            label: `${anchorName()}'s ${set.label.toLowerCase()}`,
            source: kinQuery(set, kinAnchorId, "members").source,
            personIds: new Set(kinActiveMembers.map((member) => member.personId)),
        };
    }

    // The filter bar is standing chrome: a filter is project-scoped, so it is
    // live before any selection. It is composed here rather than in the mount
    // because everything it needs is here — the dim registry it publishes into,
    // the sync-suspension reason set, the painted kin set it can scope to, and
    // the one post-render hook (ADR-0045).
    const filterBar: FilterBar | null = flowRegion
        ? createFilterBar({
              host: flowRegion,
              root,
              floatLayer,
              dim,
              run: runQuery,
              lookup,
              kinScope,
              setSyncSuspended(suspended) {
                  setSyncSuspended("filter", suspended);
              },
          })
        : null;

    function closePanel(): void {
        shownDetail = null;
        panel.close();
    }

    function showPanel(detail: EntityDetail): void {
        shownDetail = detail;
        panel.show(detail, kinState());
    }

    /** Redraw the open panel in place — the one way the kin list changes. */
    function redrawPanel(): void {
        if (shownDetail) {
            panel.show(shownDetail, kinState());
        }
    }

    function paintKin(): void {
        if (kinAnchorId === null || kinActiveSetId === null) {
            paintKinResults(root, null, dim);
            return;
        }
        paintKinResults(
            root,
            {
                anchorId: kinAnchorId,
                personIds: kinActiveMembers.map((member) => member.personId),
            },
            dim,
        );
    }

    /** Drop the painted answer, its paint and its toast. Keeps the counts. */
    function dropKinPaint(): void {
        kinPaintGeneration += 1;
        kinActiveSetId = null;
        kinActiveMembers = [];
        toaster.clear();
        paintKinResults(root, null, dim);
    }

    /** Drop every kin answer for the outgoing anchor. Keeps `kinOpen`. */
    function resetKin(anchorId: string | null): void {
        dropKinPaint();
        kinAnchorId = anchorId;
        kinCounts = new Map();
        kinCountsFor = null;
    }

    /**
     * Open or close the list.
     *
     * **Closing drops the paint.** The list is the only thing on screen that
     * says which question the teal answers; leaving three cards lit under a
     * closed list is an answer with its question hidden. It is also the reader's
     * way out of a paint without giving up the selection — as is re-clicking the
     * active row (ADR-0043).
     */
    function toggleKinList(): void {
        kinOpen = !kinOpen;
        if (!kinOpen) {
            dropKinPaint();
        }
        redrawPanel();
        if (kinOpen) {
            void loadKinCounts();
        }
    }

    /**
     * Ask the engine how big each kin set is — one `count`-projected {@link Query}
     * per row, evaluated by the engine.
     *
     * Every number the list shows is therefore the engine's own count of the
     * pinned Query value, never a length the chrome derived. The alternative —
     * one unclassified sweep, bucketed here by descriptor — would have been one
     * call instead of thirteen and was rejected on exactly that ground: it puts
     * kin-set membership in the consumer, which is the line ADR-0025 draws
     * (ADR-0043 records the cost this leaves standing).
     *
     * The sweep is memoized per anchor: `kinCountsFor` is claimed *before* the
     * first `await` so a second call cannot start a duplicate sweep, and the
     * answers are guarded on the anchor rather than on a counter, so anything
     * the reader does in the ≈13-rows-worth of latency — clicking a row, above
     * all — leaves the sweep intact.
     *
     * **The claim is released the moment the sweep does not fully answer.** A
     * project that fails its checks yields the envelope's error arm (ADR-0009),
     * which is the ordinary state of a document mid-edit, and a claim held
     * across that failure would leave every row reading `·` for as long as the
     * person stayed selected — surviving the fix and the re-render that follows
     * it. Whatever *did* answer is still shown: those numbers are the engine's,
     * and a row the sweep could not answer shows the same placeholder it shows
     * before any answer lands.
     */
    async function loadKinCounts(): Promise<void> {
        const anchorId = kinAnchorId;
        if (anchorId === null || kinCountsFor === anchorId) {
            return;
        }
        kinCountsFor = anchorId;
        /**
         * Give the claim back, unless a newer sweep has already taken it. The
         * anchor moving runs `resetKin`, which nulls it — releasing then would
         * clobber the sweep that moved in behind us.
         */
        function releaseClaim(): void {
            if (kinCountsFor === anchorId) {
                kinCountsFor = null;
            }
        }
        let answers;
        try {
            answers = await Promise.all(
                KIN_SETS.map(async (set) => ({
                    id: set.id,
                    envelope: await runKinQuery(kinQuery(set, anchorId, "count")),
                })),
            );
        } catch {
            // A rejecting `runKinQuery` is not the mounted path — `mount.ts`
            // turns a throw into the error popover and a `null` answer — but
            // the surface must not leave a claim behind for an injected one,
            // and must not raise out of a `void`-ed call either.
            releaseClaim();
            return;
        }
        if (kinAnchorId !== anchorId) {
            return;
        }
        const counts = new Map<string, number>();
        for (const { id, envelope } of answers) {
            if (!envelope || !isQueryOk(envelope)) {
                continue;
            }
            const result = envelope.result;
            if (result.kind === "count") {
                counts.set(id, result.count);
            }
        }
        if (counts.size < KIN_SETS.length) {
            releaseClaim();
        }
        kinCounts = counts;
        redrawPanel();
    }

    /**
     * Paint one kin set on the tree, or let go of the one already painted.
     *
     * The same Query value the count came from, projected to `members`: the ids
     * bind to every card their person owns (ADR-0034) and the descriptors that
     * ride along phrase the row's gloss with no lookup of their own. An empty
     * answer paints nothing and says so in a quiet toast — it is an answer, not
     * an error, so it gets no popover and no modal.
     *
     * Clicking the row that is already painted **clears it**, so the reader can
     * drop an answer without dropping the person it was about.
     */
    async function runKinSet(setId: string): Promise<void> {
        const set = kinSetById(setId);
        const anchorId = kinAnchorId;
        if (!set || anchorId === null) {
            return;
        }
        if (kinActiveSetId === set.id) {
            dropKinPaint();
            redrawPanel();
            return;
        }
        kinPaintGeneration += 1;
        const mine = kinPaintGeneration;
        const envelope = await runKinQuery(kinQuery(set, anchorId, "members"));
        if (mine !== kinPaintGeneration || kinAnchorId !== anchorId) {
            return;
        }
        if (!envelope || !isQueryOk(envelope)) {
            return;
        }
        const result = envelope.result;
        if (result.kind !== "members") {
            return;
        }
        kinActiveSetId = set.id;
        kinActiveMembers = result.members;
        paintKin();
        redrawPanel();
        if (result.members.length === 0) {
            toaster.show(emptyKinMessage(set.label, anchorName()));
        } else {
            toaster.clear();
        }
    }

    /**
     * The anchor as the reader knows it. Taken from the detail answer the panel
     * is already holding (ADR-0037) — no second call and no second provenance
     * path. The id is the honest stand-in if no detail arrived.
     */
    function anchorName(): string {
        return shownDetail?.kind === "person"
            ? shownDetail.person.name
            : (kinAnchorId ?? "");
    }

    function cancelPan(): void {
        if (inFlightPan) {
            inFlightPan.cancel();
            inFlightPan = null;
        }
    }

    function occupiedBox(): ScreenBox | null {
        const box = panel.box();
        if (!box) {
            return null;
        }
        // svg-pan-zoom's viewport is the rendered `<svg>`; translate the
        // panel's viewport box into that element's local pixels so the two
        // coordinate systems agree.
        const host = (root.querySelector("svg") ?? root) as Element;
        if (typeof host.getBoundingClientRect !== "function") {
            return null;
        }
        const base = host.getBoundingClientRect();
        return {
            left: box.left - base.left,
            right: box.right - base.left,
            top: box.top - base.top,
            bottom: box.bottom - base.top,
        };
    }

    /**
     * The panel's navigation path: move the selection onto a person named in
     * it, then centre that person's canonical card in the visible region. The
     * pan is not a flourish — the panel is how off-screen ancestors get
     * discovered, and a selection the reader cannot see is a dead end.
     */
    function walkToPerson(id: string): void {
        selection.select({ kind: "person", id });
        const card = canonicalCardFor(root, id);
        if (!card) {
            return;
        }
        cancelPan();
        inFlightPan = panToElement(getPanZoom(), card, occupiedBox());
    }

    function reconcileSyncHint(): void {
        const visible = suspensions.size > 0;
        if (!notifyRegion) {
            return;
        }
        if (!visible) {
            if (syncHint) {
                syncHint.remove();
                syncHint = null;
            }
            return;
        }
        if (syncHint) {
            return;
        }
        const hint = document.createElement("div");
        hint.className = "kul-sync-hint";
        hint.textContent = SYNC_SUSPENDED_HINT;
        notifyRegion.appendChild(hint);
        syncHint = hint;
    }

    async function loadPanel(
        current: EntitySelection,
        mine: number,
    ): Promise<void> {
        const envelope = await lookup([current]);
        if (mine !== generation) {
            return;
        }
        if (!envelope || !isQueryOk(envelope)) {
            closePanel();
            return;
        }
        const detail = envelope.result[0];
        if (!detail) {
            // A target naming no entity is a complete answer (ADR-0037), and an
            // empty panel would be a claim the engine did not make.
            closePanel();
            return;
        }
        showPanel(detail);
    }

    function setSyncSuspended(
        reason: SyncSuspensionReason,
        suspended: boolean,
    ): void {
        const was = suspensions.size > 0;
        if (suspended) {
            suspensions.add(reason);
        } else {
            suspensions.delete(reason);
        }
        const now = suspensions.size > 0;
        if (was === now) {
            return;
        }
        if (now) {
            // The takeover half of "the two meanings of highlighted never
            // co-paint": an editor-sync highlight is usually already on screen
            // when the reader clicks, because the cursor is usually on some
            // entity. Suspending has to *strip* it, not merely stop painting
            // new ones. `heldSyncRef` is untouched, so Esc still replays it.
            applySyncHighlight(null);
        } else {
            applySyncHighlight(heldSyncRef);
        }
        reconcileSyncHint();
    }

    selection.subscribe((current) => {
        generation += 1;
        paintSelection(root, current);
        // A kin answer is about one anchor. Moving the selection drops the
        // answer and its paint; the *list* survives, open, which is what "the
        // list stays open as the selection walks" means. An edge selection has
        // no anchor at all — `selectionAnchorPerson` answers null — so it gets
        // no list, which is the waypoint rule spelled in paint.
        resetKin(selectionAnchorPerson(current));
        if (!current) {
            closePanel();
            cancelPan();
        }
        // Tear down before resuming, so the replayed sync highlight is the last
        // thing to paint and the last thing to pan.
        setSyncSuspended("selection", current !== null);
        if (current) {
            void loadPanel(current, generation);
            if (kinOpen) {
                void loadKinCounts();
            }
        }
    });

    function clearSelection(): void {
        selection.clear();
    }

    function clearFilter(): void {
        filterBar?.reset();
    }

    /**
     * The mode boundary, as two calls. Both are idempotent and neither depends
     * on the other having run, so the order below is readability rather than
     * sequencing: `clearSelection` lifts the `"selection"` reason and
     * `clearFilter` the `"filter"` one, and whichever goes last is the one that
     * resumes editor sync.
     */
    function endQueryMode(): void {
        clearSelection();
        clearFilter();
    }

    /**
     * **Query mode has two exits, and they do the same thing.** An edit is one
     * (ADR-0046); Esc is the other, and it lifts whatever query state is
     * holding editor sync down — the selection, the filter, or both.
     *
     * Esc is not scoped to the selection, and the hint is why: the notify region
     * says "Editor sync paused · Esc to resume" for as long as *any* reason
     * holds, so a filter with no selection would otherwise put that sentence on
     * screen with no key behind it, leaving the reader to click ✕ on every chip
     * (ADR-0045). The guard is what keeps Esc from being swallowed when there is
     * no query state to end.
     */
    function onKeyDown(event: KeyboardEvent): void {
        if (event.key !== "Escape" || suspensions.size === 0) {
            return;
        }
        endQueryMode();
    }
    window.addEventListener("keydown", onKeyDown);

    return {
        selection,
        handleCanvasClick(target) {
            const next = selectionAtElement(target);
            if (next) {
                selection.select(next);
            } else {
                selection.clear();
            }
        },
        handleCanvasHover(target) {
            lens.handleHover(target);
            // A person selection is what arms the lens, and the lens's pill
            // docks under exactly the card a can't-say reason would. So the
            // reason stands down while one exists — and is told to, rather than
            // merely not called, so a reason already open comes off screen
            // (ADR-0045, answering the placement question ADR-0044 left to
            // this slice).
            const lensArmed = selectionAnchorPerson(selection.current) !== null;
            filterBar?.handleHover(lensArmed ? null : target);
        },
        syncHighlight(ref) {
            heldSyncRef = ref;
            if (suspensions.size > 0) {
                // Held: the query selection owns the paint, and the two
                // meanings of "highlighted" never co-paint.
                return;
            }
            applySyncHighlight(ref);
        },
        endQueryMode,
        clearSelection,
        clearFilter,
        setSyncSuspended,
        refresh: redrawPanel,
        setDimExemption,
        occupiedBox,
        dispose() {
            window.removeEventListener("keydown", onKeyDown);
            lens.dispose();
            // Drop the held ref first, before *anything* below can lift a
            // suspension reason. Disposing the filter lifts `"filter"`, and on
            // a filter-only suspension that is the last reason — which replays
            // the held highlight onto chrome being torn down unless the ref is
            // already gone.
            heldSyncRef = null;
            // Then the filter, before the dim is reset, so its source and its
            // exemption are withdrawn rather than left in a registry nobody
            // will apply again.
            filterBar?.dispose();
            // Clearing runs the normal teardown path — paint off, panel shut,
            // pan cancelled, suspension lifted. A disposed surface stops
            // listening, so a violet outline left behind would have nothing
            // left to clear it.
            selection.clear();
            cancelPan();
            closePanel();
            paintSelection(root, null);
            resetKin(null);
            dim.reset(root);
            toaster.dispose();
            suspensions.clear();
            reconcileSyncHint();
        },
    };
}
