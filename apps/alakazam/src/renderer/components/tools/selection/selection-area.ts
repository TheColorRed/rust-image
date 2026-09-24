// @ts-ignore
import * as ClipperLib from 'clipper-lib';

import * as polygonClipping from 'polygon-clipping';

export abstract class SelectionArea {
  /** The starting point of the selection area. */
  protected startPoint: [number, number] = [0, 0];
  /** The ending point of the selection area. */
  protected endPoint: [number, number] = [0, 0];
  /** The current point of the selection area. */
  protected currentPoint: [number, number] = [0, 0];
  /** The internal points that define the selection area. */
  protected areaPoints: [number, number][] = [];
  /** The temporary points that define the selection area during interaction. */
  protected tmpAreaPoints: [number, number][] = [];
  /** The merge type for the selection area. */
  protected mergeType: 'none' | 'add' | 'subtract' = 'none';
  /** Gets the points that define the selection area. */
  get points() {
    return this.areaPoints;
  }
  /** Gets the temporary points that define the selection area. */
  get tmpPoints() {
    return this.tmpAreaPoints;
  }
  /** Gets the merge type for the selection area. */
  get merge() {
    return this.mergeType;
  }
  /** Handles mouse down event for the selection area.  */
  mouseDown(x: number, y: number): void {
    if (this.merge === 'none') this.resetWith('main', [[x, y]]);
    else if (this.merge === 'add' || this.merge === 'subtract') this.resetWith('tmp', [[x, y]]);
    else this.clear('main');
  }
  /** Handles mouse move event for the selection area. */
  mouseMove(): void {}
  /** Handles mouse up event for the selection area. */
  mouseUp(): void {}
  /**
   * Sets the starting point of the selection area.
   * @param x The x coordinate of the starting point.
   * @param y The y coordinate of the starting point.
   */
  setStartPoint(x: number, y: number) {
    this.startPoint = [x, y];
  }
  /**
   * Sets the ending point of the selection area.
   * @param x The x coordinate of the ending point.
   * @param y The y coordinate of the ending point.
   */
  setEndPoint(x: number, y: number) {
    this.endPoint = [x, y];
  }
  /**
   * Sets the current point of the selection area.
   * @param x The x coordinate of the current point.
   * @param y The y coordinate of the current point.
   */
  setCurrentPoint(x: number, y: number) {
    this.currentPoint = [x, y];
  }
  /**
   * Sets the points that define the selection area. If merge is 'none', sets areaPoints; otherwise sets tmpAreaPoints.
   * @param points The points to set.
   */
  setPoints(points: [number, number][]) {
    if (this.merge === 'none') this.areaPoints = points;
    else this.tmpAreaPoints = points;
  }
  /**
   * Adds a point to the selection area. If merge is 'none', adds to areaPoints; otherwise adds to tmpAreaPoints.
   * @param x The x coordinate of the point to add.
   * @param y The y coordinate of the point to add.
   */
  addPoint(x: number, y: number) {
    if (this.merge === 'none') this.areaPoints.push([x, y]);
    else this.tmpAreaPoints.push([x, y]);
  }
  /**
   * Sets the merge type for the selection area.
   * @param mergeType The merge type ('none', 'add', or 'subtract').
   */
  setMergeType(mergeType: 'none' | 'add' | 'subtract') {
    this.mergeType = mergeType;
  }
  /** Resets the selection area with the given points. */
  resetWith(what: 'main' | 'tmp', values: [number, number][]) {
    if (what === 'main') {
      this.areaPoints = [];
      for (const point of values) this.areaPoints.push(point);
    } else {
      this.tmpAreaPoints = [];
      for (const point of values) this.tmpAreaPoints.push(point);
    }
  }
  /** Clears the selection area points. */
  clear(what: 'main' | 'tmp') {
    if (what === 'main') {
      this.areaPoints = [];
      window.alakazam.projects
        .getActiveProjectMetadata()
        .then(metadata => window.tools.selection.setArea(metadata?.id ?? '', []));
    } else this.tmpAreaPoints = [];
  }
  /** Merges the temporary area points into the main area points based on the merge type. */
  mergePoints(): [number, number][] {
    if (this.tmpAreaPoints.length > 2) {
      if (this.merge === 'add') {
        // Use polygon union to merge shapes
        this.areaPoints = this.polygonUnion(this.areaPoints, this.tmpAreaPoints);
      } else if (this.merge === 'subtract') {
        // Use polygon difference to subtract the tmp area from the main one
        this.areaPoints = this.polygonDifference(this.areaPoints, this.tmpAreaPoints);
      }
    }
    return this.areaPoints;
  }
  /**
   * Pick the largest outer ring (by absolute area) from a polygon-clipping result and return it as a simple ring.
   * @param result The polygon-clipping result.
   * @returns The largest outer ring as an array of points.
   */
  private pickLargestOuterRing = (result: any): [number, number][] => {
    if (!result || result.length === 0) return [];
    // result is an array of polygons; each polygon is an array of rings. Outer ring is index 0.
    let best: [number, number][] = [];
    let bestArea = -Infinity;
    for (const polygon of result) {
      if (!Array.isArray(polygon) || polygon.length === 0) continue;
      const outer = polygon[0];
      if (!outer || outer.length < 3) continue;
      const area = Math.abs(SelectionArea.signedArea(outer));
      if (area > bestArea) {
        bestArea = area;
        best = outer.map((p: number[]) => [p[0], p[1]]);
      }
    }
    return best;
  };
  /**
   * Computes the union of two polygons.
   * @param poly1 The first polygon represented as an array of points.
   * @param poly2 The second polygon represented as an array of points.
   * @returns The resulting polygon after the union operation.
   */
  private polygonUnion(poly1: [number, number][], poly2: [number, number][]): [number, number][] {
    if (!poly1 || poly1.length < 3) return poly2 ? poly2.slice() : [];
    if (!poly2 || poly2.length < 3) return poly1 ? poly1.slice() : [];

    try {
      // polygon-clipping expects polygons as arrays of rings: polygon = [ ring1, ring2?, ... ]
      const p1 = [poly1.map(p => [p[0], p[1]])];
      const p2 = [poly2.map(p => [p[0], p[1]])];
      const result = (polygonClipping as any).union(p1, p2);
      return this.pickLargestOuterRing(result);
    } catch (err) {
      // On failure, return original poly1 as a safe fallback
      console.warn('polygonUnion failed', err);
      return poly1.slice();
    }
  }
  /**
   * Computes the difference between two polygons.
   * @param poly1 The first polygon represented as an array of points.
   * @param poly2 The second polygon represented as an array of points.
   * @returns The resulting polygon after subtracting poly2 from poly1.
   */
  private polygonDifference(poly1: [number, number][], poly2: [number, number][]): [number, number][] {
    if (!poly1 || poly1.length < 3) return [];
    if (!poly2 || poly2.length < 3) return poly1.slice();

    try {
      const p1 = [poly1.map(p => [p[0], p[1]])];
      const p2 = [poly2.map(p => [p[0], p[1]])];
      const result = (polygonClipping as any).difference(p1, p2);
      return this.pickLargestOuterRing(result);
    } catch (err) {
      console.warn('polygonDifference failed', err);
      return poly1.slice();
    }
  }
  /**
   * Compute signed area of a ring (positive if CCW).
   * @param ring The array of points defining the ring.
   * @returns The signed area of the ring.
   */
  static signedArea(ring: [number, number][]) {
    let a = 0;
    for (let i = 0; i < ring.length; i++) {
      const [x1, y1] = ring[i];
      const [x2, y2] = ring[(i + 1) % ring.length];
      a += x1 * y2 - x2 * y1;
    }
    return a / 2;
  }
  /**
   * Computes an inset polygon using ClipperLib.
   * @param poly The polygon points.
   * @param distance The inset distance.
   * @returns The inset polygon points.
   */
  static computeInsetPolygon(poly: [number, number][], distance: number): [number, number][] {
    const CLIPPER_SCALE = 1000;
    if (!poly || poly.length < 3 || distance <= 0) return poly.slice();

    try {
      // Convert to Clipper integer path
      const path = poly.map(p => ({ X: Math.round(p[0] * CLIPPER_SCALE), Y: Math.round(p[1] * CLIPPER_SCALE) }));

      const co = new (ClipperLib as any).ClipperOffset(/* miterLimit */ 2, /* arcTolerance */ 0.25);
      // Use round joins to avoid spikes; end type closed polygon
      co.AddPath(path, (ClipperLib as any).JoinType.jtRound, (ClipperLib as any).EndType.etClosedPolygon);

      const solution = new (ClipperLib as any).Paths();
      // Clipper expects positive expansion, so negative distance yields an inset
      co.Execute(solution, -Math.round(distance * CLIPPER_SCALE));

      if (!solution || solution.length === 0) {
        // fallback to centroid-scale if Clipper produced nothing
        throw new Error('clipper produced empty result');
      }

      // Pick the largest resulting ring
      let best = solution[0];
      let bestArea = Math.abs(this.signedArea(best.map((pt: any) => [pt.X / CLIPPER_SCALE, pt.Y / CLIPPER_SCALE])));
      for (const sol of solution) {
        const area = Math.abs(this.signedArea(sol.map((pt: any) => [pt.X / CLIPPER_SCALE, pt.Y / CLIPPER_SCALE])));
        if (area > bestArea) {
          bestArea = area;
          best = sol;
        }
      }

      const res = best.map((pt: any) => [pt.X / CLIPPER_SCALE, pt.Y / CLIPPER_SCALE] as [number, number]);

      // Final safety: if inset touches the boundary or is degenerate, fallback to simple centroid scaling
      const outArea = Math.abs(this.signedArea(res));
      const origArea = Math.abs(this.signedArea(poly));
      if (!(outArea > 1e-6 && outArea < origArea)) throw new Error('clipper produced degenerate inset');

      return res;
    } catch (err) {
      // console.warn('Clipper offset failed, falling back', err);
      // Fallback: simple centroid scale (previous behavior)
      const n = poly.length;
      const centroid = poly.reduce((acc, p) => [acc[0] + p[0], acc[1] + p[1]], [0, 0] as [number, number]);
      centroid[0] /= n;
      centroid[1] /= n;
      return poly.map(p => {
        const dx = p[0] - centroid[0];
        const dy = p[1] - centroid[1];
        const len = Math.hypot(dx, dy);
        if (len === 0) return p;
        const scale = Math.max(0, len - distance) / len;
        return [centroid[0] + dx * scale, centroid[1] + dy * scale] as [number, number];
      });
    }
  }
}
