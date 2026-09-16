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

/*
 * VIEW AND CLEAR (verified).
 *
 * Frustum (df, held by the camera rig j): j.a() gives df; df.a()/b()
 * are the camera position in world units (the render entry divides by
 * ar.c = 14 and adds 0.5 for the camera cell); df.a(x, y, z) answers
 * whether a world point is inside the view and cells failing it are
 * skipped before rendering. df carries seven float arrays plus one
 * float (planes plus scratch). The port does no frustum culling.
 *
 * Clear (bq.a(Background)): straight Graphics3D.clear. Race scenes
 * (bd, r, u, co, dm, aj) build colour-only Backgrounds
 * (new + setColor + setColorClearEnable) - NOBODY calls setImage, so
 * the sky strip is not an M3G background image. It reaches the
 * screen as a 2D UI-layer image (bi id 10000, added in bd.m when the
 * al.a sky path is non-empty) composited around the 3D view, which
 * is why it is identical facing any direction. The port stretches
 * that strip fullscreen, which buries the strip's sun (at 0.52
 * height) behind the track; see Sky below.
 */
