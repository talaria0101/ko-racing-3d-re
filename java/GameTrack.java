// Deobfuscated from bs.class (extends java.lang.Object).
//
// The track model: `.map` parser plus the grid the race drives on.
// Parser verified byte for byte (all 40 maps end with zero trailing
// bytes); render and height helpers partly verified.
//
// `.map` layout (reader be):
//   u8 width, u8 height
//   repeat height (y) x width (x):
//     u8 flags: bit 0 = tile, bits 1-3 = mid detail, bits 4-6 = high
//     for each set bit in order 0..6: u8 first, u8 second payload
//       bit 0: (tileKind, tileArg) -> bm constructor
//       bits 1-3: (midKind, midArg) -> bm.a/b/c(ae, ..)
//       bits 4-6: (highKind, highArg) -> bm.a/b/c(ap, ..)
//   u8 startX, startY, finishX, finishY
//   u8 checkpointCount + pairs
// Each set flag bit contributes one payload dispatched by bit index.
// Slicing positionally (payloads 1..4 for mid) drops detail on short
// cells; the branch fix-missing-trackside-detail already walks the
// bits instead (format.rs Map::parse, tools/kora/formats.py Map).
//
// Field map: bs.a:I / bs.b:I width/height? bs.a:[[Lbm grid,
// bs.a:F / bs.b:F scratch, bs.c..j:I start/finish/checkpoint state,
// bs.a:[B scratch. Method inventory below is complete.

public class GameTrack {

    public int width;    // TODO: bs.a or bs.b
    public int height;   // TODO
    public TrackCell[][] grid; // bs.a:[[Lbm

    public int startX, startY, finishX, finishY;
    public int[] checkpoints; // flat pairs

    public GameTrack(Object assetManager, java.io.InputStream in) {
        // TODO(obfuscated): bs.<init>(Lcf;InputStream[,Z])V. The
        // Python/Rust parsers reproduce the byte walk; the M3G node
        // wiring (bf/ae/ap caches, bm construction per cell) is TODO.
    }

    // ---- verified parser helpers ----
    // bs.a(InputStream, boolean): the variable length grid walk above.

    // ---- render / query (inventories, bodies TODO) ----
    // bs.a(Lbq;Lj;[Lcl;III)V : render range with cars (minimap? race?)
    // bs.b(Lbq;Lj;[Lcl;III)V : second render range
    // bs.a(Lbq;Lj;[Lcl;)V    : full render with cars
    // bs.a(FF)F              : height sampling at world x/y
    // bs.a(FF)Lbj;           : surface point
    // bs.a(IILz;)V           : TODO
    // bs.a()LImage;          : minimap image
    // bs.a(III)I / bs.b(III)I: tile/side queries
    // bs.a(II)Lbz;           : TODO
    // bs.a(Ldi;)V / bs.a(Ldi;I)F / bs.a(FFZ)F: TODO (di/x helpers)
    // bs.a/b(Lcf;)V         : load / unload assets
    // bs.d/e/f()I           : TODO counters

    /** Height used by cars and barrier placement. Falls back to flat
     *  zero when the tile ships no collision mesh (most plain road).
     *  TODO: transcribe bs.a(FF)F. */
    public float heightAt(float x, float y) {
        return 0.0f;
    }
}

/*
 * RACE ASSEMBLY (how a track is put together for a race).
 *
 * Verified against bs.class bytecode with /tmp/dis.py. Field map used
 * below: bs.a:I width, bs.b:I height, bs.a:[[Lbm grid, bs.f/g:I start,
 * bs.i/j:I finish, bs.h:I flood length at finish, bs.b:F step float,
 * bs.e:I per-pass frame marker, bs.a:[B checkpoint bytes.
 *
 * Linking flood (bs.a()V): starts at the start cell with order counter
 * 0 and walks the road trying sides 0..3 in order (+X, -Y, -X, +Y via
 * b.a/b.b), stepping only onto mutually open neighbours
 * (bm.a(dir) here, opposite side there). Each visit stamps the cell
 * with bm.c(order) on even steps and bm.d(order) on odd steps (bm.c
 * even prints the old index to stdout when overwritten - leftover
 * debug), marks sides linked with bm.a(side, true), stops the count
 * at the finish (bs.h) but keeps walking until it returns to the
 * start. bm.c()/bm.d() are therefore path positions along the flood,
 * which ck (the AI waypoint field) indexes per cell.
 *
 * Render entry (bs.a(Lbq;Lj;[Lcl;)V): takes the camera cell from the
 * camera rig (j.a().a()/14 + 0.5, j.a().b()/14 + 0.5) and the view
 * distance (al.g), bumps the frame marker (bs.e = (bs.e + 1) % 10000)
 * and runs TWO flood passes, bs.a(...,count) then bs.b(...,count),
 * counting down. Each pass (bs.a(Lbq;Lj;[Lcl;III)V with x, y, count):
 * reject out-of-range cells; skip cells already stamped with the
 * current marker (bm.a()I == bs.e); skip cells outside the view
 * (j.a().a(x*14, y*14, bs.a:F) false - the df frustum check, see
 * Renderer.java); stamp the cell (bm.a(bs.e)); render it (bm.a(Lbq)
 * when the detail flags allow); recurse into the four neighbours
 * whose bm.b(dir) (SECOND flag block) is set, with count - 1; render
 * once more at depth 0. The port renders every occupied cell with no
 * frustum or depth cutoff instead - a performance-only divergence,
 * the same triangles in the same places.
 *
 * LOD gates: mid detail renders only above one graphics-detail
 * threshold (al.f) and high detail above another; the port's
 * Detail::Base/Mid/Full select the same layers. Exact threshold
 * constants are branch offsets in the bytecode, still TODO.
 *
 * Minimap (bs.a()LImage): int[width*3 * height*3], cleared to
 * -11645362, then per road cell per set bm.b(dir) side one pixel at
 * (x*3 + dx + 1) + (y*3 + dy + 1) * width*3 in -5592406 - row 0 at
 * the top, so game +Y runs down the image, matching mapimg.py and
 * the port's HUD minimap. Cells whose bm.b()I == 3 (finish tile kind)
 * are highlighted. No mirroring anywhere in the chain.
 */
