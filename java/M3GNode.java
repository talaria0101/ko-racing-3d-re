// Deobfuscated from am.class (extends java.lang.Object).
//
// M3G node wrapper. Every tile, detail, object, and car mesh is held
// together with three local transforms plus a composite. Verified at
// the bytecode level with /tmp/dis.py; M3G semantics cross checked
// against m3g_lwjgl in https://github.com/shinovon/KEmulator
// (VertexBuffer decodes signed bytes as raw * scale + bias, and
// Transformable.getCompositeTransform builds T * R * S * custom).
//
// Mapping (original -> here):
//   am.a : translation Transform      am.b : rotation Transform
//   am.c : scale Transform            am.d : composite Transform
//   am.a : Mesh                       am.a : Texture2D (appearance side)
//   am.a(FFF)V   -> setTranslation    am.a(FFFF)V -> setOrientation (delegates to b)
//   am.b(FFFF)V  -> postRotate        am.b(FFF)V  -> setScale
//   am.a()       -> composite()       am.a(Lj;) -> composite() (ignores the arg)
//   am.a(Lcf,...)-> makeAppearance    am.<init>()V -> clear()

import javax.microedition.m3g.Mesh;
import javax.microedition.m3g.Texture2D;
import javax.microedition.m3g.Transform;

public class M3GNode {

    // Originals: am.a (translation), am.b (rotation), am.c (scale),
    // am.d (composite). All start null and are assigned real
    // Transforms by clear().
    protected Transform translation;
    protected Transform rotation;
    protected Transform scale;
    protected Transform composite;

    protected Mesh mesh;
    protected Texture2D texture;

    public M3GNode() {
        clear();
    }

    /** Original: am.a()V. Assigns fresh identity Transforms and clears mesh. */
    protected void clear() {
        // TODO(obfuscated): the constructor bytecode uses aconst_null
        // stores (opcode 0x01 paths in /tmp/dis.py are not yet decoded),
        // but every use site calls setIdentity before post*, so the
        // effective start state is identity. Confirm by walking
        // at.<init>(String, Appearance, boolean).
        translation = new Transform();
        rotation = new Transform();
        scale = new Transform();
        mesh = null;
        composite = new Transform();
    }

    /**
     * Original: am.a(FFF)V.
     * Verified: getfield translation, setIdentity, then
     * postTranslate(x, y, z). The odd fstore shuffling in the bytecode
     * just reorders the three floats; the call is
     * translation.postTranslate(x, y, z).
     */
    public void setTranslation(float x, float y, float z) {
        translation.setIdentity();
        translation.postTranslate(x, y, z);
    }

    /**
     * Original: am.a(FFFF)V.
     * Verified: getfield rotation, setIdentity, then delegates to
     * setOrientation(angle, ax, ay, az), dropping the third component
     * slot (passes fload_1, fload_2, fconst_0, fload_4). Callers pass
     * (90 * arg, 0, 0, 1), i.e. yaw about the game's up axis.
     */
    public void setOrientation(float angle, float ax, float ay, float az) {
        rotation.setIdentity();
        postRotate(angle, ax, 0.0f, az);
    }

    /**
     * Original: am.b(FFFF)V.
     * Verified: rotation.postRotate(angle, ax, ay, az). No reset.
     */
    public void postRotate(float angle, float ax, float ay, float az) {
        rotation.postRotate(angle, ax, ay, az);
    }

    /**
     * Original: am.b(FFF)V.
     * Verified: getfield scale, setIdentity, then
     * scale.postScale(x, y, z). Tile/object/detail loaders call this
     * once with (ar.a + 0.01) on all axes, i.e. 7.01.
     */
    public void setScale(float x, float y, float z) {
        scale.setIdentity();
        scale.postScale(x, y, z);
    }

    /**
     * Original: am.a()Ljavax/microedition/m3g/Transform;.
     * Verified: composite.setIdentity, then postMultiply(translation),
     * postMultiply(rotation), postMultiply(scale). Since postMultiply
     * is this = this * other, the result is T * R * S. A vertex v goes
     * through scale first, then rotation, then translation, which is
     * the standard M3G node order (cf. Transformable in m3g_lwjgl).
     */
    public Transform composite() {
        composite.setIdentity();
        composite.postMultiply(translation);
        composite.postMultiply(rotation);
        composite.postMultiply(scale);
        return composite;
    }

    /**
     * Original: am.a(Lj;)Ljavax/microedition/m3g/Transform;.
     * Verified: ignores the argument, returns composite(). The camera
     * rig (j) is threaded through ai/bp/bt/bs signatures but the base
     * node never reads it. Subclasses (at) do not override it.
     */
    public Transform composite(CameraRig ignored) {
        return composite();
    }

    /** Original: am.a()Ljavax/microedition/m3g/Mesh;. */
    public Mesh mesh() {
        return mesh;
    }

    // Appearance helpers (am.a(Lcf,...) overloads) belong to the
    // material path; see MeshLoader.java. They build an Appearance
    // with PolygonMode culling 160Families, nearest filtering, and the
    // texture from cf. Bodies TODO.
}
