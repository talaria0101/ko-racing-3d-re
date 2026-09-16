// Deobfuscated from bm.class (extends java.lang.Object).
//
// One placed track cell: the tile plus its mid and high detail
// references for one `.map` grid square. Structure verified; a few
// helper bodies remain TODO.
//
// Field map (original -> here, partial):
//   bm.a:Lar tileDef        bm.b:I / bm.a:I indices
//   bm.a:F / bm.b:F scratch
//   bm.a:[B / bm.a:[Z / bm.b:[Z flag scratch
//   bm.a:Lbc mid0, bm.b:Lbc mid1, bm.c:Lbc mid2 + indices f,g / h,i / j,k
//   bm.a:Lbp high0, bm.b:Lbp high1, bm.c:Lbp high2 + indices l,m / n,o / p,q
//   bm.d/e:I cell position? bm.c:I ?
// The three mid slots correspond to .map flag bits 1-3, the three
// high slots to bits 4-6. Each set bit contributes one payload in bit
// order (see GameTrack.java); bm stores up to three of each.
//
// Constructor: bm.<init>(Lbf;IIII)V takes the tile cache plus four
// ints (tile kind, tile arg, cell x, cell y in some order).
// TODO(obfuscated): pin the exact parameter order by walking the
// 302 byte body.

public class TrackCell {

    public TrackTile tile;          // bm.a
    public int tileKind;            // TODO: bm.b or bm.a
    public int tileArg;             // TODO

    public MidDetail[] mid = new MidDetail[3];
    public HighDetail[] high = new HighDetail[3];

    public TrackCell(Object tileCache, int a, int b, int c, int d) {
        // TODO(obfuscated): walk bm.<init>(Lbf;IIII)V fully.
    }

    /**
     * Originals: bm.a/b/c(Lae;II)V (mid) and bm.a/b/c(Lap;II)V (high).
     * Verified for bm.a(Lae;II)V: on desert theme (r.j == 3) kinds
     * below 19, 22, 28, 29, 30, and 32..36 become 0 (skipped); then
     * ae.a(kind) loads the MidDetail and the arg is stored. The b/c
     * variants do the same for slots 1 and 2. High variants
     * (Lap;II) just load ap.a(kind) with no theme remap except the
     * object level one in bp.
     */
    public void setMid(int slot, Object midCache, int kind, int arg) {
        // TODO(obfuscated): transcribe slot field stores.
    }

    public void setHigh(int slot, Object highCache, int kind, int arg) {
        // TODO(obfuscated): transcribe slot field stores.
    }

    /** Original: bm.a(Lbq;)V. Renders tile, then mids, then highs.
     *  TODO: confirm the exact order and the detail level skips
     *  (al.d graphics detail: base skips mid, mid skips high). */
    public void render(Renderer renderer) {
        // TODO(obfuscated).
    }

    /**
     * Original: bm.a(Lbq;[Lcl;Lj;)V. Cell render during a race (takes
     * the car list and the camera rig). TODO.
     */
    public void renderRace(Renderer renderer, RaceCar[] cars, CameraRig camera) {
        // TODO(obfuscated).
    }

    /** Original: bm.a(FF)F (height), bm.a(FF)Lbj; (surface point).
     *  Rotates the sample by the cell arg (bm.a(float,float) in the
     *  Python tools) then queries the tile collision mesh, else 0.
     *  TODO: transcribe. */
    public float heightAt(float u, float v) {
        return 0.0f;
    }

    // Side queries used by the road flood and barriers:
    // bm.a(I)Z, bm.b(I)Z, bm.c(I)Z, bm.a(IZ)V, bm.b(I)V, bm.c(I)V,
    // bm.c()I, bm.d(I)V, bm.d()I, bm.e(I)V. TODO.
}

/*
 * RACE RENDER (bm.a(Lbq;[Lcl;Lj;)V, verified).
 *
 * Renders the tile (ar.a with the TILE arg), then the three high
 * slots in order, each reloaded by stored kind (ap.a(bm.?)) and
 * placed with its STORED DETAIL arg and the cell origin
 * (bp.a(renderer, camera, detailArg, cellX, cellY)) - never the tile
 * arg. Mid detail is not rendered here (it renders in bm.a(Lbq)V,
 * gated by the graphics detail); the port renders all layers in one
 * place instead, same final triangles. The tail of the method walks
 * car hooks (cl.a/cl.c with bm.b[Z] side gates) - bodies TODO.
 *
 * Marker fields, all verified: bm.c:I is a render stamp
 * (bm.a(I)V sets it, bm.a()I reads it; the render flood stamps
 * bs.e there and skips stamped cells), while bm.d:I/bm.e:I are the
 * flood path indices (bm.c(I)V/bm.d(I)V set them during linking;
 * bm.c(I)V prints the old index when called twice). bm.b:I is zero
 * from the constructor and bm.a(Lbq)V returns early when it is
 * nonzero - set nowhere in the race path so far (TODO: menu/preview
 * use). bm.b(I)V resets side state [0] (TODO: per-frame reset path).
 */
