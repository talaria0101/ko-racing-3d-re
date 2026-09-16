// Deobfuscated from bq.class (extends java.lang.Object).
//
// Thin JSR-184 renderer wrapper. Both methods verified (10 and 13
// bytes). No culling or filtering state lives here; that is in the
// Appearance (see MeshLoader.java).
//
// Field map: bq.a:Graphics3D, bq.a:Camera,
// bq.a/b/c/d:Transform scratch.

import javax.microedition.m3g.Graphics3D;
import javax.microedition.m3g.Node;
import javax.microedition.m3g.Transform;

public class Renderer {

    public Graphics3D g3d; // bq.a

    /** Original: bq.a(Lam;)V. Renders node.mesh() with
     *  node.composite(). Verified. */
    public void render(M3GNode node) {
        render(node.mesh(), node.composite());
    }

    /** Original: bq.a(LNode;LTransform;)V. Directly
     *  Graphics3D.render(node, transform). Verified. */
    public void render(Node node, Transform transform) {
        g3d.render(node, transform);
    }

    // Remaining bq methods (camera bind, background clear, target
    // bind/release, setCamera, transforms): inventories only, TODO.
    //   bq.<init>()V, bq.a(II)V, bq.a(IIF)V,
    //   bq.a(Graphics)V, bq.a(Background)V, bq.a(Transform)V,
    //   bq.a()V, bq.a()LGraphics3D, bq.b()V
}
