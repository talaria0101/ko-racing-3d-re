// Deobfuscated from j.class (extends java.lang.Object).
//
// Camera side transform holder threaded through the race render
// path: bs.a/b(Lbq;Lj;[Lcl...), bm.a(Lbq;[Lcl;Lj;),
// bp.a(Lbq;Lj;IFFF), ai.a(Lbq;Lj;FFFFFF). Fields verified; math TODO.
//
// Field map: j.a:Transform, j.a:Ldf, j.a/b/c:F, j.a:[F, j.d/e/f:F.
// The base M3GNode.composite(CameraRig) overload ignores this
// argument, so whatever billboarding or chase cam math lives here
// applies at the bt/bd level (camera set in bq), not per scenery
// object. Do not use this to explain the tree wall; SceneryObject
// documents that.

import javax.microedition.m3g.Transform;

public class CameraRig {

    public Transform transform; // j.a
    // TODO(obfuscated): df/a/b/c/F fields, a()V, a(Ldi;F)V, a()Z,
    // a(Lj;)V, c()/a()/b()F accessors.

    public Transform transform() {
        return transform;
    }
}

/*
 * CAMERA SIDE (j, verified heads).
 *
 * j.a()Ldf gives the frustum/position holder the render entry reads
 * the camera cell from (df.a()/14 + 0.5, df.b()/14 + 0.5); j.a()V
 * resets it (called from bt.e at race setup); j.a(Ldi;F)V,
 * j.a(Lj;)V, c()/a()/b()F are per-frame followers still TODO. The
 * base node composite ignores this rig (see M3GNode); it steers
 * culling and the camera, never the scenery orientation.
 */
