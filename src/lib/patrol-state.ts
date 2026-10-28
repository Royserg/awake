/** Pure wire validation and per-overlay queue. Native Rust owns cursor execution and all windows. */
export type Id = string;
export type XY = {
    x: number;
    y: number;
};
export type Rect = XY & {
    width: number;
    height: number;
};
export type Mapping = {
    origin: XY;
    width: number;
    height: number;
};
export type Point = {
    pointId: Id;
    displayId: Id;
    x: number;
    y: number;
};
export type Display = {
    displayId: Id;
    name: string;
    nativeBounds: Rect;
    unit: 'macosPoint' | 'windowsPixel';
    scale: XY;
    rotationDegrees: number;
    mirrorOf: Id | null;
};
export type Pair = {
    routeRevision: Id;
    layoutRevision: Id;
};
export type EditReason = 'invalidDrop' | 'captureLost' | 'mutationFailed' | 'notResponding';
export type OverlayEdit = {
    overlayId: Id;
    displayId: Id;
    state: 'initializing' | 'ready' | 'editing' | 'incomplete' | 'unresponsive';
    pointIds: Id[];
    reason: EditReason | null;
};
export type Preparation = Pair & {
    preparationId: Id;
    requesterOverlayId: Id;
    requestId: Id;
    pendingOverlayIds: Id[];
    readyOverlayIds: Id[];
};
export type Setup = Pair & {
    setupId: Id;
    points: Point[];
    pauseMs: number;
    displays: Display[];
    invalidPoints: {
        pointId: Id;
        reason: 'displayMissing' | 'outsideDisplay' | 'invalidCoordinate';
    }[];
    routeValidation: {
        valid: boolean;
        errors: ('pointCount' | 'distinctPositions' | 'invalidPoints' | 'pause')[];
    };
    review: {
        required: boolean;
        confirmedFor: Pair | null;
    };
    overlays: OverlayEdit[];
    preparation: Preparation | null;
};
export const errorCodes = ['unsupportedPlatform', 'nativeUnavailable', 'permissionRequired', 'invalidPayload', 'invalidRoute', 'forbidden', 'staleSetup', 'staleSnapshot', 'staleAction', 'staleRun', 'stalePreparation', 'reviewRequired', 'alreadyRunning', 'busy', 'startCanceled', 'editIncomplete', 'editPreparationTimedOut', 'timerExpired', 'awakeActivationFailed', 'awakeDeactivationFailed', 'overlayFailed', 'cursorConstrained', 'stopPending', 'internalError'] as const;
export type ErrorCode = typeof errorCodes[number];
export type Failure = {
    code: ErrorCode;
    message: string;
};
export type StopReason = 'escape' | 'manualMovement' | 'mouseButton' | 'trayStop' | 'awakeOff' | 'timerExpired' | 'displayChanged' | 'sessionInactive' | 'secureInput' | 'observerLost' | 'availabilityExpired' | 'permissionLost' | 'cursorConstrained' | 'nativeFailure' | 'executionStalled' | 'appExit';
export type Snapshot = {
    stateVersion: Id;
    status: {
        phase: 'stopped' | 'editing' | 'starting' | 'running' | 'stopping' | 'reviewRequired';
        startStage: 'preparingEdits' | 'arming' | null;
        runId: Id | null;
        stopReason: StopReason | null;
        lastFailure: Failure | null;
    };
    awake: {
        active: boolean;
        timerRemainingMs: string | null;
    };
    availability: {
        state: 'checking' | 'ready' | 'unsupportedPlatform' | 'permissionRequired' | 'nativeUnavailable';
        message: string | null;
    };
    setup: Setup | null;
    caller: {
        overlayId: Id;
        displayId: Id;
        isControlPanel: boolean;
        mapping: Mapping;
        lastAcceptedActionSeq: string;
    } | null;
};
export type Receipt = {
    type: 'update';
    setupId: Id;
    overlayId: Id;
    actionSeq: Id;
} | {
    type: 'start';
    setupId: Id;
    overlayId: Id;
    requestId: Id;
    runId: Id;
};
export type Reply = {
    snapshot: Snapshot;
    receipt: Receipt | null;
};
export type PatrolError = Failure & {
    snapshot: Snapshot | null;
};
export type Action = {
    type: 'addPoint';
    u: number;
    v: number;
} | {
    type: 'movePoint';
    pointId: Id;
    u: number;
    v: number;
} | {
    type: 'removePoint';
    pointId: Id;
} | {
    type: 'setPause';
    pauseMs: number;
} | {
    type: 'confirmReview';
} | {
    type: 'reportEditState';
    state: 'ready' | 'editing' | 'incomplete';
    pointIds: Id[];
    reason: EditReason | null;
} | {
    type: 'ackStartPreparation';
    preparationId: Id;
    outcome: 'ready' | 'incomplete';
    drainedThrough: string;
};
export type UpdateInput = Pair & {
    setupId: Id;
    actionSeq: Id;
    action: Action;
};
export type StartInput = Pair & {
    setupId: Id;
    requestId: Id;
};
const max = 18446744073709551615n;
export function decimal(value: unknown, zero = false): string { if (typeof value !== 'string' || !/^(0|[1-9][0-9]{0,19})$/.test(value) || BigInt(value) > max || (!zero && value === '0'))
    throw new Error('Invalid decimal counter'); return value; }
export class CounterAllocator {
    value: bigint;
    constructor(initial = '0') { this.value = BigInt(decimal(initial, true)); }
    next(): Id { if (this.value === max)
        throw new Error('Counter exhausted; cancel and reopen setup'); return String(++this.value); }
}
type Obj = Record<string, unknown>;
function object(v: unknown, keys: string): Obj { if (!v || typeof v !== 'object' || Array.isArray(v))
    throw new Error('Expected object'); const o = v as Obj; const list = keys.split(' '); if (Object.keys(o).length !== list.length || list.some(k => !Object.hasOwn(o, k)))
    throw new Error('Invalid object fields'); return o; }
function text(v: unknown): string { if (typeof v !== 'string')
    throw new Error('Expected text'); return v; }
function bool(v: unknown): boolean { if (typeof v !== 'boolean')
    throw new Error('Expected boolean'); return v; }
function num(v: unknown): number { if (typeof v !== 'number' || !Number.isFinite(v))
    throw new Error('Expected finite number'); return Object.is(v, -0) ? 0 : v; }
function one<const T extends readonly string[]>(v: unknown, options: T): T[number] { const s = text(v); if (!options.includes(s))
    throw new Error('Invalid enum'); return s as T[number]; }
function nullable<T>(v: unknown, fn: (v: unknown) => T): T | null { return v === null ? null : fn(v); }
function array<T>(v: unknown, fn: (v: unknown) => T): T[] { if (!Array.isArray(v))
    throw new Error('Expected array'); return v.map(fn); }
function ids(v: unknown): Id[] { const result = array(v, v => decimal(v)); if (new Set(result).size !== result.length)
    throw new Error('Duplicate ID'); return result; }
function xy(v: unknown): XY { const o = object(v, 'x y'); return { x: num(o.x), y: num(o.y) }; }
function extent(v: unknown): Rect { const o = object(v, 'x y width height'); const r = { x: num(o.x), y: num(o.y), width: num(o.width), height: num(o.height) }; if (r.width <= 0 || r.height <= 0 || !Number.isFinite(r.x + r.width) || !Number.isFinite(r.y + r.height))
    throw new Error('Invalid rectangle'); return r; }
function mapping(v: unknown): Mapping { const o = object(v, 'origin width height'); const origin = xy(o.origin); const r = extent({ ...origin, width: o.width, height: o.height }); return { origin, width: r.width, height: r.height }; }
function pair(v: unknown): Pair { const o = object(v, 'routeRevision layoutRevision'); return { routeRevision: decimal(o.routeRevision), layoutRevision: decimal(o.layoutRevision) }; }
function failure(v: unknown): Failure { const o = object(v, 'code message'); return { code: one(o.code, errorCodes), message: text(o.message) }; }
const reasons = ['invalidDrop', 'captureLost', 'mutationFailed', 'notResponding'] as const;
function setup(v: unknown): Setup {
    const o = object(v, 'setupId routeRevision layoutRevision points pauseMs displays invalidPoints routeValidation review overlays preparation');
    const points = array(o.points, v => { const p = object(v, 'pointId displayId x y'); return { pointId: decimal(p.pointId), displayId: decimal(p.displayId), x: num(p.x), y: num(p.y) }; });
    const displays = array(o.displays, v => { const d = object(v, 'displayId name nativeBounds unit scale rotationDegrees mirrorOf'); const scale = xy(d.scale); if (scale.x <= 0 || scale.y <= 0)
        throw new Error('Invalid scale'); return { displayId: decimal(d.displayId), name: text(d.name), nativeBounds: extent(d.nativeBounds), unit: one(d.unit, ['macosPoint', 'windowsPixel'] as const), scale, rotationDegrees: num(d.rotationDegrees), mirrorOf: nullable(d.mirrorOf, v => decimal(v)) }; });
    const validation = object(o.routeValidation, 'valid errors');
    const review = object(o.review, 'required confirmedFor');
    const overlays = array(o.overlays, v => { const r = object(v, 'overlayId displayId state pointIds reason'); return { overlayId: decimal(r.overlayId), displayId: decimal(r.displayId), state: one(r.state, ['initializing', 'ready', 'editing', 'incomplete', 'unresponsive'] as const), pointIds: ids(r.pointIds), reason: nullable(r.reason, v => one(v, reasons)) }; });
    const preparation = nullable(o.preparation, v => { const p = object(v, 'preparationId requesterOverlayId requestId routeRevision layoutRevision pendingOverlayIds readyOverlayIds'); return { preparationId: decimal(p.preparationId), requesterOverlayId: decimal(p.requesterOverlayId), requestId: decimal(p.requestId), routeRevision: decimal(p.routeRevision), layoutRevision: decimal(p.layoutRevision), pendingOverlayIds: ids(p.pendingOverlayIds), readyOverlayIds: ids(p.readyOverlayIds) }; });
    const pauseMs = num(o.pauseMs);
    if (!Number.isInteger(pauseMs) || pauseMs < 1000 || pauseMs > 3600000 || pauseMs % 1000)
        throw new Error('Invalid pause');
    const result: Setup = { setupId: decimal(o.setupId), routeRevision: decimal(o.routeRevision), layoutRevision: decimal(o.layoutRevision), points, displays, pauseMs, overlays, preparation, invalidPoints: array(o.invalidPoints, v => { const p = object(v, 'pointId reason'); return { pointId: decimal(p.pointId), reason: one(p.reason, ['displayMissing', 'outsideDisplay', 'invalidCoordinate'] as const) }; }), routeValidation: { valid: bool(validation.valid), errors: array(validation.errors, v => one(v, ['pointCount', 'distinctPositions', 'invalidPoints', 'pause'] as const)) }, review: { required: bool(review.required), confirmedFor: nullable(review.confirmedFor, pair) } };
    const pointIds = new Set(points.map(p => p.pointId));
    const displayIds = new Set(displays.map(d => d.displayId));
    const overlayIds = new Set(overlays.map(r => r.overlayId));
    if (points.length > 256 || pointIds.size !== points.length || displayIds.size !== displays.length || overlayIds.size !== overlays.length || result.invalidPoints.some(p => !pointIds.has(p.pointId)) || overlays.some(r => !displayIds.has(r.displayId) || r.pointIds.some(id => !pointIds.has(id))) || displays.some(d => d.mirrorOf !== null && (!displayIds.has(d.mirrorOf) || d.mirrorOf === d.displayId)))
        throw new Error('Inconsistent setup IDs');
    if (result.routeValidation.valid !== (result.routeValidation.errors.length === 0))
        throw new Error('Inconsistent route validation');
    if (result.review.confirmedFor && (!result.review.required || !samePair(result, result.review.confirmedFor)))
        throw new Error('Obsolete review consent');
    if (preparation && (!overlayIds.has(preparation.requesterOverlayId) || preparation.pendingOverlayIds.some(id => !overlayIds.has(id)) || preparation.readyOverlayIds.some(id => !overlayIds.has(id) || preparation.pendingOverlayIds.includes(id)) || preparation.pendingOverlayIds.length + preparation.readyOverlayIds.length !== overlays.length))
        throw new Error('Inconsistent preparation');
    return result;
}
export function decodeSnapshot(v: unknown): Snapshot {
    const o = object(v, 'stateVersion status awake availability setup caller');
    const s = object(o.status, 'phase startStage runId stopReason lastFailure');
    const a = object(o.awake, 'active timerRemainingMs');
    const av = object(o.availability, 'state message');
    const result: Snapshot = { stateVersion: decimal(o.stateVersion), status: { phase: one(s.phase, ['stopped', 'editing', 'starting', 'running', 'stopping', 'reviewRequired'] as const), startStage: nullable(s.startStage, v => one(v, ['preparingEdits', 'arming'] as const)), runId: nullable(s.runId, v => decimal(v)), stopReason: nullable(s.stopReason, v => one(v, ['escape', 'manualMovement', 'mouseButton', 'trayStop', 'awakeOff', 'timerExpired', 'displayChanged', 'sessionInactive', 'secureInput', 'observerLost', 'availabilityExpired', 'permissionLost', 'cursorConstrained', 'nativeFailure', 'executionStalled', 'appExit'] as const)), lastFailure: nullable(s.lastFailure, failure) }, awake: { active: bool(a.active), timerRemainingMs: nullable(a.timerRemainingMs, v => decimal(v, true)) }, availability: { state: one(av.state, ['checking', 'ready', 'unsupportedPlatform', 'permissionRequired', 'nativeUnavailable'] as const), message: nullable(av.message, text) }, setup: nullable(o.setup, setup), caller: nullable(o.caller, v => { const c = object(v, 'overlayId displayId isControlPanel mapping lastAcceptedActionSeq'); return { overlayId: decimal(c.overlayId), displayId: decimal(c.displayId), isControlPanel: bool(c.isControlPanel), mapping: mapping(c.mapping), lastAcceptedActionSeq: decimal(c.lastAcceptedActionSeq, true) }; }) };
    const running = ['starting', 'running', 'stopping'].includes(result.status.phase);
    if (running !== (result.status.runId !== null) || (result.status.phase === 'starting') !== (result.status.startStage !== null) || (result.setup?.preparation !== null && result.setup?.preparation !== undefined && result.status.startStage !== 'preparingEdits'))
        throw new Error('Inconsistent patrol status');
    if (result.caller && (!result.setup || !result.setup.overlays.some(r => r.overlayId === result.caller?.overlayId && r.displayId === result.caller?.displayId)))
        throw new Error('Unregistered caller');
    return result;
}
export function decodeReply(v: unknown): Reply { const o = object(v, 'snapshot receipt'); const snapshot = decodeSnapshot(o.snapshot); const receipt: Receipt | null = nullable(o.receipt, v => { const type = (v as Obj)?.type; if (type === 'update') {
    const r = object(v, 'type setupId overlayId actionSeq');
    return { type, setupId: decimal(r.setupId), overlayId: decimal(r.overlayId), actionSeq: decimal(r.actionSeq) };
} const r = object(v, 'type setupId overlayId requestId runId'); if (r.type !== 'start')
    throw new Error('Invalid receipt'); return { type: 'start', setupId: decimal(r.setupId), overlayId: decimal(r.overlayId), requestId: decimal(r.requestId), runId: decimal(r.runId) }; }); return { snapshot, receipt }; }
export function decodeError(v: unknown): PatrolError { const o = object(v, 'code message snapshot'); return { code: one(o.code, errorCodes), message: text(o.message), snapshot: nullable(o.snapshot, decodeSnapshot) }; }
export function samePair(a: Pair, b: Pair): boolean { return a.routeRevision === b.routeRevision && a.layoutRevision === b.layoutRevision; }
export function samePoint(a: Point | undefined, b: Point | undefined): boolean { return !!a && !!b && a.pointId === b.pointId && a.displayId === b.displayId && a.x === b.x && a.y === b.y; }
export function reviewConfirmed(setup: Setup): boolean { return !setup.review.required || !!setup.review.confirmedFor && samePair(setup, setup.review.confirmedFor); }
export interface Transport {
    get(): Promise<Reply>;
    update(input: UpdateInput): Promise<Reply>;
    start(input: StartInput): Promise<Reply>;
    cancel(setupId: Id): Promise<Reply>;
    permissions(setupId: Id): Promise<Reply>;
    listen(changed: () => void): Promise<() => void>;
}
type Capture = {
    id: Id;
    mapping: string;
    layout: Id;
    expected: Point;
    released: boolean;
    failed: boolean;
};
type Job = {
    action: Action;
    frozen?: Pair;
    capture?: Capture;
    resolve: () => void;
    reject: (e: unknown) => void;
    preview?: boolean;
};
export class PatrolClient {
    snapshot: Snapshot | null = null;
    error: string | null = null;
    disposed = false;
    private transport: Transport;
    private changed: () => void;
    private setupId: Id | null = null;
    private overlayId: Id | null = null;
    private actions = new CounterAllocator();
    private requests = new CounterAllocator();
    private queue: Job[] = [];
    private inFlight = false;
    private fetching = false;
    private fetchAgain = false;
    private unlisten: (() => void) | null = null;
    private capture: Capture | null = null;
    private preview: {
        capture: Capture;
        u: number;
        v: number;
    } | null = null;
    private previewTimer: ReturnType<typeof setTimeout> | null = null;
    private failure: {
        pointIds: Id[];
        reason: EditReason;
    } | null = null;
    private reported = '';
    private acked = new Set<Id>();
    private preparingAck = false;
    private startPending = false;
    constructor(transport: Transport, changed: () => void = () => { }) { this.transport = transport; this.changed = changed; }
    async connect(): Promise<void> { const unlisten = await this.transport.listen(() => { void this.refresh(); }); if (this.disposed) {
        unlisten();
        return;
    } this.unlisten = unlisten; await this.refresh(); this.settle(); }
    accept(snapshot: Snapshot): void {
        if (this.disposed)
            return;
        if (this.snapshot && BigInt(snapshot.stateVersion) < BigInt(this.snapshot.stateVersion))
            return;
        if (this.setupId && snapshot.setup?.setupId !== this.setupId) {
            this.snapshot = snapshot;
            this.dispose();
            this.changed();
            return;
        }
        if (snapshot.caller && snapshot.setup) {
            if (!this.setupId) {
                this.setupId = snapshot.setup.setupId;
                this.overlayId = snapshot.caller.overlayId;
                this.actions = new CounterAllocator(snapshot.caller.lastAcceptedActionSeq);
            }
            if (snapshot.caller.overlayId !== this.overlayId) {
                this.dispose();
                return;
            }
        }
        this.snapshot = snapshot;
        if (this.capture && !this.inFlight && !this.capture.failed && !this.captureValid(this.capture))
            this.failCapture(this.capture, 'captureLost');
        if (this.failure) {
            const hadPoints = this.failure.pointIds.length > 0;
            this.failure.pointIds = this.failure.pointIds.filter(id => snapshot.setup?.points.some(p => p.pointId === id));
            if (hadPoints && !this.failure.pointIds.length && (!this.capture || this.capture.released))
                this.failure = null;
        }
        this.changed();
        this.settle();
    }
    async refresh(): Promise<void> { if (this.disposed)
        return; if (this.fetching) {
        this.fetchAgain = true;
        return;
    } this.fetching = true; do {
        this.fetchAgain = false;
        try {
            this.accept((await this.transport.get()).snapshot);
        }
        catch (e) {
            this.showError(e);
        }
    } while (this.fetchAgain && !this.disposed); this.fetching = false; }
    private showError(e: unknown): void { let message = e instanceof Error ? e.message : 'The edit could not be completed'; try {
        const error = decodeError(e);
        message = error.message;
        if (error.snapshot)
            this.accept(error.snapshot);
    }
    catch { /* A transport error has no authoritative state. */ } this.error = message; this.changed(); }
    private current(): {
        setup: Setup;
        caller: NonNullable<Snapshot['caller']>;
    } { if (this.disposed || !this.snapshot?.setup || !this.snapshot.caller)
        throw new Error('This setup is closed'); return { setup: this.snapshot.setup, caller: this.snapshot.caller }; }
    get frozen(): boolean { return this.disposed || this.startPending || this.snapshot?.status.phase === 'starting' || this.snapshot?.status.phase === 'stopping' || !!this.snapshot?.setup?.preparation; }
    private enqueue(action: Action, frozen?: Pair, capture?: Capture): Promise<void> { if (this.disposed)
        return Promise.reject(new Error('Setup is closed')); return new Promise((resolve, reject) => { this.queue.push({ action: structuredClone(action), frozen, capture, resolve, reject }); void this.pump(); }); }
    private captureValid(c: Capture): boolean { const s = this.snapshot; return !!s?.setup && !!s.caller && s.setup.layoutRevision === c.layout && JSON.stringify(s.caller.mapping) === c.mapping && samePoint(s.setup.points.find(p => p.pointId === c.id), c.expected); }
    private async pump(): Promise<void> {
        if (this.inFlight || this.disposed)
            return;
        const job = this.queue.shift();
        if (!job) {
            this.settle();
            return;
        }
        this.inFlight = true;
        try {
            let { setup } = this.current();
            if (job.capture && (job.capture.failed || !this.captureValid(job.capture)))
                throw new Error('That point changed while it was being dragged');
            const pair = job.frozen ?? { routeRevision: setup.routeRevision, layoutRevision: setup.layoutRevision };
            let input: UpdateInput = { setupId: setup.setupId, ...pair, actionSeq: this.actions.next(), action: job.action };
            let reply: Reply | null = null;
            for (let attempt = 0; attempt < 2; attempt++) {
                try {
                    reply = await this.transport.update(input);
                    break;
                }
                catch (e) {
                    let error: PatrolError;
                    try {
                        error = decodeError(e);
                    }
                    catch { // One identical replay safely recovers an accepted update whose reply was lost.
                        if (job.capture) {
                            await this.refresh();
                            throw e;
                        }
                        if (attempt === 0)
                            continue;
                        throw e;
                    }
                    if (error.snapshot)
                        this.accept(error.snapshot);
                    if (attempt !== 0 || error.code !== 'staleSnapshot' || job.action.type === 'confirmReview' || !['addPoint', 'movePoint', 'removePoint', 'setPause'].includes(job.action.type))
                        throw e;
                    ({ setup } = this.current());
                    if (setup.layoutRevision !== input.layoutRevision || job.capture && !this.captureValid(job.capture))
                        throw e;
                    input = { ...input, routeRevision: setup.routeRevision }; // Rejected sequence has no accepted identity yet.
                }
            }
            if (!reply)
                throw new Error('Missing edit result');
            const receipt = reply.receipt;
            if (!receipt || receipt.type !== 'update' || receipt.setupId !== input.setupId || receipt.overlayId !== this.overlayId || receipt.actionSeq !== input.actionSeq)
                throw new Error('Invalid edit receipt');
            // Use this receipt's own canonical result even when a newer event snapshot has already rendered.
            if (job.capture && job.action.type === 'movePoint') {
                const canonical = reply.snapshot.setup?.points.find(p => p.pointId === job.capture?.id);
                if (!canonical)
                    throw new Error('The dragged point disappeared');
                job.capture.expected = canonical;
            }
            this.accept(reply.snapshot);
            if (job.capture && !this.captureValid(job.capture))
                throw new Error('Another window changed that point');
            job.resolve();
        }
        catch (e) {
            if (job.capture) {
                let reason: EditReason = 'mutationFailed';
                try {
                    if (decodeError(e).code === 'invalidRoute')
                        reason = 'invalidDrop';
                }
                catch { }
                this.failCapture(job.capture, reason);
            }
            else if (!['confirmReview', 'reportEditState', 'ackStartPreparation'].includes(job.action.type))
                this.failure = { pointIds: [], reason: 'mutationFailed' };
            this.showError(e);
            job.reject(e);
        }
        finally {
            this.inFlight = false;
            void this.pump();
        }
    }
    private fire(promise: Promise<void>): void { void promise.catch(() => { }); }
    private settle(): void {
        if (this.disposed || !this.snapshot?.setup || !this.snapshot.caller || this.inFlight || this.queue.length || this.capture && !this.capture.released)
            return;
        if (this.capture?.released) {
            const capture = this.capture;
            const removed = !this.snapshot.setup.points.some(p => p.pointId === capture.id);
            if (removed || !capture.failed && (!this.failure || this.failure.pointIds.every(id => id === capture.id)))
                this.failure = null;
            this.capture = null;
        }
        const setup = this.snapshot.setup;
        const caller = this.snapshot.caller;
        const state = this.failure ? 'incomplete' : 'ready';
        const pointIds = this.failure?.pointIds ?? [];
        const reason = this.failure?.reason ?? null;
        const report = JSON.stringify({ state, pointIds, reason });
        const positiveAck = setup.preparation?.readyOverlayIds.includes(caller.overlayId);
        if (positiveAck || this.snapshot.status.startStage === 'arming' || this.snapshot.status.phase === 'stopping')
            return;
        if (report !== this.reported) {
            this.reported = report;
            this.fire(this.enqueue({ type: 'reportEditState', state, pointIds, reason }));
            return;
        }
        const preparation = setup.preparation;
        if (preparation && preparation.pendingOverlayIds.includes(caller.overlayId) && !this.acked.has(preparation.preparationId) && !this.preparingAck) {
            this.acked.add(preparation.preparationId);
            this.preparingAck = true;
            this.fire(this.enqueue({ type: 'ackStartPreparation', preparationId: preparation.preparationId, outcome: this.failure ? 'incomplete' : 'ready', drainedThrough: caller.lastAcceptedActionSeq }).finally(() => { this.preparingAck = false; }));
        }
    }
    add(u: number, v: number): Promise<void> { if (this.frozen)
        return Promise.reject(new Error('Finish starting before editing')); this.error = null; return this.enqueue({ type: 'addPoint', u, v }).then(() => { if (!this.failure?.pointIds.length)
        this.failure = null; this.settle(); }); }
    remove(pointId: Id): Promise<void> { if (this.frozen)
        return Promise.reject(new Error('Setup is preparing')); return this.enqueue({ type: 'removePoint', pointId }).then(() => { if (this.failure && this.failure.pointIds.every(id => id === pointId || !this.snapshot?.setup?.points.some(p => p.pointId === id)))
        this.failure = null; this.settle(); }); }
    pause(pauseMs: number): Promise<void> { if (this.frozen)
        return Promise.reject(new Error('Setup is preparing')); return this.enqueue({ type: 'setPause', pauseMs }); }
    confirm(): Promise<void> { const { setup } = this.current(); const frozen = { routeRevision: setup.routeRevision, layoutRevision: setup.layoutRevision }; return this.enqueue({ type: 'confirmReview' }, frozen); }
    begin(pointId: Id): boolean { if (this.frozen || this.capture)
        return false; const { setup, caller } = this.current(); const point = setup.points.find(p => p.pointId === pointId); if (!point)
        return false; this.capture = { id: pointId, mapping: JSON.stringify(caller.mapping), layout: setup.layoutRevision, expected: { ...point }, released: false, failed: false }; this.reported = ''; this.fire(this.enqueue({ type: 'reportEditState', state: 'editing', pointIds: [pointId], reason: null })); return true; }
    move(u: number, v: number): void { const capture = this.capture; if (!capture || capture.released || capture.failed)
        return; this.preview = { capture, u, v }; if (!this.previewTimer)
        this.previewTimer = setTimeout(() => { this.previewTimer = null; const p = this.preview; this.preview = null; if (p && !p.capture.released && !p.capture.failed) {
            const queued = this.queue.find(j => j.preview && j.capture === p.capture);
            if (queued)
                queued.action = { type: 'movePoint', pointId: p.capture.id, u: p.u, v: p.v };
            else {
                const promise = new Promise<void>((resolve, reject) => { this.queue.push({ action: { type: 'movePoint', pointId: p.capture.id, u: p.u, v: p.v }, capture: p.capture, preview: true, resolve, reject }); });
                this.fire(promise);
                void this.pump();
            }
        } }, 1000 / 30); }
    release(u: number, v: number): void { const capture = this.capture; if (!capture || capture.released)
        return; this.dropPreview(); this.queue = this.queue.filter(job => { if (job.preview && job.capture === capture) {
        job.resolve();
        return false;
    } return true; }); capture.released = true; if (!capture.failed)
        this.fire(this.enqueue({ type: 'movePoint', pointId: capture.id, u, v }, undefined, capture));
    else
        this.settle(); }
    captureLost(): void { if (this.capture && !this.capture.released) {
        this.capture.released = true;
        this.failCapture(this.capture, 'captureLost');
        this.settle();
    } }
    private failCapture(c: Capture, reason: EditReason): void { c.failed = true; this.dropPreview(); this.failure = { pointIds: this.snapshot?.setup?.points.some(p => p.pointId === c.id) ? [c.id] : [], reason }; }
    private dropPreview(): void { if (this.previewTimer)
        clearTimeout(this.previewTimer); this.previewTimer = null; this.preview = null; }
    async start(): Promise<void> { if (this.frozen)
        return; const { setup } = this.current(); this.startPending = true; this.changed(); const input: StartInput = { setupId: setup.setupId, routeRevision: setup.routeRevision, layoutRevision: setup.layoutRevision, requestId: this.requests.next() }; try {
        const reply = await this.transport.start(input);
        this.accept(reply.snapshot);
    }
    catch (e) {
        this.showError(e);
    }
    finally {
        this.startPending = false;
        this.changed();
    } }
    async cancel(): Promise<void> { try {
        const { setup } = this.current();
        this.accept((await this.transport.cancel(setup.setupId)).snapshot);
    }
    catch (e) {
        this.showError(e);
    } }
    async permissions(): Promise<void> { try {
        const { setup } = this.current();
        this.accept((await this.transport.permissions(setup.setupId)).snapshot);
    }
    catch (e) {
        this.showError(e);
    } }
    dispose(): void { if (this.disposed)
        return; this.disposed = true; this.dropPreview(); this.unlisten?.(); this.unlisten = null; for (const job of this.queue.splice(0))
        job.reject(new Error('Setup closed')); }
}
