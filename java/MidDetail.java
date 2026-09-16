// Deobfuscated from bc.class (extends java.lang.Object).
//
// Mid detail: one `.md` tile item (model + texture plus one ignored
// trailing byte). Parser and placement verified at the bytecode
// level. Seasonal texture swap verified (matches the Rust port
// detail_texture and the screenshots: without it autumn trees use
// the summer sheet).
//
// `.md` layout (reader be, loader bl):
//   str model     // models/<model>
//   str texture   // tex/<texture>, season swapped (see below)
//   u8 trailing   // present in every file, ignored (likely a
//                 // rotation/height leftover from the exporter)
// Field map: bc.a:Lat node, bc.a/b:String model/texture, bc.c:String,
// bc.a:Z loaded.
//
// Seasonal swap (bc.a(Lcf;)V, verified branches on al.j theme):
//   theme 2/3/4 and texture contains "texpack" -> ts.png / td.png / tf.png
//   theme 2/3/4 and texture starts with "t.png" (indexOf == 0)
//     -> ts2.png / td2.png / tf2.png
//   else keep the stored name. Themes 0-1 keep everything.

public class MidDetail {

    public String model;    // bc.a
    public String texture;  // bc.b
    public MeshLoader node; // bc.a (Lat, extends M3GNode)

    public MidDetail(String file) {
        // TODO(obfuscated): bc.<init>(String)V just stores the name.
    }

    /** Original: bc.a(Lcf;)V. Reads the two strings, swaps the texture
     *  by theme (above), closes the stream, then bc.a(Z)V binds the
     *  model through de with scale nodeScale + 0.01. */
    public void load(Object textureCache) {
        // TODO(obfuscated): branch walk done for the swap, bind tail
        // (de.a(model, texture, true) + at.b(7.01 x3)) assumed from ai;
        // confirm the exact de.a/de.b overload for bc.
    }

    /**
     * Original: bc.a(Lbq;IFFF)V.
     * Fully verified (35 bytes, identical shape to ar.a):
     *   node.setOrientation(90 * arg, 0, 0, 1);
     *   node.setTranslation(x, y, 0);
     *   renderer.render(node);
     * Mid detail sits at z = 0 with yaw only, exactly like tiles.
     * There is no per-model height offset; models whose own geometry
     * floats (sh: z -1.05..-0.19, i.e. 1.33..7.36 above ground after
     * the 7.01 scale) float in the game too. Suspect behind some of
     * the floating rock/rail reports, but not yet proven; marked TODO
     * in the README rather than asserted.
     */
    public void place(Renderer renderer, int arg, float x, float y, float zIgnored) {
        node.setOrientation(90.0f * (float) arg, 0.0f, 0.0f, 1.0f);
        node.setTranslation(x, y, 0.0f);
        renderer.render(node);
    }
}
