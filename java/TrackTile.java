// Deobfuscated from ar.class (extends java.lang.Object) plus a.class.
// a.class is the collision triangle soup reader.
//
// Track tile: the `.tl` parser, the static world scale, and tile
// placement. Parser and placement verified at the bytecode level;
// collision math confirmed against 26 shipped meshes and the Rust
// port (format.rs Tile/Collision, scene.rs surface_height).
//
// Key statics (original -> here):
//   ar.a : nodeScale   (7.0, or 9.0 on the low graphics path)
//   ar.b : halfTile    (1 * nodeScale = 7.0)
//   ar.c : tileEdge    (2 * halfTile = 14.0)
// The render node scale is nodeScale + 0.01 = 7.01, applied once at
// load via M3GNode.setScale (see ar.a(Z)V tail:
//   getstatic ar.a, ldc 0.01, fadd, x3, am.b(FFF)).

// Layout of a `.tl` file (class ar, reader be):
//   str name            // models/p/<name>
//   str texture         // tex/<texture>, season swapped (texpack -> ts/td/tf)
//   u8 variant          // read then discarded
//   u8 pointCount + points (u8 x, u8 y, percent of tile)
//   u8 edgeCount + edges (flat u8 vertex indices, consumed as pairs)
//   u8 heightCount + heights: value < 100 -> (value, 1000.0); else the
//     next byte is stored and the world height is 14 * (value-100)/100
//   u8 solid[4]         // edge rendering / minimap only
//   u8 split; if nonzero, u8 open[4], else open = solid
//     Despite reading like walls, open[] is the DRIVABLE side flag:
//     bm.b(i) returns ar.b((i + arg) % 4) and the bs flood branches on
//     it. Using open rotated by arg connects all 40 maps; using solid
//     leaves 28 in fragments.
//   u8 unused           // discarded
//   collision: u8 vertexCount; if > 0, vertices as u8 x,y,z (/100,
//     wrap > 2.0 by -2.56) then u8 triangleCount + triples.
//   All 62 shipped files parse with zero trailing bytes.

import javax.microedition.m3g.Transform;

public class TrackTile {

    /** Original: ar.a static float. 7.0, or 9.0 when al.m == 2 and not cl.f. */
    public static float nodeScale = 7.0f;
    /** Original: ar.b static float. 1 * nodeScale. */
    public static float halfTile = 7.0f;
    /** Original: ar.c static float. 2 * halfTile = 14.0. */
    public static float tileEdge = 14.0f;

    // Instance state (original field names in comments).
    public String modelName;    // ar.b
    public String textureName;  // ar.c
    public M3GNode node;        // ar.a (Lam)

    // Outline / edges / heights kept for the minimap and edge drawing.
    // Types: z[] points (ar.a:[Lz), int[] edges (ar.a:[I), heights.
    // TODO(obfuscated): rename z, a, ct helpers once y/ca are done.

    public TrackTile() {
        // TODO(obfuscated): ar.<init>()V body not walked.
    }

    public TrackTile(String file) {
        // TODO(obfuscated): ar.<init>(String)V reads the file name,
        // delegates to load(). Body not walked instruction by
        // instruction, but the Python and Rust parsers reproduce it
        // byte for byte.
    }

    /**
     * Original: ar.a(Lcf;)V (plus ar.a(Z)V for the model bind).
     * Verified: sets nodeScale to 7.0 (9.0 on the low path), then
     * halfTile = 1 * nodeScale, tileEdge = 2 * halfTile. Reads name,
     * texture (with the texpack -> ts/td/tf seasonal swap on themes
     * 2/3/4), points (scaled by tileEdge/100), edges, heights (with
     * the 14 * (value-100)/100 rule), solid/open flags, and the
     * collision mesh. Binds models/p/<name> with tex/<texture> through
     * de (model cache) and calls node.setScale(nodeScale + 0.01 on all
     * axes), i.e. 7.01. The vl -> vl1 rename when al.f() != 0 is in
     * here too (ar.a(Z)V head).
     */
    public void load(Object textureCache) {
        // TODO(obfuscated): full body walk is long (500+ bytecode
        // bytes); the control flow above is confirmed, each block is
        // not yet transcribed line by line.
        nodeScale = 7.0f;
        halfTile = nodeScale;
        tileEdge = 2.0f * halfTile;
        node = new M3GNode();
        node.setScale(nodeScale + 0.01f, nodeScale + 0.01f, nodeScale + 0.01f);
    }

    /**
     * Original: ar.a(Lbq;IFFF)V.
     * Fully verified (35 bytes):
     *   node.setOrientation(90 * arg, 0, 0, 1);
     *   node.setTranslation(x, y, 0);
     *   renderer.render(node);
     * So tiles sit at z = 0 with yaw only. No pitch, no roll, no
     * height offset; ramps come from the mesh itself plus the
     * collision height function, not from the node.
     */
    public void place(Renderer renderer, int arg, float x, float y, float zIgnored) {
        node.setOrientation(90.0f * (float) arg, 0.0f, 0.0f, 1.0f);
        node.setTranslation(x, y, 0.0f);
        renderer.render(node);
    }

    /**
     * Original: ar.a(FF)F and ar.a(FF)Lbj; (height sampling through
     * bm). The game negates the mesh z when building triangles
     * (three fnegs in a.<init>), so height in the Z-down world is
     * -z * 14. The Rust port is Y-up and reports +z * 14 instead;
     * using the game formula raw mirrors ramps into pits. See
     * scene::surface_height and tools/kora/formats.py Collision.
     */
    public static float sampleHeight(float u, float v) {
        // TODO(obfuscated): transcribe the barycentric walk
        // (bm.a(float,float) rotates the sample by arg first).
        return 0.0f;
    }
}
