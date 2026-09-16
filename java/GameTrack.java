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
