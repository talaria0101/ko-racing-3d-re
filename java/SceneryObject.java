// Deobfuscated from ai.class (extends java.lang.Object).
//
// Scenery object: one `.ob` entry (model + texture + flag). The flag
// does two jobs, both verified at the bytecode level:
//   1. texture format in cf.a: true loads Image2D type 100 (RGBA),
//      false loads type 99 (RGB). Set on trees, bushes, cacti
//      (t1-t4, b1, c1, z2, z3), clear on everything else.
//   2. render path in ai.a(Lbq;Lj;FFFFFF)V (below).
//
// `.ob` layout:
//   str model     // models/<model>
//   str texture   // tex/<texture>; if it starts with "t.png"
//                 // (indexOf == 0) it is season swapped on themes
//                 // 2/3/4 to ts2/td2/tf2.png. The shared atlas is
//                 // NOT swapped here (unlike bc), so objects keep the
//                 // plain atlas whatever the season.
//   u8 flag       // != 0 -> RGBA path + flagged render path
// Field map: ai.a:Lat node, ai.a/b/c:String, ai.a:Z loaded,
// ai.b:Z flag.
//
// This is the file behind the giant tree wall screenshot. Read the
// render paths carefully before touching scene.rs again.

import javax.microedition.m3g.Mesh;
import javax.microedition.m3g.Transform;

public class SceneryObject {

    public String model;    // ai.a
    public String texture;  // ai.b
    public boolean alpha;   // ai.b (true -> RGBA Image2D type 100)

    public MeshLoader node; // ai.a (Lat, extends M3GNode)

    public SceneryObject(String file) {
        // TODO(obfuscated): ai.<init>(String)V just stores the name.
    }

    /** Original: ai.a(Lcf;)V. Reads model/texture/flag, applies the
     *  t.png seasonal swap above, then ai.a(Z)V binds through de and
     *  calls node.setScale(ar.a + 0.01 on all axes) = 7.01. The four
     *  de overloads (a/b x with/without boolean) select the RGBA vs
     *  RGB texture path by the flag. */
    public void load(Object textureCache) {
        // TODO(obfuscated): exact de.a/de.b overload per branch is
        // listed in /tmp/clsinfo output; transcribe which branch uses
        // which (the pattern matches bc but with the extra flag test).
    }

    /**
     * Original: ai.a(Lbq;Lj;FFFFFF)V (100 bytes, two branches).
     * Signature: (renderer, camera, x, y, z, rx, ry, rz). In practice
     * bp always passes rx = 0, ry = 0, rz = 0/90/180/270.
     *
     * Flag CLEAR (normal: church, houses, gates, fences, walls):
     *   node.setTranslation(x, y, z);            // at.a(FFF)
     *   node.setOrientation(0, 1, 0, 0);         // at.a(FFFF), identity
     *   node.postRotateSetup(0, 0, 1, 0);        // at.b(FFFF), identity
     *   node.postRotate(rz, 0, 0, 1);            // at.b(FFFF), the yaw
     *   renderer.render(node);                   // bq.a(Lam)
     * So normal objects get the cell yaw. The two identity rotations
     * are no-ops; they just reset the two rotation stages.
     *
     * Flag SET (trees t1-t4, bush b1, cacti c1/z2/z3):
     *   node.setTranslation(x, y, z);            // at.a(FFF)
     *   renderer.render(mesh, transform);        // bq.a(Node, Transform)
     *   with mesh = node.mesh() and transform = node.composite().
     * No orientation is set on this path at all. The composite keeps
     * whatever rotation it had (identity after load), so flagged
     * objects render with IDENTITY rotation regardless of the rz the
     * caller passed. bp still computes the mirrored x/y, but the yaw
     * is dropped for trees.
     *
     * PORT BUG (giant wall): scene.rs applies arg * 90 yaw to every
     * high detail object, including flagged trees. The game applies
     * yaw to normal objects only. A tree plane (t1: y = 0.15
     * constant, 0.36 wide, 0.53 tall pre-scale) rotated 90/270 degrees
     * presents its edge or piles face-on into the neighbours and reads
     * as a continuous tall wall (Timberton img 1). Fix: render flagged
     * objects with identity rotation (or, equivalently, skip the yaw
     * for .ob flag true). KEmulator's m3g_lwjgl confirms there is no
     * auto-billboarding here; both paths go through
     * Graphics3D.render(Node, Transform) with an explicit Transform.
     */
    public void place(Renderer renderer, CameraRig camera,
                      float x, float y, float z,
                      float rxIgnored, float ryIgnored, float rz) {
        if (!alpha) {
            node.setTranslation(x, y, z);
            node.setOrientation(0.0f, 1.0f, 0.0f, 0.0f);
            // TODO(obfuscated): the exact at.b(FFFF) split (reset vs
            // post) is per M3GNode; the net effect is yaw rz about Z.
            node.postRotate(0.0f, 0.0f, 1.0f, 0.0f);
            node.postRotate(rz, 0.0f, 0.0f, 1.0f);
            renderer.render(node);
        } else {
            node.setTranslation(x, y, z);
            Mesh mesh = node.mesh();
            Transform transform = node.composite();
            renderer.render(mesh, transform);
        }
    }
}
