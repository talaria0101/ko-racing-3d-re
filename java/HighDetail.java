// Deobfuscated from bp.class (extends java.lang.Object).
//
// High detail: one `.hd` tile item. A `.hd` holds several placements
// of scenery objects (trees, houses, gates, church parts). Parser and
// per-side mirroring verified at the bytecode level
// (bp.a(Lb;)V for the file, bp.a(Lbq;Lj;IFFF)V for placement).
//
// `.hd` layout (reader be, loader bl):
//   u8 count
//   repeat count:
//     u8 kind
//     u8 x, y, z    // position: ar.b * (value/100 - 1), i.e. 7 * (..)
//     u8 s0, s1, s2 // first triple: value/100
//     u8 t0, t1, t2 // second triple: value/100
//   u8 trailing     // present in every file, ignored
// Field map: bp.a:I count, bp.a:[I kinds, bp.a:[F positions (3 per
// entry, already multiplied by ar.b), bp.b:[F second triple,
// bp.c:[F third triple, bp.a:Lb reader, bp.a:String name.
//
// What the two triples mean: in every shipped file both are
// (1.0, 1.0, 1.0), and placement (below) never reads them. They are
// parsed and stored but unused for rendering. Do not mistake them
// for a render scale; the render scale is the shared node scale
// 7.01 from M3GNode. Kept here so a future exporter change does not
// silently break.
//
// Theme remap on desert (r.j == 3): kinds below 19, 22, 28, 29, 30,
// and 32..36 become 0 (skipped); object kinds 0/1 -> 5 and 2/3 -> 4
// (themed_detail / themed_object in scene.rs). Verified in
// bm.a(Lae;II)V and bp.a(Lb;)V branches.

public class HighDetail {

    public int count;          // bp.a
    public int[] kinds;        // bp.a:[I
    public float[] positions;  // bp.a:[F, 3 per entry, world units
    public float[] second;     // bp.b:[F, parsed, unused for render
    public float[] third;      // bp.c:[F, parsed, unused for render

    public HighDetail(String file) {
        // TODO(obfuscated): bp.<init>(String)V just stores the name.
    }

    /**
     * Original: bp.a(Lb;)V.
     * Verified head: count = be.a(stream); kinds = new int[count];
     * positions = new float[3*count]; second = new float[3*count];
     * third = new float[3*count]. Per entry: kind = be.a; positions
     * are ar.b * (be.a/100 - 1) per component (fdiv 100, fsub 1, fmul
     * ar.b); the two triples are be.a/100 each. Desert remap above
     * applies to kinds. Trailing byte ignored.
     */
    public void load(Object reader) {
        // TODO(obfuscated): transcribe the tail (stream close) line by
        // line. The numeric rules above are confirmed.
    }

    /**
     * Original: bp.a(Lbq;Lj;IFFF)V.
     * Verified for all four arg values (294 bytes, four branches):
     *   arg 0: ai(x - py, y - px, 0 + pz, 0, 0, 0)
     *   arg 1: ai(x - px, y - py, 0 + pz, 0, 0, 90)
     *   arg 2: ai(x + py, y + px, 0 + pz, 0, 0, 180)
     *   arg 3: ai(x + px, y + py, 0 + pz, 0, 0, 270)
     * where (px, py, pz) is the stored position triple for the entry
     * and (x, y) is the cell origin passed in. In words: half tile
     * offsets mirrored per side, yaw = arg * 90. The z passed to ai
     * is 0 + stored z (stored z is 0 in most entries, 0.1*7 = 0.7 or
     * 0.2*7 = 1.4 in a few, e.g. t50.hd places zc 1.4 up).
     *
     * OPEN QUESTION (floating rails/rocks): t50.hd kind 31 (zc fence)
     * at z +1.4 and several t*.hd entries with z +0.7 are taken as is
     * here because the game does exactly this. Whether they are meant
     * as overhead gantries or sit on walls the port does not build is
     * not settled; see README. Do not clamp z to 0 without evidence.
     */
    public void place(Renderer renderer, CameraRig camera, int arg,
                      float cellX, float cellY, float cellZIgnored,
                      SceneryList objects) {
        for (int i = 0; i < count; i++) {
            float px = positions[3 * i];
            float py = positions[3 * i + 1];
            float pz = positions[3 * i + 2];
            SceneryObject obj = objects.byIndex(kinds[i]);
            if (obj == null) {
                continue;
            }
            switch (arg) {
                case 0:
                    obj.place(renderer, camera,
                            cellX - py, cellY - px, pz, 0, 0, 0);
                    break;
                case 1:
                    obj.place(renderer, camera,
                            cellX - px, cellY - py, pz, 0, 0, 90);
                    break;
                case 2:
                    obj.place(renderer, camera,
                            cellX + py, cellY + px, pz, 0, 0, 180);
                    break;
                default:
                    obj.place(renderer, camera,
                            cellX + px, cellY + py, pz, 0, 0, 270);
                    break;
            }
        }
    }
}
