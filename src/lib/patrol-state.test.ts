import test from 'node:test';
import assert from 'node:assert/strict';
import { CounterAllocator, decimal, decodeReply, decodeSnapshot, PatrolClient, reviewConfirmed } from './patrol-state.ts';
import type { Reply, Snapshot, Transport, UpdateInput, PatrolError } from './patrol-state.ts';
const clone = <T>(v: T): T => structuredClone(v);
function fixture(): Snapshot {
    return {
        stateVersion: '1', status: { phase: 'editing', startStage: null, runId: null, stopReason: null, lastFailure: null }, awake: { active: true, timerRemainingMs: '18446744073709551615' }, availability: { state: 'ready', message: null },
        setup: { setupId: '1', routeRevision: '1', layoutRevision: '1', points: [{ pointId: '1', displayId: '1', x: 10, y: 20 }, { pointId: '2', displayId: '2', x: 30, y: 40 }], pauseMs: 10000, displays: [1, 2].map(id => ({ displayId: String(id), name: `Screen ${id}`, nativeBounds: { x: (id - 1) * 1000, y: 0, width: 1000, height: 800 }, unit: 'macosPoint', scale: { x: 2, y: 2 }, rotationDegrees: 0, mirrorOf: null })), invalidPoints: [], routeValidation: { valid: true, errors: [] }, review: { required: false, confirmedFor: null }, overlays: [1, 2].map(id => ({ overlayId: String(id), displayId: String(id), state: 'ready', pointIds: [], reason: null })), preparation: null },
        caller: { overlayId: '1', displayId: '1', isControlPanel: true, mapping: { origin: { x: 0, y: 0 }, width: 1000, height: 800 }, lastAcceptedActionSeq: '0' }
    };
}
function deferred<T>() { let resolve!: (v: T) => void; let reject!: (e: unknown) => void; const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; }
const flush = async () => { for (let i = 0; i < 12; i++)
    await new Promise<void>(resolve => setImmediate(resolve)); };
function harness(initial = fixture()) {
    let state = clone(initial);
    const calls: UpdateInput[] = [];
    let changed = () => { };
    let updates: ((input: UpdateInput) => Promise<Reply>) | null = null;
    let listening = false;
    let unlistened = false;
    const respond = (input: UpdateInput): Reply => {
        state = clone(state);
        state.stateVersion = String(BigInt(state.stateVersion) + 1n);
        state.caller!.lastAcceptedActionSeq = input.actionSeq;
        const a = input.action;
        if (a.type === 'movePoint') {
            const p = state.setup!.points.find(p => p.pointId === a.pointId)!;
            p.x = a.u * 1000;
            p.y = a.v * 800;
            state.setup!.routeRevision = String(BigInt(state.setup!.routeRevision) + 1n);
        }
        if (a.type === 'reportEditState') {
            const row = state.setup!.overlays.find(r => r.overlayId === state.caller!.overlayId)!;
            row.state = a.state;
            row.reason = a.reason;
            row.pointIds = a.pointIds;
        }
        if (a.type === 'ackStartPreparation' && a.outcome === 'ready') {
            const p = state.setup!.preparation!;
            p.pendingOverlayIds = p.pendingOverlayIds.filter(id => id !== state.caller!.overlayId);
            p.readyOverlayIds.push(state.caller!.overlayId);
        }
        return { snapshot: clone(state), receipt: { type: 'update', setupId: input.setupId, overlayId: state.caller!.overlayId, actionSeq: input.actionSeq } };
    };
    const transport: Transport = { listen: async (listener) => { listening = true; changed = listener; return () => { unlistened = true; }; }, get: async () => { assert.ok(listening); return { snapshot: clone(state), receipt: null }; }, update: async (input) => { calls.push(clone(input)); return updates ? updates(input) : respond(input); }, start: async () => ({ snapshot: clone(state), receipt: null }), cancel: async () => ({ snapshot: clone(state), receipt: null }), permissions: async () => ({ snapshot: clone(state), receipt: null }) };
    const client = new PatrolClient(transport);
    return { client, calls, respond, setUpdates(fn: (i: UpdateInput) => Promise<Reply>) { updates = fn; }, state: () => state, push(fn: (s: Snapshot) => void) { state = clone(state); fn(state); state.stateVersion = String(BigInt(state.stateVersion) + 1n); changed(); }, unlistened: () => unlistened };
}
test('wire keeps all 64 bits, canonical strings, explicit nulls and closed records', () => {
    const s = fixture();
    assert.equal(decodeSnapshot(s).awake.timerRemainingMs, '18446744073709551615');
    for (const invalid of ['01', '-1', '1e2', '18446744073709551616', 1, 0, ''])
        assert.throws(() => decimal(invalid));
    const counter = new CounterAllocator('18446744073709551614');
    assert.equal(counter.next(), '18446744073709551615');
    assert.throws(() => counter.next());
    const missing = clone(s) as unknown as Record<string, unknown>;
    delete missing.caller;
    assert.throws(() => decodeSnapshot(missing));
    assert.throws(() => decodeSnapshot({ ...s, extra: true }));
    assert.throws(() => decodeReply({ snapshot: s }));
    const bad = clone(s);
    bad.caller!.mapping.width = Infinity;
    assert.throws(() => decodeSnapshot(bad));
    bad.caller!.mapping.width = 1000;
    bad.setup!.overlays.push(clone(bad.setup!.overlays[0]));
    assert.throws(() => decodeSnapshot(bad));
});
test('review consent belongs only to the exact displayed revision pair', () => { const s = fixture().setup!; s.review = { required: true, confirmedFor: { routeRevision: '1', layoutRevision: '1' } }; assert.ok(reviewConfirmed(s)); s.routeRevision = '2'; assert.equal(reviewConfirmed(s), false); assert.throws(() => decodeSnapshot({ ...fixture(), setup: s })); });
test('subscribe precedes initial fetch; disposal unlistens and cannot stop a committed patrol', async () => { const h = harness(); await h.client.connect(); await flush(); h.client.dispose(); assert.ok(h.unlistened()); const n = h.calls.length; h.push(s => { s.setup!.pauseMs = 2000; }); await flush(); assert.equal(h.calls.length, n); });
test('older snapshots never roll back rendered state', async () => { const h = harness(); await h.client.connect(); await flush(); const old = clone(h.client.snapshot!); h.push(s => { s.setup!.pauseMs = 2000; }); await flush(); h.client.accept(old); assert.equal(h.client.snapshot!.setup!.pauseMs, 2000); h.client.dispose(); });
test('confirmation freezes the rendered pair before waiting for another mutation and never retries consent', async () => {
    const h = harness();
    await h.client.connect();
    await flush();
    const gate = deferred<Reply>();
    h.setUpdates(async (input) => input.action.type === 'setPause' ? gate.promise : Promise.reject({ code: 'staleSnapshot', message: 'Review again', snapshot: clone(h.state()) } satisfies PatrolError));
    const pending = h.client.pause(2000);
    const consent = h.client.confirm().catch(e => e);
    h.push(s => { s.setup!.routeRevision = '9'; s.setup!.pauseMs = 3000; });
    await flush();
    gate.resolve(h.respond(h.calls.at(-1)!));
    await pending;
    await consent;
    await flush();
    const confirmations = h.calls.filter(i => i.action.type === 'confirmReview');
    assert.equal(confirmations.length, 1);
    assert.equal(confirmations[0].routeRevision, '1');
    assert.ok(h.client.error?.includes('Review again'));
    h.client.dispose();
});
test('lost confirmation reply replays the identical consent identity without moving it to newer revisions', async () => {
    const h = harness();
    await h.client.connect();
    await flush();
    let once = false;
    h.setUpdates(async (input) => { if (input.action.type === 'confirmReview' && !once) {
        once = true;
        throw new Error('Lost reply');
    } return h.respond(input); });
    await h.client.confirm();
    const confirmations = h.calls.filter(i => i.action.type === 'confirmReview');
    assert.equal(confirmations.length, 2);
    assert.deepEqual(confirmations[0], confirmations[1]);
    h.client.dispose();
});
test('pointer release drops an unsent preview and retains its own immutable final coordinate', async () => {
    const h = harness();
    await h.client.connect();
    await flush();
    assert.ok(h.client.begin('1'));
    h.client.move(.2, .3);
    h.client.release(.4, .5);
    h.client.move(.9, .9);
    await flush();
    const moves = h.calls.filter(i => i.action.type === 'movePoint');
    assert.equal(moves.length, 1);
    assert.deepEqual(moves[0].action, { type: 'movePoint', pointId: '1', u: .4, v: .5 });
    assert.equal(h.client.snapshot!.setup!.points[0].x, 400);
    h.client.dispose();
});
test('slow preview replies cannot cause a burst of queued old samples before final release', async () => {
    const h = harness();
    await h.client.connect();
    await flush();
    const gate = deferred<Reply>();
    let blocked: UpdateInput | null = null;
    h.setUpdates(async (input) => { if (input.action.type === 'movePoint' && !blocked) {
        blocked = input;
        return gate.promise;
    } return h.respond(input); });
    h.client.begin('1');
    h.client.move(.2, .2);
    await new Promise(r => setTimeout(r, 45));
    h.client.move(.3, .3);
    await new Promise(r => setTimeout(r, 45));
    h.client.move(.4, .4);
    await new Promise(r => setTimeout(r, 45));
    h.client.release(.6, .6);
    assert.ok(blocked);
    gate.resolve(h.respond(blocked));
    await flush();
    const moves = h.calls.filter(i => i.action.type === 'movePoint');
    assert.equal(moves.length, 2);
    assert.deepEqual(moves[1].action, { type: 'movePoint', pointId: '1', u: .6, v: .6 });
    h.client.dispose();
});
test('a concurrent target edit is never overwritten by a stale move retry', async () => {
    const h = harness();
    await h.client.connect();
    await flush();
    h.client.begin('1');
    await flush();
    h.setUpdates(async (input) => { if (input.action.type === 'movePoint') {
        h.push(s => { s.setup!.points[0].x = 777; s.setup!.routeRevision = '8'; });
        throw { code: 'staleSnapshot', message: 'Changed elsewhere', snapshot: clone(h.state()) };
    } return h.respond(input); });
    h.client.release(.4, .5);
    await flush();
    assert.equal(h.calls.filter(i => i.action.type === 'movePoint').length, 1);
    assert.equal(h.state().setup!.points[0].x, 777);
    assert.equal(h.state().setup!.overlays[0].state, 'incomplete');
    h.client.dispose();
});
test('a lost move result fails closed instead of rebasing on a possibly peer-edited canonical target', async () => {
    const h = harness();
    await h.client.connect();
    await flush();
    h.client.begin('1');
    await flush();
    h.setUpdates(async (input) => { if (input.action.type === 'movePoint') {
        h.respond(input);
        throw new Error('Lost result');
    } return h.respond(input); });
    h.client.release(.4, .5);
    await flush();
    assert.equal(h.calls.filter(i => i.action.type === 'movePoint').length, 1);
    assert.equal(h.state().setup!.overlays[0].state, 'incomplete');
    h.client.dispose();
});
test('Start preparation waits for a captured release, readiness, then one drained acknowledgment', async () => {
    const h = harness();
    await h.client.connect();
    await flush();
    h.client.begin('1');
    await flush();
    h.push(s => { s.status = { ...s.status, phase: 'starting', startStage: 'preparingEdits', runId: '1' }; s.setup!.preparation = { preparationId: '1', requesterOverlayId: '2', requestId: '1', routeRevision: '1', layoutRevision: '1', pendingOverlayIds: ['1', '2'], readyOverlayIds: [] }; });
    await flush();
    assert.equal(h.calls.some(i => i.action.type === 'ackStartPreparation'), false);
    assert.equal(h.client.begin('2'), false);
    h.client.release(.4, .5);
    await flush();
    const ack = h.calls.findIndex(i => i.action.type === 'ackStartPreparation');
    const final = h.calls.findIndex(i => i.action.type === 'movePoint');
    assert.ok(ack > final);
    const previous = h.calls[ack - 1];
    const action = h.calls[ack].action;
    assert.equal(previous.action.type, 'reportEditState');
    if (action.type === 'ackStartPreparation')
        assert.equal(action.drainedThrough, previous.actionSeq);
    await h.client.refresh();
    await flush();
    assert.equal(h.calls.filter(i => i.action.type === 'ackStartPreparation').length, 1);
    h.client.dispose();
});
test('layout or setup changes invalidate captures and old queued work', async () => { const h = harness(); await h.client.connect(); await flush(); h.client.begin('1'); await flush(); h.push(s => { s.setup!.layoutRevision = '2'; }); await flush(); h.client.release(.5, .5); await flush(); assert.equal(h.calls.filter(i => i.action.type === 'movePoint').length, 0); h.push(s => { s.setup!.setupId = '2'; }); await flush(); assert.equal(h.client.disposed, true); assert.ok(h.unlistened()); });
test('an older own receipt cannot legitimize a newer peer change while a final move waits', async () => {
    const h = harness();
    await h.client.connect();
    await flush();
    const gate = deferred<Reply>();
    let own: Reply | null = null;
    h.setUpdates(async (input) => { if (input.action.type === 'movePoint' && !own) {
        own = h.respond(input);
        return gate.promise;
    } return h.respond(input); });
    h.client.begin('1');
    h.client.move(.2, .2);
    await new Promise(r => setTimeout(r, 45));
    assert.ok(own);
    h.push(s => { s.setup!.points[0].x = 777; s.setup!.routeRevision = '9'; });
    await flush();
    h.client.release(.6, .6);
    gate.resolve(own);
    await flush();
    assert.equal(h.calls.filter(i => i.action.type === 'movePoint').length, 1);
    assert.equal(h.state().setup!.points[0].x, 777);
    assert.equal(h.state().setup!.overlays[0].state, 'incomplete');
    h.client.dispose();
});
test('unrelated successful drags cannot clear a failed point; authoritative removal lets its source report ready', async () => {
    const h = harness();
    await h.client.connect();
    await flush();
    let rejected = false;
    h.setUpdates(async (input) => { if (input.action.type === 'movePoint' && input.action.pointId === '1' && !rejected) {
        rejected = true;
        throw { code: 'invalidRoute', message: 'Invalid drop', snapshot: clone(h.state()) };
    } return h.respond(input); });
    h.client.begin('1');
    h.client.release(-1, -1);
    await flush();
    assert.equal(h.state().setup!.overlays[0].state, 'incomplete');
    h.client.begin('2');
    h.client.release(.5, .5);
    await flush();
    assert.equal(h.state().setup!.overlays[0].state, 'incomplete');
    assert.deepEqual(h.state().setup!.overlays[0].pointIds, ['1']);
    h.push(s => { s.setup!.points = s.setup!.points.filter(p => p.pointId !== '1'); s.setup!.overlays[0].pointIds = []; s.setup!.routeRevision = '20'; });
    await flush();
    assert.equal(h.state().setup!.overlays[0].state, 'ready');
    h.client.dispose();
});
