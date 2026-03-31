// ═══════════════════════════════════════════════════════════════
// PIXEL ANIMATION - Generated TypeScript
// Canvas: 16x16 | Duration: 1000ms | Layers: 3
// ═══════════════════════════════════════════════════════════════

export interface PixelTransform {
  x: number
  y: number
  rotation: number
  scaleX: number
  scaleY: number
  opacity: number
}

interface EncodedPixelsRleV1 {
  codec: 'pixels-rle-v1'
  origin: { x: number; y: number }
  size: { w: number; h: number }
  palette: string[]
  rows: Array<Array<[xStart: number, len: number, colorIndex: number]>>
}

type PixelSnapshotPayload = Array<{ x: number; y: number; color: string }> | EncodedPixelsRleV1

export interface PixelLayerData {
  id: string
  name: string
  keyframes: Array<{
    percent: number
    transform: PixelTransform
    objectSnapshots: PixelObjectSnapshot[]
    easing: string
  }>
}

export interface PixelObjectSnapshot {
  objectId: string
  type: 'pixels' | 'rect' | 'circle' | 'path'
  pixels?: PixelSnapshotPayload
  x?: number
  y?: number
  width?: number
  height?: number
  cx?: number
  cy?: number
  r?: number
  path?: string
  color: string
  transform: PixelTransform
}

interface CompactPixelObjectPatch {
  objectId: string
  remove?: 1
  type?: 'pixels' | 'rect' | 'circle' | 'path'
  pixels?: PixelSnapshotPayload
  x?: number
  y?: number
  width?: number
  height?: number
  cx?: number
  cy?: number
  r?: number
  path?: string
  color?: string
  transform?: PixelTransform
}

interface CompactPixelLayerData {
  id: string
  name: string
  keyframes: Array<{
    percent: number
    transform: PixelTransform
    objectPatches?: CompactPixelObjectPatch[]
    order?: string[]
    easing: string
  }>
}

export interface PixelAnimationOptions {
  container?: HTMLElement
  duration?: number
  speed?: number
  repeat?: 'loop' | 'once'
  autoPlay?: boolean
  onComplete?: () => void
  onFinished?: (detail: { progress: number; iteration: number }) => void
  onFrame?: (progress: number) => void
}

export interface PixelAnimationController {
  readonly svg: SVGSVGElement
  readonly isPlaying: boolean
  readonly progress: number
  readonly duration: number
  readonly speed: number
  play(): void
  pause(): void
  stop(): void
  seek(percent: number): void
  setSpeed(multiplier: number): void
  setRepeatMode(mode: 'loop' | 'once'): void
  setDirection(direction: 'normal' | 'reverse' | 'alternate' | 'alternate-reverse'): void
  getLayer(name: string): SVGGElement | null
  getLayerNames(): string[]
  destroy(): void
}

// Animation data
const COMPACT_ANIMATION_DATA: CompactPixelLayerData[] = [{"id":"iytnzam0v","name":"Layer 1","keyframes":[{"percent":0,"transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1},"objectPatches":[{"objectId":"vptowlrbz","type":"pixels","pixels":{"codec":"pixels-rle-v1","origin":{"x":0,"y":0},"size":{"w":16,"h":16},"palette":["#000000"],"rows":[[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]],[[0,16,0]]]},"color":"#000000","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}}],"order":["vptowlrbz"],"easing":"linear"}]},{"id":"1fakijj73","name":"Layer 2","keyframes":[{"percent":0,"transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1},"objectPatches":[{"objectId":"ggk4z8mpd","type":"pixels","pixels":[{"x":1,"y":7,"color":"#FFFFFF"},{"x":2,"y":7,"color":"#FFFFFF"},{"x":1,"y":8,"color":"#FFFFFF"},{"x":2,"y":8,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"wtjo2u5a9","type":"pixels","pixels":{"codec":"pixels-rle-v1","origin":{"x":13,"y":7},"size":{"w":2,"h":2},"palette":["#FFFFFF"],"rows":[[[0,2,0]],[[0,2,0]]]},"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"bdc3edntg","type":"pixels","pixels":[{"x":3,"y":8,"color":"#FFFFFF"},{"x":4,"y":8,"color":"#FFFFFF"},{"x":3,"y":9,"color":"#FFFFFF"},{"x":4,"y":9,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":-16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"7vlefs6hb","type":"pixels","pixels":[{"x":5,"y":8,"color":"#FFFFFF"},{"x":6,"y":8,"color":"#FFFFFF"},{"x":5,"y":9,"color":"#FFFFFF"},{"x":6,"y":9,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":-16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"gl9p6skg4","type":"pixels","pixels":[{"x":7,"y":8,"color":"#FFFFFF"},{"x":8,"y":8,"color":"#FFFFFF"},{"x":7,"y":9,"color":"#FFFFFF"},{"x":8,"y":9,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":-16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"9mhfsu3rw","type":"pixels","pixels":{"codec":"pixels-rle-v1","origin":{"x":9,"y":8},"size":{"w":2,"h":2},"palette":["#FFFFFF"],"rows":[[[0,2,0]],[[0,2,0]]]},"color":"#FFFFFF","transform":{"x":0,"y":-16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"yn9gtixov","type":"pixels","pixels":{"codec":"pixels-rle-v1","origin":{"x":11,"y":8},"size":{"w":2,"h":2},"palette":["#FFFFFF"],"rows":[[[0,2,0]],[[0,2,0]]]},"color":"#FFFFFF","transform":{"x":0,"y":-16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"ntcfpp067","type":"pixels","pixels":[{"x":3,"y":6,"color":"#FFFFFF"},{"x":4,"y":6,"color":"#FFFFFF"},{"x":3,"y":7,"color":"#FFFFFF"},{"x":4,"y":7,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"fqfwvpt95","type":"pixels","pixels":[{"x":5,"y":6,"color":"#FFFFFF"},{"x":6,"y":6,"color":"#FFFFFF"},{"x":5,"y":7,"color":"#FFFFFF"},{"x":6,"y":7,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"h354opyx0","type":"pixels","pixels":[{"x":7,"y":6,"color":"#FFFFFF"},{"x":8,"y":6,"color":"#FFFFFF"},{"x":7,"y":7,"color":"#FFFFFF"},{"x":8,"y":7,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"ka9jc82y7","type":"pixels","pixels":{"codec":"pixels-rle-v1","origin":{"x":9,"y":6},"size":{"w":2,"h":2},"palette":["#FFFFFF"],"rows":[[[0,2,0]],[[0,2,0]]]},"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"p7guolraj","type":"pixels","pixels":{"codec":"pixels-rle-v1","origin":{"x":11,"y":6},"size":{"w":2,"h":2},"palette":["#FFFFFF"],"rows":[[[0,2,0]],[[0,2,0]]]},"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}}],"order":["ggk4z8mpd","wtjo2u5a9","bdc3edntg","7vlefs6hb","gl9p6skg4","9mhfsu3rw","yn9gtixov","ntcfpp067","fqfwvpt95","h354opyx0","ka9jc82y7","p7guolraj"],"easing":"linear"},{"percent":20,"transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1},"objectPatches":[{"objectId":"bdc3edntg","transform":{"x":0,"y":16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"7vlefs6hb","transform":{"x":0,"y":16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"gl9p6skg4","transform":{"x":0,"y":16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"9mhfsu3rw","transform":{"x":0,"y":16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"yn9gtixov","transform":{"x":0,"y":16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}}],"easing":"linear"},{"percent":80,"transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1},"objectPatches":[{"objectId":"7vlefs6hb","transform":{"x":0,"y":32,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"gl9p6skg4","transform":{"x":0,"y":48,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"9mhfsu3rw","transform":{"x":0,"y":32,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"ntcfpp067","transform":{"x":0,"y":-16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"fqfwvpt95","transform":{"x":0,"y":-32,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"h354opyx0","transform":{"x":0,"y":-48,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"ka9jc82y7","transform":{"x":0,"y":-32,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"p7guolraj","transform":{"x":0,"y":-16,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}}],"easing":"linear"},{"percent":90,"transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1},"easing":"linear"},{"percent":100,"transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1},"easing":"linear"}]},{"id":"n6lghbvm2","name":"Layer 3","keyframes":[{"percent":68,"transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1},"objectPatches":[{"objectId":"vqycdi7iw","type":"pixels","pixels":[{"x":7,"y":7,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"dl5s98czf","type":"pixels","pixels":[{"x":7,"y":8,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"cgdnf2xep","type":"pixels","pixels":[{"x":8,"y":7,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}},{"objectId":"i5dqptifm","type":"pixels","pixels":[{"x":8,"y":8,"color":"#FFFFFF"}],"color":"#FFFFFF","transform":{"x":0,"y":0,"rotation":0,"scaleX":1,"scaleY":1,"opacity":1}}],"order":["vqycdi7iw","dl5s98czf","cgdnf2xep","i5dqptifm"],"easing":"linear"}]}];
const DEFAULT_TRANSFORM: PixelTransform = { x: 0, y: 0, rotation: 0, scaleX: 1, scaleY: 1, opacity: 1 }

function hasOwn(obj: object, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(obj, key)
}

function cloneTransform(transform: PixelTransform): PixelTransform {
  return {
    x: transform.x,
    y: transform.y,
    rotation: transform.rotation,
    scaleX: transform.scaleX,
    scaleY: transform.scaleY,
    opacity: transform.opacity
  }
}

function clonePayload(payload: PixelSnapshotPayload | undefined): PixelSnapshotPayload | undefined {
  if (!payload) return payload
  if (Array.isArray(payload)) return payload.map(pixel => ({ x: pixel.x, y: pixel.y, color: pixel.color }))
  return {
    codec: payload.codec,
    origin: { x: payload.origin.x, y: payload.origin.y },
    size: { w: payload.size.w, h: payload.size.h },
    palette: [...payload.palette],
    rows: payload.rows.map(row => row.map(run => [run[0], run[1], run[2]]))
  }
}

function cloneSnapshot(snapshot: PixelObjectSnapshot): PixelObjectSnapshot {
  return {
    objectId: snapshot.objectId,
    type: snapshot.type,
    pixels: clonePayload(snapshot.pixels),
    x: snapshot.x,
    y: snapshot.y,
    width: snapshot.width,
    height: snapshot.height,
    cx: snapshot.cx,
    cy: snapshot.cy,
    r: snapshot.r,
    path: snapshot.path,
    color: snapshot.color,
    transform: cloneTransform(snapshot.transform)
  }
}

function applyPatch(
  prev: PixelObjectSnapshot | undefined,
  patch: CompactPixelObjectPatch
): PixelObjectSnapshot | null {
  if (patch.remove === 1) return null
  const typeChanged = typeof patch.type === 'string' && patch.type !== prev?.type
  const next: PixelObjectSnapshot = prev && !typeChanged
    ? cloneSnapshot(prev)
    : {
      objectId: patch.objectId,
      type: patch.type ?? prev?.type ?? 'pixels',
      color: prev?.color ?? '#000000',
      transform: cloneTransform(prev?.transform ?? DEFAULT_TRANSFORM)
    }

  next.objectId = patch.objectId
  if (hasOwn(patch, 'type') && patch.type) next.type = patch.type
  if (hasOwn(patch, 'pixels')) next.pixels = clonePayload(patch.pixels)
  if (hasOwn(patch, 'x')) next.x = patch.x
  if (hasOwn(patch, 'y')) next.y = patch.y
  if (hasOwn(patch, 'width')) next.width = patch.width
  if (hasOwn(patch, 'height')) next.height = patch.height
  if (hasOwn(patch, 'cx')) next.cx = patch.cx
  if (hasOwn(patch, 'cy')) next.cy = patch.cy
  if (hasOwn(patch, 'r')) next.r = patch.r
  if (hasOwn(patch, 'path')) next.path = patch.path
  if (hasOwn(patch, 'color') && patch.color) next.color = patch.color
  if (hasOwn(patch, 'transform') && patch.transform) next.transform = cloneTransform(patch.transform)

  if (!next.color) next.color = '#000000'
  if (!next.transform) next.transform = cloneTransform(DEFAULT_TRANSFORM)
  return next
}

function expandAnimationData(compactLayers: CompactPixelLayerData[]): PixelLayerData[] {
  return compactLayers.map(layer => {
    const byId = new Map<string, PixelObjectSnapshot>()
    let order: string[] = []
    const keyframes = layer.keyframes.map(keyframe => {
      if (Array.isArray(keyframe.order)) {
        order = [...keyframe.order]
      }
      const patches = Array.isArray(keyframe.objectPatches) ? keyframe.objectPatches : []
      for (const patch of patches) {
        if (patch.remove === 1) {
          byId.delete(patch.objectId)
          order = order.filter(id => id !== patch.objectId)
          continue
        }
        const next = applyPatch(byId.get(patch.objectId), patch)
        if (!next) continue
        byId.set(patch.objectId, next)
        if (!order.includes(patch.objectId)) order.push(patch.objectId)
      }

      const objectSnapshots: PixelObjectSnapshot[] = []
      const seen = new Set<string>()
      for (const id of order) {
        const snapshot = byId.get(id)
        if (!snapshot) continue
        seen.add(id)
        objectSnapshots.push(cloneSnapshot(snapshot))
      }
      for (const [id, snapshot] of byId) {
        if (seen.has(id)) continue
        objectSnapshots.push(cloneSnapshot(snapshot))
      }

      return {
        percent: keyframe.percent,
        transform: cloneTransform(keyframe.transform),
        objectSnapshots,
        easing: keyframe.easing
      }
    }).sort((a, b) => a.percent - b.percent)

    return {
      id: layer.id,
      name: layer.name,
      keyframes
    }
  })
}

const ANIMATION_DATA: PixelLayerData[] = expandAnimationData(COMPACT_ANIMATION_DATA);

// ═══════════════════════════════════════════════════════════════
// PIXEL ANIMATION CLASS
// ═══════════════════════════════════════════════════════════════

export class PixelAnimation implements PixelAnimationController {
  readonly svg: SVGSVGElement
  readonly duration: number

  private _isPlaying: boolean = false
  private _progress: number = 0
  private _speed: number = 1
  private _repeatMode: 'loop' | 'once' = 'loop'
  private _direction: 'normal' | 'reverse' | 'alternate' | 'alternate-reverse' = 'normal'
  private _iteration: number = 0
  private _raf: number | null = null
  private _lastTime: number = 0

  private readonly _layers: Map<string, { group: SVGGElement; data: PixelLayerData; firstAppearances: Map<string, number> }> = new Map()
  private readonly _options: PixelAnimationOptions
  private readonly _width: number = 256
  private readonly _height: number = 256
  private readonly _zoom: number = 16

  constructor(options: PixelAnimationOptions = {}) {
    this._options = options
    this._speed = options.speed ?? 1
    this._repeatMode = options.repeat ?? 'loop'
    this.duration = options.duration ?? 1000

    // Create SVG element
    this.svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg')
    this.svg.setAttribute('viewBox', `0 0 ${this._width} ${this._height}`)
    this.svg.setAttribute('shape-rendering', 'crispEdges')
    this.svg.style.width = '100%'
    this.svg.style.height = '100%'

    // Build layer groups
    this._buildLayers()

    // Initial render
    this._render()

    // Attach to container if provided
    if (options.container) {
      options.container.appendChild(this.svg)
    }

    // Auto-play if requested
    if (options.autoPlay) {
      this.play()
    }
  }

  get isPlaying(): boolean { return this._isPlaying }
  get progress(): number { return this._progress }
  get speed(): number { return this._speed }

  // ── Playback Control ───────────────────────────────────────

  play(): void {
    if (this._isPlaying) return
    this._isPlaying = true
    this._lastTime = performance.now()
    this._tick()
  }

  pause(): void {
    this._isPlaying = false
    if (this._raf !== null) {
      cancelAnimationFrame(this._raf)
      this._raf = null
    }
  }

  stop(): void {
    this.pause()
    this._progress = 0
    this._iteration = 0
    this._render()
  }

  seek(percent: number): void {
    this._progress = Math.max(0, Math.min(100, percent))
    this._render()
  }

  setSpeed(multiplier: number): void {
    this._speed = Math.max(0.1, multiplier)
  }

  setRepeatMode(mode: 'loop' | 'once'): void {
    this._repeatMode = mode
  }

  setDirection(direction: 'normal' | 'reverse' | 'alternate' | 'alternate-reverse'): void {
    this._direction = direction
  }

  // ── Element Access ─────────────────────────────────────────

  getLayer(name: string): SVGGElement | null {
    const layer = this._layers.get(name)
    return layer?.group ?? null
  }

  getLayerNames(): string[] {
    return Array.from(this._layers.keys())
  }

  // ── Lifecycle ──────────────────────────────────────────────

  destroy(): void {
    this.pause()
    this.svg.remove()
    this._layers.clear()
  }

  // ── Private Methods ────────────────────────────────────────

  private _tick = (): void => {
    if (!this._isPlaying) return

    const now = performance.now()
    const delta = (now - this._lastTime) * this._speed
    this._lastTime = now

    // Calculate progress increment based on direction
    const progressDelta = (delta / this.duration) * 100

    switch (this._direction) {
      case 'normal':
        this._progress += progressDelta
        break
      case 'reverse':
        this._progress -= progressDelta
        break
      case 'alternate':
        this._progress += (this._iteration % 2 === 0 ? progressDelta : -progressDelta)
        break
      case 'alternate-reverse':
        this._progress += (this._iteration % 2 === 0 ? -progressDelta : progressDelta)
        break
    }

    let finished = false

    // Handle boundaries
    if (this._progress >= 100) {
      if (this._repeatMode === 'once' && this._isMovingForward()) {
        this._progress = 100
        finished = true
      } else {
        this._progress = this._progress % 100
        this._iteration++
        this._options.onComplete?.()
      }
    } else if (this._progress < 0) {
      if (this._repeatMode === 'once' && !this._isMovingForward()) {
        this._progress = 0
        finished = true
      } else {
        this._progress = 100 + (this._progress % 100)
        this._iteration++
        this._options.onComplete?.()
      }
    }

    this._render()
    this._options.onFrame?.(this._progress)

    if (finished) {
      this.pause()
      this._emitFinished()
      return
    }

    this._raf = requestAnimationFrame(this._tick)
  }

  private _isMovingForward(): boolean {
    switch (this._direction) {
      case 'normal':
        return true
      case 'reverse':
        return false
      case 'alternate':
        return this._iteration % 2 === 0
      case 'alternate-reverse':
        return this._iteration % 2 !== 0
    }
  }

  private _emitFinished(): void {
    const detail = { progress: this._progress, iteration: this._iteration }
    this._options.onComplete?.()
    this._options.onFinished?.(detail)
    this.svg.dispatchEvent(new CustomEvent('animation-finished', { detail }))
  }

  private _render(): void {
    for (const [name, { group, data, firstAppearances }] of this._layers) {
      if (data.keyframes.length === 0) continue

      const transform = this._interpolateTransform(data.keyframes, this._progress)

      // Apply layer transform
      const tx = transform.x
      const ty = transform.y
      const rot = transform.rotation
      const sx = transform.scaleX
      const sy = transform.scaleY

      if (tx !== 0 || ty !== 0 || rot !== 0 || sx !== 1 || sy !== 1) {
        group.setAttribute('transform', `translate(${tx}, ${ty}) rotate(${rot}) scale(${sx}, ${sy})`)
      } else {
        group.removeAttribute('transform')
      }
      group.setAttribute('opacity', String(transform.opacity))

      const snapshots = this._getInterpolatedSnapshots(data.keyframes, this._progress, firstAppearances)
      this._renderSnapshots(group, snapshots)
    }
  }

  private _getInterpolatedSnapshots(
    keyframes: PixelLayerData['keyframes'],
    percent: number,
    firstAppearances: Map<string, number>
  ): PixelObjectSnapshot[] {
    if (keyframes.length === 0) return []
    if (keyframes.length === 1) {
      return keyframes[0].objectSnapshots.filter(
        snapshot => (firstAppearances.get(snapshot.objectId) ?? 0) <= percent
      )
    }
    if (percent <= keyframes[0].percent) {
      return keyframes[0].objectSnapshots.filter(
        snapshot => (firstAppearances.get(snapshot.objectId) ?? 0) <= percent
      )
    }
    if (percent >= keyframes[keyframes.length - 1].percent) {
      return keyframes[keyframes.length - 1].objectSnapshots
    }

    for (const kf of keyframes) {
      if (Math.abs(kf.percent - percent) < 0.0001) return kf.objectSnapshots
    }

    let prev = keyframes[0]
    let next = keyframes[keyframes.length - 1]
    for (let i = 0; i < keyframes.length - 1; i++) {
      if (percent >= keyframes[i].percent && percent <= keyframes[i + 1].percent) {
        prev = keyframes[i]
        next = keyframes[i + 1]
        break
      }
    }

    if (prev.easing === 'hold') return prev.objectSnapshots

    const range = next.percent - prev.percent
    const t = range > 0 ? (percent - prev.percent) / range : 0
    const prevById = new Map(prev.objectSnapshots.map(snapshot => [snapshot.objectId, snapshot]))
    const nextById = new Map(next.objectSnapshots.map(snapshot => [snapshot.objectId, snapshot]))
    const ids = new Set<string>([...prevById.keys(), ...nextById.keys()])
    const result: PixelObjectSnapshot[] = []

    for (const id of ids) {
      const before = prevById.get(id)
      const after = nextById.get(id)
      if (before && after) {
        result.push(this._interpolateSnapshot(before, after, t))
        continue
      }
      if (before && !after) {
        result.push(before)
      }
      // Spawn semantics: if object appears at next keyframe only, keep it hidden before that.
    }

    return result.filter(snapshot => (firstAppearances.get(snapshot.objectId) ?? 0) <= percent)
  }

  private _getFirstAppearances(keyframes: PixelLayerData['keyframes']): Map<string, number> {
    const firstAppearances = new Map<string, number>()
    const seen = new Set<string>()
    for (const keyframe of keyframes) {
      for (const snapshot of keyframe.objectSnapshots) {
        if (seen.has(snapshot.objectId)) continue
        seen.add(snapshot.objectId)
        firstAppearances.set(snapshot.objectId, keyframe.percent)
      }
    }
    return firstAppearances
  }

  private _interpolateSnapshot(
    a: PixelObjectSnapshot,
    b: PixelObjectSnapshot,
    t: number
  ): PixelObjectSnapshot {
    const snapshot: PixelObjectSnapshot = {
      ...a,
      transform: this._interpolateObjectTransform(a.transform, b.transform, t)
    }

    if (a.type === b.type) {
      if (a.type === 'rect') {
        snapshot.x = this._lerp(a.x ?? 0, b.x ?? 0, t)
        snapshot.y = this._lerp(a.y ?? 0, b.y ?? 0, t)
        snapshot.width = this._lerp(a.width ?? 0, b.width ?? 0, t)
        snapshot.height = this._lerp(a.height ?? 0, b.height ?? 0, t)
      } else if (a.type === 'circle') {
        snapshot.cx = this._lerp(a.cx ?? 0, b.cx ?? 0, t)
        snapshot.cy = this._lerp(a.cy ?? 0, b.cy ?? 0, t)
        snapshot.r = this._lerp(a.r ?? 0, b.r ?? 0, t)
      } else if (a.type === 'path') {
        snapshot.path = t < 0.5 ? a.path : b.path
      }
    }

    return snapshot
  }

  private _interpolateObjectTransform(a: PixelTransform, b: PixelTransform, t: number): PixelTransform {
    return {
      x: this._lerp(a.x, b.x, t),
      y: this._lerp(a.y, b.y, t),
      rotation: this._lerp(a.rotation, b.rotation, t),
      scaleX: this._lerp(a.scaleX, b.scaleX, t),
      scaleY: this._lerp(a.scaleY, b.scaleY, t),
      opacity: this._lerp(a.opacity, b.opacity, t)
    }
  }

  private _interpolateTransform(
    keyframes: PixelLayerData['keyframes'],
    percent: number
  ): PixelTransform {
    if (keyframes.length === 0) {
      return { x: 0, y: 0, rotation: 0, scaleX: 1, scaleY: 1, opacity: 1 }
    }

    if (keyframes.length === 1) return { ...keyframes[0].transform }

    let prev = keyframes[0], next = keyframes[keyframes.length - 1]
    for (let i = 0; i < keyframes.length - 1; i++) {
      if (percent >= keyframes[i].percent && percent <= keyframes[i + 1].percent) {
        prev = keyframes[i]
        next = keyframes[i + 1]
        break
      }
    }

    const range = next.percent - prev.percent
    const t = range > 0 ? (percent - prev.percent) / range : 0

    return {
      x: this._lerp(prev.transform.x, next.transform.x, t),
      y: this._lerp(prev.transform.y, next.transform.y, t),
      rotation: this._lerp(prev.transform.rotation, next.transform.rotation, t),
      scaleX: this._lerp(prev.transform.scaleX, next.transform.scaleX, t),
      scaleY: this._lerp(prev.transform.scaleY, next.transform.scaleY, t),
      opacity: this._lerp(prev.transform.opacity, next.transform.opacity, t)
    }
  }

  private _lerp(a: number, b: number, t: number): number {
    return a + (b - a) * t
  }

  private _buildLayers(): void {
    for (const layerData of ANIMATION_DATA) {
      const normalizedKeyframes = [...layerData.keyframes].sort((a, b) => a.percent - b.percent)
      const normalizedLayerData: PixelLayerData = {
        id: layerData.id,
        name: layerData.name,
        keyframes: normalizedKeyframes
      }
      const firstAppearances = this._getFirstAppearances(normalizedKeyframes)
      const group = document.createElementNS('http://www.w3.org/2000/svg', 'g')
      group.setAttribute('id', this._sanitizeId(layerData.name))
      group.setAttribute('data-layer', layerData.name)

      // Render initial keyframe
      if (normalizedKeyframes.length > 0) {
        this._renderSnapshots(group, this._getInterpolatedSnapshots(normalizedKeyframes, 0, firstAppearances))
      }

      this.svg.appendChild(group)
      this._layers.set(layerData.name, { group, data: normalizedLayerData, firstAppearances })
    }
  }

  private _renderSnapshots(group: SVGGElement, snapshots: PixelObjectSnapshot[]): void {
    // Clear existing content
    group.innerHTML = ''

    for (const snapshot of snapshots) {
      const el = this._renderSnapshot(snapshot)
      if (el) group.appendChild(el)
    }
  }

  private _renderSnapshot(snapshot: PixelObjectSnapshot): SVGElement | null {
    let el: SVGElement

    if (snapshot.type === 'pixels') {
      const decodedPixels = this._ensureDecodedPixels(snapshot)
      if (decodedPixels.length === 0) return null

      // Group pixels by color for optimization
      const byColor = new Map<string, Array<{ x: number; y: number }>>()
      for (const p of decodedPixels) {
        if (!byColor.has(p.color)) byColor.set(p.color, [])
        byColor.get(p.color)!.push({ x: p.x, y: p.y })
      }

      const g = document.createElementNS('http://www.w3.org/2000/svg', 'g')

      for (const [color, pixels] of byColor) {
        for (const p of pixels) {
          const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect')
          rect.setAttribute('x', String(p.x * this._zoom))
          rect.setAttribute('y', String(p.y * this._zoom))
          rect.setAttribute('width', String(this._zoom))
          rect.setAttribute('height', String(this._zoom))
          rect.setAttribute('fill', color)
          g.appendChild(rect)
        }
      }

      // Apply object transform
      if (snapshot.transform) {
        const center = this._getSnapshotCenter(snapshot)
        this._applyTransform(g, snapshot.transform, center.x, center.y)
      }

      return g
    }

    if (snapshot.type === 'rect') {
      el = document.createElementNS('http://www.w3.org/2000/svg', 'rect')
      el.setAttribute('x', String((snapshot.x ?? 0) * this._zoom))
      el.setAttribute('y', String((snapshot.y ?? 0) * this._zoom))
      el.setAttribute('width', String((snapshot.width ?? 0) * this._zoom))
      el.setAttribute('height', String((snapshot.height ?? 0) * this._zoom))
      el.setAttribute('fill', snapshot.color)

      if (snapshot.transform) {
        const center = this._getSnapshotCenter(snapshot)
        this._applyTransform(el, snapshot.transform, center.x, center.y)
      }
      return el
    }

    if (snapshot.type === 'circle') {
      el = document.createElementNS('http://www.w3.org/2000/svg', 'circle')
      el.setAttribute('cx', String((snapshot.cx ?? 0) * this._zoom))
      el.setAttribute('cy', String((snapshot.cy ?? 0) * this._zoom))
      el.setAttribute('r', String((snapshot.r ?? 0) * this._zoom))
      el.setAttribute('fill', snapshot.color)

      if (snapshot.transform) {
        const center = this._getSnapshotCenter(snapshot)
        this._applyTransform(el, snapshot.transform, center.x, center.y)
      }
      return el
    }

    if (snapshot.type === 'path' && snapshot.path) {
      el = document.createElementNS('http://www.w3.org/2000/svg', 'path')
      el.setAttribute('d', snapshot.path)
      el.setAttribute('fill', snapshot.color)

      if (snapshot.transform) {
        this._applyTransform(el, snapshot.transform, 0, 0)
      }
      return el
    }

    return null
  }

  private _getSnapshotCenter(snapshot: PixelObjectSnapshot): { x: number; y: number } {
    if (snapshot.type === 'pixels') {
      const pixels = this._ensureDecodedPixels(snapshot)
      if (pixels.length === 0) return { x: 0, y: 0 }
      let minX = Infinity
      let minY = Infinity
      let maxX = -Infinity
      let maxY = -Infinity
      for (const pixel of pixels) {
        minX = Math.min(minX, pixel.x)
        minY = Math.min(minY, pixel.y)
        maxX = Math.max(maxX, pixel.x)
        maxY = Math.max(maxY, pixel.y)
      }
      return {
        x: ((minX + maxX + 1) / 2) * this._zoom,
        y: ((minY + maxY + 1) / 2) * this._zoom
      }
    }
    if (snapshot.type === 'rect') {
      return {
        x: ((snapshot.x ?? 0) + (snapshot.width ?? 0) / 2) * this._zoom,
        y: ((snapshot.y ?? 0) + (snapshot.height ?? 0) / 2) * this._zoom
      }
    }
    if (snapshot.type === 'circle') {
      return {
        x: (snapshot.cx ?? 0) * this._zoom,
        y: (snapshot.cy ?? 0) * this._zoom
      }
    }
    return { x: 0, y: 0 }
  }

  private _ensureDecodedPixels(snapshot: PixelObjectSnapshot): Array<{ x: number; y: number; color: string }> {
    const payload = snapshot.pixels
    if (!payload) return []
    if (Array.isArray(payload)) return payload
    if (!this._isPixelsRle(payload)) return []
    const decoded = this._decodePixelsRle(payload)
    snapshot.pixels = decoded
    return decoded
  }

  private _isPixelsRle(payload: PixelSnapshotPayload): payload is EncodedPixelsRleV1 {
    return (
      !!payload &&
      typeof payload === 'object' &&
      !Array.isArray(payload) &&
      (payload as { codec?: string }).codec === 'pixels-rle-v1' &&
      Array.isArray((payload as { palette?: unknown }).palette) &&
      Array.isArray((payload as { rows?: unknown }).rows)
    )
  }

  private _decodePixelsRle(payload: EncodedPixelsRleV1): Array<{ x: number; y: number; color: string }> {
    const originX = Number.isFinite(payload.origin?.x) ? Math.trunc(payload.origin.x) : 0
    const originY = Number.isFinite(payload.origin?.y) ? Math.trunc(payload.origin.y) : 0
    const palette = Array.isArray(payload.palette) ? payload.palette : []
    const rows = Array.isArray(payload.rows) ? payload.rows : []
    const dedup = new Map<string, { x: number; y: number; color: string }>()
    for (let y = 0; y < rows.length; y++) {
      const rowRuns = Array.isArray(rows[y]) ? rows[y] : []
      for (const run of rowRuns) {
        if (!Array.isArray(run) || run.length < 3) continue
        const xStart = Math.trunc(Number(run[0]))
        const len = Math.trunc(Number(run[1]))
        const colorIndex = Math.trunc(Number(run[2]))
        if (!Number.isFinite(xStart) || !Number.isFinite(len) || len <= 0) continue
        const color = palette[colorIndex]
        if (typeof color !== 'string') continue
        for (let dx = 0; dx < len; dx++) {
          const x = originX + xStart + dx
          const yAbs = originY + y
          dedup.set(`${x},${yAbs}`, { x, y: yAbs, color })
        }
      }
    }
    return Array.from(dedup.values()).sort((a, b) => a.y - b.y || a.x - b.x || a.color.localeCompare(b.color))
  }

  private _applyTransform(el: SVGElement, transform: PixelTransform, centerX: number, centerY: number): void {
    const parts: string[] = []
    const hasTranslate = transform.x !== 0 || transform.y !== 0
    const hasRotate = transform.rotation !== 0
    const hasScale = transform.scaleX !== 1 || transform.scaleY !== 1
    if (hasRotate || hasScale) {
      parts.push(`translate(${transform.x + centerX}, ${transform.y + centerY})`)
      if (hasRotate) parts.push(`rotate(${transform.rotation})`)
      if (hasScale) parts.push(`scale(${transform.scaleX}, ${transform.scaleY})`)
      parts.push(`translate(${-centerX}, ${-centerY})`)
    } else if (hasTranslate) {
      parts.push(`translate(${transform.x}, ${transform.y})`)
    }
    if (parts.length > 0) {
      el.setAttribute('transform', parts.join(' '))
    }
    if (transform.opacity !== 1) {
      el.setAttribute('opacity', String(transform.opacity))
    }
  }

  private _sanitizeId(name: string): string {
    return name.replace(/[^a-zA-Z0-9_-]/g, '_')
  }
}

// ═══════════════════════════════════════════════════════════════
// USAGE EXAMPLE
// ═══════════════════════════════════════════════════════════════

/*
import { PixelAnimation } from './pixel-animation'

// Basic usage
const animation = new PixelAnimation({
  container: document.getElementById('my-container'),
  duration: 2000, // Optional: override default duration (ms)
  repeat: 'once', // 'loop' | 'once'
  onFinished: () => console.log('Animation finished'),
  autoPlay: true
})

// Control playback
animation.play()
animation.pause()
animation.seek(50) // Jump to 50%
animation.setSpeed(2) // 2x speed
animation.setRepeatMode('loop') // Switch repeat behavior at runtime
animation.setDirection('alternate')

// Listen for finish event
animation.svg.addEventListener('animation-finished', (event) => {
  console.log('finished event', (event as CustomEvent).detail)
})

// Attach events to layers
const eyes = animation.getLayer('eyes')
if (eyes) {
  eyes.addEventListener('mouseenter', () => {
    animation.pause()
  })
  eyes.addEventListener('mouseleave', () => {
    animation.play()
  })
}

// Get all layer names
console.log('Layers:', animation.getLayerNames())

// Clean up
animation.destroy()
*/
